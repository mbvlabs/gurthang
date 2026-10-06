use gurthang::prelude::*;
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

pub fn routes(ctx: &Context) -> Router<Context> {
    let welcome = Welcome {
        inertia: ctx.inertia.clone(),
    };
    mount!(welcome, {
        crate::routes::welcome::WELCOME => get(welcome, Welcome::show),
    })
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
