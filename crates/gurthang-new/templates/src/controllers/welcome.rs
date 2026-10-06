use axum::{
    http::{HeaderMap, Method, Uri},
    response::Response,
};
use gurthang_inertia::{InertiaRenderer, InertiaRequest};
use serde::Serialize;
use ts_rs::TS;

use crate::{error::Result, services::auth::AuthSession};

use super::shared::SharedProps;

#[derive(Clone)]
pub struct Welcome {
    pub inertia: InertiaRenderer,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct WelcomeProps {
    pub title: String,
    pub status: String,
}

impl Welcome {
    pub async fn show(
        self,
        auth: AuthSession,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
    ) -> Result<Response> {
        let shared = SharedProps::from_auth(&auth).await?;
        Ok(self
            .inertia
            .render(
                &InertiaRequest::from_parts(&method, &uri, &headers),
                "Welcome",
                WelcomeProps {
                    title: "{{ project_name }}".into(),
                    status: "Inertia React is connected.".into(),
                },
                shared,
            )
            .await?)
    }
}
