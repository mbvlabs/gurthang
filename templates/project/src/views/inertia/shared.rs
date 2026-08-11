use std::collections::BTreeMap;

use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    models::user::User,
    services::auth::AuthSession,
};

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct SafeUser {
    pub id: Uuid,
    pub email: String,
}

#[derive(Clone, Debug, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct AuthProps {
    pub user: Option<SafeUser>,
}

#[derive(Clone, Debug, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct FlashProps {
    pub success: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct SharedProps {
    pub auth: AuthProps,
    pub errors: BTreeMap<String, String>,
    pub flash: FlashProps,
}

impl SharedProps {
    pub fn anonymous() -> Self {
        Self::default()
    }

    pub async fn from_auth(auth: &AuthSession) -> Result<Self> {
        let errors = auth
            .session
            .remove("errors")
            .await
            .map_err(|error| AppError::Session(error.to_string()))?
            .unwrap_or_default();
        let success = auth
            .session
            .remove("flash.success")
            .await
            .map_err(|error| AppError::Session(error.to_string()))?;
        Ok(Self {
            auth: AuthProps {
                user: auth.user.as_ref().map(SafeUser::from),
            },
            errors,
            flash: FlashProps { success },
        })
    }
}

impl From<&User> for SafeUser {
    fn from(user: &User) -> Self {
        Self {
            id: user.id,
            email: user.email.clone(),
        }
    }
}
