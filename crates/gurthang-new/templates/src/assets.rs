use axum::{extract::Path as AxumPath, response::Response};
use include_dir::{Dir, include_dir};

use gurthang::http::embedded_response;

static BUILD_ASSETS: Dir<'_> = include_dir!("$OUT_DIR/dist");
static PUBLIC_ASSETS: Dir<'_> = include_dir!("$OUT_DIR/assets");

pub fn manifest_bytes() -> &'static [u8] {
    BUILD_ASSETS
        .get_file(".vite/manifest.json")
        .map(|file| file.contents())
        .unwrap_or_default()
}

pub async fn serve_build(AxumPath(path): AxumPath<String>) -> Response {
    embedded_response(&BUILD_ASSETS, &path, true)
}

pub async fn serve_public(AxumPath(path): AxumPath<String>) -> Response {
    embedded_response(&PUBLIC_ASSETS, &path, false)
}
