use std::{collections::HashMap, fs, path::Path};

use axum::{
    body::Body,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use include_dir::Dir;
use serde::Deserialize;

const ENTRYPOINT: &str = "resources/js/app.tsx";

/// Public URL prefix for the Vite build, served from the embedded `assets/` tree.
pub const VITE_PUBLIC_BASE: &str = "/assets/dist";

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct AssetError(pub String);

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
    pub fn development(server_url: impl Into<String>) -> Self {
        Self::Development {
            server_url: server_url.into().trim_end_matches('/').to_owned(),
        }
    }

    pub fn from_manifest(path: impl AsRef<Path>) -> Result<Self, AssetError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|error| {
            AssetError(format!(
                "could not read {} ({error}); run `npm run build`",
                path.display()
            ))
        })?;
        Self::from_manifest_bytes(&bytes)
    }

    pub fn from_manifest_bytes(bytes: &[u8]) -> Result<Self, AssetError> {
        let manifest: HashMap<String, ManifestEntry> = serde_json::from_slice(bytes)
            .map_err(|error| AssetError(format!("invalid Vite manifest: {error}")))?;
        let entry = manifest
            .get(ENTRYPOINT)
            .cloned()
            .ok_or_else(|| AssetError(format!("Vite manifest has no {ENTRYPOINT} entry")))?;
        Ok(Self::Production { entry })
    }

    pub fn head_tags(&self) -> String {
        match self {
            Self::Development { server_url } => {
                let refresh = development_url(server_url, "@react-refresh");
                let client = development_url(server_url, "@vite/client");
                let entry = development_url(server_url, ENTRYPOINT);
                format!(
                    r#"<script type="module">import RefreshRuntime from '{refresh}'; RefreshRuntime.injectIntoGlobalHook(window); window.$RefreshReg$ = () => {{}}; window.$RefreshSig$ = () => type => type; window.__vite_plugin_react_preamble_installed__ = true;</script>
<script type="module" src="{client}"></script>
<script type="module" src="{entry}"></script>"#
                )
            }
            Self::Production { entry } => {
                let mut tags = entry
                    .css
                    .iter()
                    .map(|file| format!(r#"<link rel="stylesheet" href="{}">"#, public_asset(file)))
                    .collect::<Vec<_>>();
                tags.push(format!(
                    r#"<script type="module" src="{}"></script>"#,
                    public_asset(&entry.file)
                ));
                tags.join("\n")
            }
        }
    }
}

fn development_url(server_url: &str, path: &str) -> String {
    format!(
        "{server_url}{VITE_PUBLIC_BASE}/{}",
        path.trim_start_matches('/')
    )
}

fn public_asset(file: &str) -> String {
    format!("{VITE_PUBLIC_BASE}/{}", file.trim_start_matches('/'))
}

pub fn embedded_response(
    directory: &'static Dir<'static>,
    path: &str,
    immutable: bool,
) -> Response {
    let Some(file) = directory.get_file(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    bytes_response(path, file.contents(), immutable)
}

pub fn bytes_response(path: &str, contents: &[u8], immutable: bool) -> Response {
    if !Path::new(path)
        .components()
        .all(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    let content_type = mime_guess::from_path(path).first_or_octet_stream();
    let mut response = Body::from(contents.to_vec()).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type.as_ref())
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    if immutable {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_tags_use_the_vite_assets_prefix() {
        let tags = AssetResolver::development("http://127.0.0.1:5173").head_tags();
        assert!(tags.contains("http://127.0.0.1:5173/assets/dist/@react-refresh"));
        assert!(tags.contains("http://127.0.0.1:5173/assets/dist/@vite/client"));
        assert!(tags.contains("http://127.0.0.1:5173/assets/dist/resources/js/app.tsx"));
        assert!(!tags.contains("http://127.0.0.1:5173/@vite/client"));
    }

    #[test]
    fn production_manifest_emits_styles_before_script() {
        let temp = tempfile::tempdir().unwrap();
        let manifest = temp.path().join("manifest.json");
        fs::write(
            &manifest,
            r#"{"resources/js/app.tsx":{"file":"app-123.js","css":["app-123.css"]}}"#,
        )
        .unwrap();
        let tags = AssetResolver::from_manifest(manifest).unwrap().head_tags();
        assert!(tags.contains("/assets/dist/app-123.css"));
        assert!(tags.contains("/assets/dist/app-123.js"));
        assert!(tags.find(".css").unwrap() < tags.find(".js").unwrap());
        assert!(!tags.contains("/build/"));
    }
}
