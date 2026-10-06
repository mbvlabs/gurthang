use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread,
    time::{Duration, Instant},
};

use gurthang_project::{GurthangToml, find_root_from};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

const RESTART_DEBOUNCE: Duration = Duration::from_millis(100);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

enum DevelopmentEvent {
    Files(notify::Result<Event>),
    Shutdown,
}

#[derive(Debug)]
pub enum Error {
    Io { context: String, source: std::io::Error },
    Development(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { context, source } => write!(formatter, "{context}: {source}"),
            Self::Development(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<gurthang_project::Error> for Error {
    fn from(error: gurthang_project::Error) -> Self {
        Self::Development(error.to_string())
    }
}

pub fn execute(out: &mut impl Write) -> Result<()> {
    let current = std::env::current_dir()
        .map_err(|error| Error::io("could not determine current directory", error))?;
    let root = find_root_from(&current)?;
    validate_project(&root)?;

    let (sender, receiver) = mpsc::channel();
    let _watcher = watch_project(&root, sender.clone())?;
    install_signal_handler(sender)?;

    writeln!(
        out,
        "Starting Gurthang development server in {}",
        root.display()
    )
    .map_err(|error| Error::io("could not write development status", error))?;
    writeln!(out, "  application: http://127.0.0.1:3000")
        .map_err(|error| Error::io("could not write development status", error))?;

    let mut processes = Processes::start(&root)?;
    let result = supervise(&root, &receiver, &mut processes, out);
    processes.stop_all();
    result
}

fn supervise(
    root: &Path,
    receiver: &Receiver<DevelopmentEvent>,
    processes: &mut Processes,
    out: &mut impl Write,
) -> Result<()> {
    let mut restart_at = None;

    loop {
        match receiver.recv_timeout(POLL_INTERVAL) {
            Ok(DevelopmentEvent::Shutdown) => return Ok(()),
            Ok(DevelopmentEvent::Files(event)) => {
                if backend_change(root, event, out)? {
                    restart_at = Some(Instant::now() + RESTART_DEBOUNCE);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(Error::Development("development watcher stopped".into()));
            }
        }

        while let Ok(event) = receiver.try_recv() {
            match event {
                DevelopmentEvent::Shutdown => return Ok(()),
                DevelopmentEvent::Files(event) => {
                    if backend_change(root, event, out)? {
                        restart_at = Some(Instant::now() + RESTART_DEBOUNCE);
                    }
                }
            }
        }

        if restart_at.is_some_and(|deadline| Instant::now() >= deadline) {
            writeln!(
                out,
                "[gurthang] Rust or configuration changed; restarting backend"
            )
            .map_err(|error| Error::io("could not write development status", error))?;
            processes.restart_backend()?;
            restart_at = None;
        }

        processes.check(out)?;
    }
}

fn backend_change(root: &Path, event: notify::Result<Event>, out: &mut impl Write) -> Result<bool> {
    let event = match event {
        Ok(event) => event,
        Err(error) => {
            writeln!(out, "[gurthang] watcher warning: {error}")
                .map_err(|error| Error::io("could not write watcher warning", error))?;
            return Ok(false);
        }
    };
    if !matches!(
        event.kind,
        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) {
        return Ok(false);
    }

    let source = root.join("src");
    let config = root.join("config");
    Ok(event.paths.iter().any(|path| {
        (path.starts_with(&source) && path.extension().is_some_and(|extension| extension == "rs"))
            || path.starts_with(&config)
            || (path.parent() == Some(root)
                && path.file_name().is_some_and(|name| {
                    name == "Cargo.toml" || name == ".env" || name == "gurthang.toml"
                }))
    }))
}

fn watch_project(root: &Path, sender: Sender<DevelopmentEvent>) -> Result<RecommendedWatcher> {
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = sender.send(DevelopmentEvent::Files(event));
    })
    .map_err(|error| Error::Development(format!("could not start file watcher: {error}")))?;
    watcher
        .watch(&root.join("src"), RecursiveMode::Recursive)
        .map_err(|error| Error::Development(format!("could not watch src: {error}")))?;
    let config = root.join("config");
    if config.is_dir() {
        watcher
            .watch(&config, RecursiveMode::Recursive)
            .map_err(|error| Error::Development(format!("could not watch config: {error}")))?;
    }
    watcher
        .watch(root, RecursiveMode::NonRecursive)
        .map_err(|error| Error::Development(format!("could not watch project root: {error}")))?;
    Ok(watcher)
}

fn install_signal_handler(sender: Sender<DevelopmentEvent>) -> Result<()> {
    ctrlc::set_handler(move || {
        let _ = sender.send(DevelopmentEvent::Shutdown);
    })
    .map_err(|error| Error::Development(format!("could not install signal handler: {error}")))
}

fn validate_project(root: &Path) -> Result<()> {
    for relative in ["src", "resources/js", "migrations"] {
        if !root.join(relative).exists() {
            return Err(Error::Development(format!(
                "{} does not look like a Gurthang application: missing {relative}",
                root.display()
            )));
        }
    }
    Ok(())
}

fn spawn_backend(root: &Path) -> Result<ManagedChild> {
    let bin = GurthangToml::load(root)?.project.name;
    let args = ["run", "--bin", bin.as_str()];
    ManagedChild::spawn(root, "backend", "cargo", &args)
}

struct Processes {
    root: PathBuf,
    frontend: ManagedChild,
    backend: Option<ManagedChild>,
}

impl Processes {
    fn start(root: &Path) -> Result<Self> {
        let frontend = ManagedChild::spawn(root, "Vite", "npm", &["run", "dev"])?;
        let backend = spawn_backend(root)?;
        Ok(Self {
            root: root.to_path_buf(),
            frontend,
            backend: Some(backend),
        })
    }

    fn restart_backend(&mut self) -> Result<()> {
        if let Some(mut backend) = self.backend.take() {
            backend.stop();
        }
        self.backend = Some(spawn_backend(&self.root)?);
        Ok(())
    }

    fn check(&mut self, out: &mut impl Write) -> Result<()> {
        if let Some(status) = self.frontend.status()? {
            return Err(process_stopped("Vite", status));
        }
        if let Some(backend) = &mut self.backend
            && let Some(status) = backend.status()?
        {
            writeln!(
                out,
                "[gurthang] backend stopped ({status}); edit a Rust file to retry"
            )
            .map_err(|error| Error::io("could not write backend status", error))?;
            self.backend = None;
        }
        Ok(())
    }

    fn stop_all(&mut self) {
        if let Some(mut backend) = self.backend.take() {
            backend.stop();
        }
        self.frontend.stop();
    }
}

impl Drop for Processes {
    fn drop(&mut self) {
        self.stop_all();
    }
}

struct ManagedChild {
    name: &'static str,
    child: Child,
}

impl ManagedChild {
    fn spawn(root: &Path, name: &'static str, program: &str, args: &[&str]) -> Result<Self> {
        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(root)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        configure_process_group(&mut command);
        let child = command.spawn().map_err(|error| {
            Error::io(format!("could not start {name} with `{program}`"), error)
        })?;
        Ok(Self { name, child })
    }

    fn status(&mut self) -> Result<Option<ExitStatus>> {
        self.child
            .try_wait()
            .map_err(|error| Error::io(format!("could not inspect {}", self.name), error))
    }

    fn stop(&mut self) {
        if self.child.try_wait().ok().flatten().is_some() {
            return;
        }
        terminate_process_group(&mut self.child, false);
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }
        terminate_process_group(&mut self.child, true);
        let _ = self.child.wait();
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        self.stop();
    }
}

fn process_stopped(name: &str, status: ExitStatus) -> Error {
    Error::Development(format!("{name} stopped unexpectedly ({status})"))
}

fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

fn terminate_process_group(child: &mut Child, force: bool) {
    use nix::{
        sys::signal::{Signal, killpg},
        unistd::Pid,
    };

    let signal = if force {
        Signal::SIGKILL
    } else {
        Signal::SIGTERM
    };
    let _ = killpg(Pid::from_raw(child.id() as i32), signal);
}
