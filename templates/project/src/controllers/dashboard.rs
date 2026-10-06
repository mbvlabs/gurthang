use axum::{
    extract::State,
    http::{HeaderMap, Method, Uri},
    response::Response,
};

use crate::{
    app::AppState,
    error::Result,
    services::auth::AuthSession,
    views::inertia::{
        dashboard::DashboardProps,
        shared::{SafeUser, SharedProps},
    },
};
use gurthang_inertia::{InertiaRequest, mutation_redirect};

pub async fn show(
    State(state): State<AppState>,
    auth: AuthSession,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {
    let Some(user) = auth.user.as_ref() else {
        return Ok(mutation_redirect("/login")?);
    };
    let request = InertiaRequest::from_parts(&method, &uri, &headers);
    let shared = SharedProps::from_auth(&auth).await?;
    Ok(state
        .inertia
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
