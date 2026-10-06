use std::collections::BTreeMap;

use axum::{
    body::to_bytes,
    extract::Request,
    http::{HeaderMap, Method, Uri, header},
    response::Response,
};
use gurthang::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    error::{AppError, Result},
    models::user::{CreateUserData, UserError, normalize_email},
    routes::{auth, dashboard},
    services::auth::{AuthSession, Credentials, RegistrationError},
};

use super::shared::SharedProps;

const GENERIC_CREDENTIAL_ERROR: &str = "The email or password is incorrect.";

#[derive(Clone)]
pub struct Auth {
    pub inertia: InertiaRenderer,
}

#[derive(Deserialize)]
pub struct AuthForm {
    email: String,
    password: String,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct LoginProps {
    pub email: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct RegisterProps {
    pub email: Option<String>,
}

pub fn routes(ctx: &Context) -> Router<Context> {
    let auth = Auth {
        inertia: ctx.inertia.clone(),
    };
    mount!(auth, {
        crate::routes::auth::REGISTER => get(auth, Auth::new_register),
        crate::routes::auth::REGISTER_CREATE => post(auth, Auth::register_user),
        crate::routes::auth::LOGIN => get(auth, Auth::new_login),
        crate::routes::auth::LOGIN_CREATE => post(auth, Auth::login),
        crate::routes::auth::LOGOUT => delete(auth, Auth::logout),
    })
}

impl Auth {
    pub async fn new_login(
        self,
        auth_session: AuthSession,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
    ) -> Result<Response> {
        if auth_session.user.is_some() {
            return Ok(mutation_redirect(dashboard::DASHBOARD)?);
        }
        let email = take_old_email(&auth_session).await?;
        let shared = SharedProps::from_auth(&auth_session).await?;
        Ok(self
            .inertia
            .render(
                &InertiaRequest::from_parts(&method, &uri, &headers),
                "Auth/Login",
                LoginProps { email },
                shared,
            )
            .await?)
    }

    pub async fn new_register(
        self,
        auth_session: AuthSession,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
    ) -> Result<Response> {
        if auth_session.user.is_some() {
            return Ok(mutation_redirect(dashboard::DASHBOARD)?);
        }
        let email = take_old_email(&auth_session).await?;
        let shared = SharedProps::from_auth(&auth_session).await?;
        Ok(self
            .inertia
            .render(
                &InertiaRequest::from_parts(&method, &uri, &headers),
                "Auth/Register",
                RegisterProps { email },
                shared,
            )
            .await?)
    }

    pub async fn register_user(
        self,
        mut auth_session: AuthSession,
        request: Request,
    ) -> Result<Response> {
        let _ = self;
        let form = parse_auth_form(request).await?;
        let email = normalize_email(&form.email);
        match auth_session
            .backend
            .register(CreateUserData {
                email: email.clone(),
                password: form.password,
            })
            .await
        {
            Ok(user) => {
                auth_session
                    .login(&user)
                    .await
                    .map_err(|error| AppError::Authentication(error.to_string()))?;
                flash(&auth_session, "Welcome! Your account is ready.").await?;
                Ok(mutation_redirect(dashboard::DASHBOARD)?)
            }
            Err(RegistrationError::Validation(errors)) => {
                invalid(&auth_session, auth::REGISTER, email, errors).await
            }
            Err(RegistrationError::User(UserError::DuplicateEmail)) => {
                invalid(
                    &auth_session,
                    auth::REGISTER,
                    email,
                    BTreeMap::from([("email".into(), "Email has already been registered.".into())]),
                )
                .await
            }
            Err(error) => {
                tracing::error!(error = %error, "registration failed");
                Err(AppError::Internal)
            }
        }
    }

    pub async fn login(self, mut auth_session: AuthSession, request: Request) -> Result<Response> {
        let _ = self;
        let form = parse_auth_form(request).await?;
        let email = normalize_email(&form.email);
        let user = auth_session
            .authenticate(Credentials {
                email: email.clone(),
                password: form.password,
            })
            .await
            .map_err(|error| AppError::Authentication(error.to_string()))?;
        if let Some(user) = user {
            auth_session
                .login(&user)
                .await
                .map_err(|error| AppError::Authentication(error.to_string()))?;
            flash(&auth_session, "Signed in successfully.").await?;
            return Ok(mutation_redirect(dashboard::DASHBOARD)?);
        }
        invalid(
            &auth_session,
            auth::LOGIN,
            email,
            BTreeMap::from([("credentials".into(), GENERIC_CREDENTIAL_ERROR.into())]),
        )
        .await
    }

    pub async fn logout(self, mut auth_session: AuthSession) -> Result<Response> {
        let _ = self;
        auth_session
            .logout()
            .await
            .map_err(|error| AppError::Authentication(error.to_string()))?;
        flash(&auth_session, "Signed out successfully.").await?;
        Ok(mutation_redirect(auth::LOGIN)?)
    }
}

async fn invalid(
    auth_session: &AuthSession,
    destination: Route,
    email: String,
    errors: BTreeMap<String, String>,
) -> Result<Response> {
    auth_session
        .session
        .insert("errors", errors)
        .await
        .map_err(|error| AppError::Session(error.to_string()))?;
    auth_session
        .session
        .insert("old.email", email)
        .await
        .map_err(|error| AppError::Session(error.to_string()))?;
    Ok(mutation_redirect(destination)?)
}

async fn take_old_email(auth_session: &AuthSession) -> Result<Option<String>> {
    auth_session
        .session
        .remove("old.email")
        .await
        .map_err(|error| AppError::Session(error.to_string()))
}

async fn flash(auth_session: &AuthSession, message: &str) -> Result<()> {
    auth_session
        .session
        .insert("flash.success", message)
        .await
        .map_err(|error| AppError::Session(error.to_string()))
}

async fn parse_auth_form(request: Request) -> Result<AuthForm> {
    let is_json = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"));
    let bytes = to_bytes(request.into_body(), 64 * 1024)
        .await
        .map_err(|_| AppError::BadRequest("could not read form body".into()))?;
    if is_json {
        serde_json::from_slice(&bytes)
            .map_err(|_| AppError::BadRequest("invalid authentication form".into()))
    } else {
        serde_urlencoded::from_bytes(&bytes)
            .map_err(|_| AppError::BadRequest("invalid authentication form".into()))
    }
}
