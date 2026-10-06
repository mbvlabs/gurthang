use axum::{
    extract::State,
    http::{HeaderMap, Method, Uri},
    response::Response,
    Router,
};
use serde::Serialize;
use ts_rs::TS;

use crate::{app::App, error::Result, routes::welcome, services::auth::AuthSession};
use gurthang_http::AddRoute;
use gurthang_inertia::{InertiaPage, InertiaRenderMode, InertiaRenderer, InertiaRequest};

use super::shared::SharedProps;

pub fn register(router: Router<App>) -> Router<App> {
    router.add_route(welcome::WELCOME, show)
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct WelcomeProps {
    pub title: String,
    pub status: String,
}

impl InertiaPage for WelcomeProps {
    const COMPONENT: &'static str = "Welcome";
    const RENDER_MODE: InertiaRenderMode = InertiaRenderMode::Client;
}

pub fn export_payloads() -> Result<(), Box<dyn std::error::Error>> {
    WelcomeProps::export()?;
    Ok(())
}

pub async fn show(
    State(inertia): State<InertiaRenderer>,
    auth: AuthSession,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {
    let shared = SharedProps::from_auth(&auth).await?;
    Ok(inertia
        .render(
            &InertiaRequest::from_parts(&method, &uri, &headers),
            WelcomeProps {
                title: "{{ project_name }}".into(),
                status: "Inertia React is connected.".into(),
            },
            shared,
        )
        .await?)
}
