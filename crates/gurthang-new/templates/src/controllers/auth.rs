use std::collections::BTreeMap;

use axum::{
    body::to_bytes,
    extract::{Request, State},
    http::{HeaderMap, Method, Uri, header},
    response::Response,
    Router,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    app::App,
    error::{AppError, Result},
    models::user::{CreateUserData, UserError, normalize_email},
    routes::{auth, dashboard},
    services::auth::{AuthSession, Credentials, RegistrationError},
};
use gurthang_http::{AddRoute, Route};
use gurthang_inertia::{
    InertiaPage, InertiaRenderMode, InertiaRenderer, InertiaRequest, mutation_redirect,
};

use super::shared::SharedProps;

const GENERIC_CREDENTIAL_ERROR: &str = "The email or password is incorrect.";

pub fn register(router: Router<App>) -> Router<App> {
    router
        .add_route(auth::REGISTER, new_register)
        .add_route(auth::REGISTER_CREATE, register_user)
        .add_route(auth::LOGIN, new_login)
        .add_route(auth::LOGIN_CREATE, login)
        .add_route(auth::LOGOUT, logout)
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

impl InertiaPage for LoginProps {
    const COMPONENT: &'static str = "Auth/Login";
    const RENDER_MODE: InertiaRenderMode = InertiaRenderMode::Client;
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct RegisterProps {
    pub email: Option<String>,
}

impl InertiaPage for RegisterProps {
    const COMPONENT: &'static str = "Auth/Register";
    const RENDER_MODE: InertiaRenderMode = InertiaRenderMode::Client;
}

pub fn export_payloads() -> Result<(), Box<dyn std::error::Error>> {
    LoginProps::export()?;
    RegisterProps::export()?;
    Ok(())
}

pub async fn new_login(
    State(inertia): State<InertiaRenderer>,
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
    Ok(inertia
        .render(
            &InertiaRequest::from_parts(&method, &uri, &headers),
            LoginProps { email },
            shared,
        )
        .await?)
}

pub async fn new_register(
    State(inertia): State<InertiaRenderer>,
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
    Ok(inertia
        .render(
            &InertiaRequest::from_parts(&method, &uri, &headers),
            RegisterProps { email },
            shared,
        )
        .await?)
}

pub async fn register_user(mut auth_session: AuthSession, request: Request) -> Result<Response> {
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

pub async fn login(mut auth_session: AuthSession, request: Request) -> Result<Response> {
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

pub async fn logout(mut auth_session: AuthSession) -> Result<Response> {
    auth_session
        .logout()
        .await
        .map_err(|error| AppError::Authentication(error.to_string()))?;
    flash(&auth_session, "Signed out successfully.").await?;
    Ok(mutation_redirect(auth::LOGIN)?)
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
