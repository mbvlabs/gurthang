use std::{collections::HashMap, fs, path::Path};

use serde::Deserialize;

use crate::{config::Config, error::AppError};

const ENTRYPOINT: &str = "resources/js/app.tsx";

#[derive(Clone, Debug)]
pub enum AssetResolver {
    Development { server_url: String },
    Production { entry: ManifestEntry },
}

#[derive(Clone, Debug, Deserialize)]
pub struct ManifestEntry {
    pub file: String,
    #[serde(default)]
    pub css: Vec<String>,
}

impl AssetResolver {
    pub fn from_config(config: &Config) -> Result<Self, AppError> {
        if config.is_development() {
            let server_url = config.vite_dev_server_url.clone().ok_or_else(|| {
                AppError::Asset("VITE_DEV_SERVER_URL is required in development".into())
            })?;
            return Ok(Self::Development {
                server_url: server_url.trim_end_matches('/').to_owned(),
            });
        }
        Self::from_manifest("dist/.vite/manifest.json")
    }

    pub fn from_manifest(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|error| {
            AppError::Asset(format!(
                "could not read {} ({error}); run `npm run build`",
                path.display()
            ))
        })?;
        let manifest: HashMap<String, ManifestEntry> = serde_json::from_slice(&bytes)
            .map_err(|error| AppError::Asset(format!("invalid Vite manifest: {error}")))?;
        let entry = manifest
            .get(ENTRYPOINT)
            .cloned()
            .ok_or_else(|| AppError::Asset(format!("Vite manifest has no {ENTRYPOINT} entry")))?;
        Ok(Self::Production { entry })
    }

    pub fn head_tags(&self) -> String {
        match self {
            Self::Development { server_url } => format!(
                r#"<script type="module">import RefreshRuntime from '{server_url}/@react-refresh'; RefreshRuntime.injectIntoGlobalHook(window); window.$RefreshReg$ = () => {{}}; window.$RefreshSig$ = () => type => type; window.__vite_plugin_react_preamble_installed__ = true;</script>
<script type="module" src="{server_url}/@vite/client"></script>
<script type="module" src="{server_url}/{ENTRYPOINT}"></script>"#
            ),
            Self::Production { entry } => {
                let mut tags = entry
                    .css
                    .iter()
                    .map(|file| format!(r#"<link rel="stylesheet" href="/build/{file}">"#))
                    .collect::<Vec<_>>();
                tags.push(format!(
                    r#"<script type="module" src="/build/{}"></script>"#,
                    entry.file
                ));
                tags.join("\n")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_manifest_emits_styles_before_script() {
        let temp = tempfile::tempdir().unwrap();
        let manifest = temp.path().join("manifest.json");
        fs::write(
            &manifest,
            r#"{"resources/js/app.tsx":{"file":"assets/app-123.js","css":["assets/app-123.css"]}}"#,
        )
        .unwrap();
        let tags = AssetResolver::from_manifest(manifest).unwrap().head_tags();
        assert!(tags.contains("/build/assets/app-123.css"));
        assert!(tags.contains("/build/assets/app-123.js"));
        assert!(tags.find(".css").unwrap() < tags.find(".js").unwrap());
    }

    #[test]
    fn missing_entrypoint_is_actionable() {
        let temp = tempfile::tempdir().unwrap();
        let manifest = temp.path().join("manifest.json");
        fs::write(&manifest, "{}").unwrap();
        let error = AssetResolver::from_manifest(manifest)
            .unwrap_err()
            .to_string();
        assert!(error.contains(ENTRYPOINT));
    }
}
