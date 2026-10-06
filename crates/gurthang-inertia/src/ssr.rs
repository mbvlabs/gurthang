use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use serde::Deserialize;
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::Mutex,
    time::timeout,
};

use crate::page::Page;

const PROTOCOL_PREFIX: &str = "gurthang:ssr:";

#[derive(Clone, Debug)]
pub struct SsrOptions {
    pub runtime: String,
    pub timeout_ms: u64,
    pub development_bundle: PathBuf,
    pub embedded_bundle: &'static [u8],
    pub is_development: bool,
}

#[derive(Clone, Default)]
pub struct InertiaSsr {
    inner: Option<Arc<Inner>>,
}

struct Inner {
    runtime: String,
    bundle: Bundle,
    timeout: Duration,
    worker: Mutex<Option<Worker>>,
}

enum Bundle {
    Development(PathBuf),
    Embedded(&'static [u8]),
}

#[derive(Debug, Deserialize)]
pub struct SsrOutput {
    pub head: Vec<String>,
    pub body: String,
}

#[derive(Deserialize)]
struct ReadyMessage {
    ready: bool,
}

#[derive(Deserialize)]
struct RenderMessage {
    ok: bool,
    result: Option<SsrOutput>,
    error: Option<String>,
}

struct Worker {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    extracted_bundle: Option<PathBuf>,
}

impl InertiaSsr {
    pub fn from_options(options: SsrOptions) -> Self {
        let bundle = if options.is_development {
            Bundle::Development(options.development_bundle)
        } else {
            Bundle::Embedded(options.embedded_bundle)
        };
        Self {
            inner: Some(Arc::new(Inner {
                runtime: options.runtime,
                bundle,
                timeout: Duration::from_millis(options.timeout_ms),
                worker: Mutex::new(None),
            })),
        }
    }

    pub async fn render(&self, page: &Page) -> Option<SsrOutput> {
        let inner = self.inner.as_ref()?;
        let mut worker = inner.worker.lock().await;
        if worker.is_none() {
            match Worker::start(&inner.runtime, &inner.bundle, inner.timeout).await {
                Ok(started) => *worker = Some(started),
                Err(error) => {
                    tracing::warn!(error, "Inertia SSR worker failed to start; using CSR");
                    return None;
                }
            }
        }

        let rendered = {
            let worker = worker.as_mut().expect("SSR worker was initialized");
            timeout(inner.timeout, worker.render(page)).await
        };
        match rendered {
            Ok(Ok(output)) => Some(output),
            Ok(Err(error)) => {
                tracing::warn!(error, "Inertia SSR render failed; using CSR");
                stop_worker(&mut worker).await;
                None
            }
            Err(_) => {
                tracing::warn!(
                    timeout_ms = inner.timeout.as_millis(),
                    "Inertia SSR render timed out; using CSR"
                );
                stop_worker(&mut worker).await;
                None
            }
        }
    }
}

impl Worker {
    async fn start(
        runtime: &str,
        bundle: &Bundle,
        startup_timeout: Duration,
    ) -> std::result::Result<Self, String> {
        let version = runtime_version(runtime).await?;
        if is_node(runtime) && !supported_node_version(&version) {
            return Err(format!(
                "Inertia SSR requires Node.js 22 or newer; found {version}"
            ));
        }
        tracing::info!(runtime, version, "Inertia SSR runtime is available");
        let (bundle_path, extracted_bundle) = prepare_bundle(bundle).await?;
        let mut child = Command::new(runtime)
            .arg(&bundle_path)
            .env("NODE_ENV", "production")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| format!("could not execute {runtime}: {error}"))?;
        let input = child
            .stdin
            .take()
            .ok_or_else(|| "SSR worker stdin was not available".to_owned())?;
        let output = child
            .stdout
            .take()
            .ok_or_else(|| "SSR worker stdout was not available".to_owned())?;
        let mut worker = Self {
            child,
            input,
            output: BufReader::new(output),
            extracted_bundle,
        };
        let ready: ReadyMessage = timeout(startup_timeout, worker.read_message())
            .await
            .map_err(|_| "SSR worker startup timed out".to_owned())??;
        if !ready.ready {
            return Err("SSR worker returned an invalid ready message".into());
        }
        tracing::info!(process_id = worker.child.id(), "Inertia SSR worker started");
        Ok(worker)
    }

    async fn render(&mut self, page: &Page) -> std::result::Result<SsrOutput, String> {
        let mut request = serde_json::to_vec(page).map_err(|error| error.to_string())?;
        request.push(b'\n');
        self.input
            .write_all(&request)
            .await
            .map_err(|error| format!("could not write to SSR worker: {error}"))?;
        self.input
            .flush()
            .await
            .map_err(|error| format!("could not flush SSR request: {error}"))?;

        let response: RenderMessage = self.read_message().await?;
        if response.ok {
            response
                .result
                .ok_or_else(|| "SSR worker returned no render result".into())
        } else {
            Err(response
                .error
                .unwrap_or_else(|| "SSR worker returned an unknown error".into()))
        }
    }

    async fn read_message<T: for<'de> Deserialize<'de>>(
        &mut self,
    ) -> std::result::Result<T, String> {
        loop {
            let mut line = String::new();
            let read = self
                .output
                .read_line(&mut line)
                .await
                .map_err(|error| format!("could not read from SSR worker: {error}"))?;
            if read == 0 {
                let status = self
                    .child
                    .try_wait()
                    .map_err(|error| format!("could not inspect SSR worker: {error}"))?;
                return Err(format!("SSR worker closed its output ({status:?})"));
            }
            let Some(message) = line.strip_prefix(PROTOCOL_PREFIX) else {
                tracing::warn!(output = line.trim(), "unexpected SSR worker output");
                continue;
            };
            return serde_json::from_str(message)
                .map_err(|error| format!("invalid SSR worker response: {error}"));
        }
    }

    async fn stop(&mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
        if let Some(path) = self.extracted_bundle.take() {
            let _ = fs::remove_file(path).await;
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(path) = self.extracted_bundle.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}

async fn stop_worker(worker: &mut Option<Worker>) {
    if let Some(mut failed) = worker.take() {
        failed.stop().await;
    }
}

async fn prepare_bundle(
    bundle: &Bundle,
) -> std::result::Result<(PathBuf, Option<PathBuf>), String> {
    match bundle {
        Bundle::Development(path) => {
            if !path.is_file() {
                return Err(format!(
                    "{} does not exist; run `npm run build:ssr`",
                    path.display()
                ));
            }
            Ok((path.clone(), None))
        }
        Bundle::Embedded(bytes) => {
            if bytes.is_empty() {
                return Err(
                    "no SSR bundle was embedded; run `npm run build` before `cargo build --release`"
                        .into(),
                );
            }
            let path = std::env::temp_dir()
                .join(format!("gurthang-{}-inertia-ssr.mjs", std::process::id()));
            fs::write(&path, bytes)
                .await
                .map_err(|error| format!("could not extract SSR bundle: {error}"))?;
            Ok((path.clone(), Some(path)))
        }
    }
}

async fn runtime_version(runtime: &str) -> std::result::Result<String, String> {
    let output = timeout(
        Duration::from_secs(3),
        Command::new(runtime).arg("--version").output(),
    )
    .await
    .map_err(|_| format!("`{runtime} --version` timed out"))?
    .map_err(|error| format!("could not run `{runtime} --version`: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "`{runtime} --version` exited with {}",
            output.status
        ));
    }
    let version = String::from_utf8(output.stdout)
        .map_err(|_| format!("`{runtime} --version` returned non-UTF-8 output"))?
        .trim()
        .to_owned();
    if version.is_empty() {
        return Err(format!("`{runtime} --version` returned no version"));
    }
    Ok(version)
}

fn is_node(runtime: &str) -> bool {
    Path::new(runtime)
        .file_stem()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("node"))
}

fn supported_node_version(version: &str) -> bool {
    version
        .trim_start_matches('v')
        .split('.')
        .next()
        .and_then(|major| major.parse::<u64>().ok())
        .is_some_and(|major| major >= 22)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_version_check_requires_version_22() {
        assert!(supported_node_version("v22.0.0"));
        assert!(supported_node_version("26.2.0"));
        assert!(!supported_node_version("v21.9.0"));
        assert!(!supported_node_version("unknown"));
    }

    #[test]
    fn node_runtime_detection_accepts_an_absolute_path() {
        assert!(is_node("node"));
        assert!(is_node("/usr/local/bin/node"));
        assert!(!is_node("bun"));
    }
}
