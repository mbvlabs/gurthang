use axum::{
    http::{HeaderMap, Method, Uri},
    response::Response,
};
use gurthang_inertia::{InertiaRenderer, InertiaRequest, mutation_redirect};
use serde::Serialize;
use ts_rs::TS;

use crate::{error::Result, routes::auth, services::auth::AuthSession};

use super::shared::{SafeUser, SharedProps};

#[derive(Clone)]
pub struct Dashboard {
    pub inertia: InertiaRenderer,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct DashboardProps {
    pub title: String,
    pub status: String,
    pub user: SafeUser,
}

impl Dashboard {
    pub async fn show(
        self,
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
        Ok(self
            .inertia
            .render_ssr(
                &request,
                "Dashboard",
                DashboardProps {
                    title: "Dashboard".into(),
                    status: "Typed Inertia v3 is connected.".into(),
                    user: SafeUser::from(&user.0),
                },
                shared,
            )
            .await?)
    }
}
