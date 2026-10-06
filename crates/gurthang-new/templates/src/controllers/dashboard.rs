use axum::{
    extract::State,
    http::{HeaderMap, Method, Uri},
    response::Response,
    Router,
};
use serde::Serialize;
use ts_rs::TS;

use crate::{
    app::App,
    error::Result,
    routes::{auth, dashboard},
    services::auth::AuthSession,
};
use gurthang_http::AddRoute;
use gurthang_inertia::{
    InertiaPage, InertiaRenderMode, InertiaRenderer, InertiaRequest, mutation_redirect,
};

use super::shared::{SafeUser, SharedProps};

pub fn register(router: Router<App>) -> Router<App> {
    router.add_route(dashboard::DASHBOARD, show)
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct DashboardProps {
    pub title: String,
    pub status: String,
    pub user: SafeUser,
}

impl InertiaPage for DashboardProps {
    const COMPONENT: &'static str = "Dashboard";
    const RENDER_MODE: InertiaRenderMode = InertiaRenderMode::Ssr;
}

pub fn export_payloads() -> Result<(), Box<dyn std::error::Error>> {
    DashboardProps::export()?;
    Ok(())
}

pub async fn show(
    State(inertia): State<InertiaRenderer>,
    auth: AuthSession,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {
    let Some(user) = auth.user.as_ref() else {
        return Ok(mutation_redirect(auth::LOGIN)?);
    };
    let request = InertiaRequest::from_parts(&method, &uri, &headers);
    let shared = SharedProps::from_auth(&auth).await?;
    Ok(inertia
        .render(
            &request,
            DashboardProps {
                title: "Dashboard".into(),
                status: "Typed Inertia v3 is connected.".into(),
                user: SafeUser::from(&user.0),
            },
            shared,
        )
        .await?)
}
