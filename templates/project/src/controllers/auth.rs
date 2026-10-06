use std::collections::BTreeMap;

use axum::{
    body::to_bytes,
    extract::{Request, State},
    http::{HeaderMap, Method, Uri, header},
    response::Response,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    error::{AppError, Result},
    models::user::{CreateUserData, UserError, normalize_email},
    services::auth::{AuthSession, Credentials, RegistrationError},
    views::inertia::{
        auth::{LoginProps, RegisterProps},
        shared::SharedProps,
    },
};
use gurthang_inertia::{InertiaRequest, mutation_redirect};

const GENERIC_CREDENTIAL_ERROR: &str = "The email or password is incorrect.";

#[derive(Deserialize)]
pub struct AuthForm {
    email: String,
    password: String,
}

pub async fn new_login(
    State(state): State<AppState>,
    auth: AuthSession,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {
    if auth.user.is_some() {
        return Ok(mutation_redirect("/dashboard")?);
    }
    let email = take_old_email(&auth).await?;
    let shared = SharedProps::from_auth(&auth).await?;
    Ok(state
        .inertia
        .render(
            &InertiaRequest::from_parts(&method, &uri, &headers),
            LoginProps { email },
            shared,
        )
        .await?)
}

pub async fn new_register(
    State(state): State<AppState>,
    auth: AuthSession,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {
    if auth.user.is_some() {
        return Ok(mutation_redirect("/dashboard")?);
    }
    let email = take_old_email(&auth).await?;
    let shared = SharedProps::from_auth(&auth).await?;
    Ok(state
        .inertia
        .render(
            &InertiaRequest::from_parts(&method, &uri, &headers),
            RegisterProps { email },
            shared,
        )
        .await?)
}

pub async fn register(mut auth: AuthSession, request: Request) -> Result<Response> {
    let form = parse_auth_form(request).await?;
    let email = normalize_email(&form.email);
    match auth
        .backend
        .register(CreateUserData {
            email: email.clone(),
            password: form.password,
        })
        .await
    {
        Ok(user) => {
            auth.login(&user)
                .await
                .map_err(|error| AppError::Authentication(error.to_string()))?;
            flash(&auth, "Welcome! Your account is ready.").await?;
            Ok(mutation_redirect("/dashboard")?)
        }
        Err(RegistrationError::Validation(errors)) => {
            invalid(&auth, "/register", email, errors).await
        }
        Err(RegistrationError::User(UserError::DuplicateEmail)) => {
            invalid(
                &auth,
                "/register",
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

pub async fn login(mut auth: AuthSession, request: Request) -> Result<Response> {
    let form = parse_auth_form(request).await?;
    let email = normalize_email(&form.email);
    let user = auth
        .authenticate(Credentials {
            email: email.clone(),
            password: form.password,
        })
        .await
        .map_err(|error| AppError::Authentication(error.to_string()))?;
    if let Some(user) = user {
        auth.login(&user)
            .await
            .map_err(|error| AppError::Authentication(error.to_string()))?;
        flash(&auth, "Signed in successfully.").await?;
        return Ok(mutation_redirect("/dashboard")?);
    }
    invalid(
        &auth,
        "/login",
        email,
        BTreeMap::from([("credentials".into(), GENERIC_CREDENTIAL_ERROR.into())]),
    )
    .await
}

pub async fn logout(mut auth: AuthSession) -> Result<Response> {
    auth.logout()
        .await
        .map_err(|error| AppError::Authentication(error.to_string()))?;
    flash(&auth, "Signed out successfully.").await?;
    Ok(mutation_redirect("/login")?)
}

async fn invalid(
    auth: &AuthSession,
    destination: &str,
    email: String,
    errors: BTreeMap<String, String>,
) -> Result<Response> {
    auth.session
        .insert("errors", errors)
        .await
        .map_err(|error| AppError::Session(error.to_string()))?;
    auth.session
        .insert("old.email", email)
        .await
        .map_err(|error| AppError::Session(error.to_string()))?;
    Ok(mutation_redirect(destination)?)
}

async fn take_old_email(auth: &AuthSession) -> Result<Option<String>> {
    auth.session
        .remove("old.email")
        .await
        .map_err(|error| AppError::Session(error.to_string()))
}

async fn flash(auth: &AuthSession, message: &str) -> Result<()> {
    auth.session
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
