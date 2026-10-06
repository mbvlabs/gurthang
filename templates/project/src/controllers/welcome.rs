use axum::{
    extract::State,
    http::{HeaderMap, Method, Uri},
    response::Response,
};

use crate::{
    app::AppState,
    error::Result,
    services::auth::AuthSession,
    views::inertia::{shared::SharedProps, welcome::WelcomeProps},
};
use gurthang_inertia::InertiaRequest;

pub async fn show(
    State(state): State<AppState>,
    auth: AuthSession,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {
    let shared = SharedProps::from_auth(&auth).await?;
    Ok(state
        .inertia
        .render(
            &InertiaRequest::from_parts(&method, &uri, &headers),
            WelcomeProps {
                title: "__GURTHANG_PROJECT_NAME__".into(),
                status: "Inertia React is connected.".into(),
            },
            shared,
        )
        .await?)
}
