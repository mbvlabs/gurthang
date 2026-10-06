use axum::{extract::Path as AxumPath, http::StatusCode, response::IntoResponse, response::Response, routing::get};
use rust_embed::RustEmbed;

use gurthang::http::bytes_response;
use gurthang::prelude::*;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

pub fn manifest_bytes() -> &'static [u8] {
    static BYTES: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    match Assets::get("dist/manifest.json") {
        Some(file) => BYTES.get_or_init(|| file.data.into_owned()).as_slice(),
        None => &[],
    }
}

pub async fn serve(AxumPath(path): AxumPath<String>) -> Response {
    match Assets::get(&path) {
        Some(file) => bytes_response(&path, &file.data, path.starts_with("dist/")),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub fn mount(router: Router<Context>, ctx: &Context) -> Router<Context> {
    let enabled = ctx
        .config
        .server
        .middlewares
        .static_assets
        .as_ref()
        .map(|layer| layer.enable)
        .unwrap_or(true);
    if enabled && !ctx.config.is_development() {
        router.route("/assets/{*path}", get(serve))
    } else {
        router
    }
}
