use std::marker::PhantomData;

use axum::{
    Router,
    extract::Request,
    http::{StatusCode, header::ACCEPT},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_login::{AuthSession, AuthnBackend};
use serde::{Deserialize, Serialize};

use crate::{
    app::Context,
    controller::middleware::{MiddlewareKind, MiddlewareLayer},
    error::Result,
};

/// Leftover YAML. Runtime membership and values do not read this.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub public_paths: Vec<String>,
    #[serde(default)]
    pub redirect: Option<String>,
    #[serde(default)]
    pub status: u16,
}

/// Pre-processing middleware: the request must be logged in.
///
/// Attach it with [`RouteGroupExt::add_mw`](crate::RouteGroupExt::add_mw) to
/// the routes that need a user. Inertia and HTML requests redirect to
/// `redirect`; other clients get 401.
pub struct RequireAuth<B> {
    redirect: String,
    marker: PhantomData<fn() -> B>,
}

impl<B> RequireAuth<B> {
    pub fn new(redirect: impl Into<String>) -> Self {
        Self {
            redirect: redirect.into(),
            marker: PhantomData,
        }
    }
}

impl<B> MiddlewareLayer for RequireAuth<B>
where
    B: AuthnBackend + Clone + Send + Sync + 'static,
{
    fn name(&self) -> &'static str {
        "authn"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Pre
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        Ok(serde_json::json!({ "redirect": self.redirect }))
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        let redirect = self.redirect.clone();
        Ok(app.layer(axum::middleware::from_fn(
            move |auth: AuthSession<B>, request: Request, next: Next| {
                let redirect = redirect.clone();
                async move { require_auth(redirect, auth, request, next).await }
            },
        )))
    }
}

async fn require_auth<B>(
    redirect: String,
    auth: AuthSession<B>,
    request: Request,
    next: Next,
) -> Response
where
    B: AuthnBackend + Clone + Send + Sync + 'static,
{
    if auth.user.is_some() {
        return next.run(request).await;
    }

    if wants_redirect(request.headers()) {
        return gurthang_inertia::mutation_redirect(redirect)
            .unwrap_or_else(|_| StatusCode::UNAUTHORIZED.into_response());
    }

    StatusCode::UNAUTHORIZED.into_response()
}

fn wants_redirect(headers: &axum::http::HeaderMap) -> bool {
    if headers
        .get("x-inertia")
        .and_then(|value| value.to_str().ok())
        == Some("true")
    {
        return true;
    }
    headers
        .get(ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|accept| {
            accept.split(',').any(|part| {
                matches!(
                    part.trim().split(';').next().unwrap_or("").trim(),
                    "text/html" | "application/xhtml+xml"
                )
            })
        })
}

/// Placeholder used by the framework default stack. The application replaces it
/// or attaches [`RequireAuth`] on a sub-router; the framework cannot know the
/// auth backend type.
pub struct AuthnPlaceholder;

pub fn placeholder() -> AuthnPlaceholder {
    AuthnPlaceholder
}

impl MiddlewareLayer for AuthnPlaceholder {
    fn name(&self) -> &'static str {
        "authn"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Pre
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        Ok(serde_json::json!({}))
    }

    fn apply(&self, app: Router<Context>) -> Result<Router<Context>> {
        Ok(app)
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::wants_redirect;

    #[test]
    fn inertia_and_html_redirect() {
        let mut headers = HeaderMap::new();
        headers.insert("x-inertia", HeaderValue::from_static("true"));
        assert!(wants_redirect(&headers));

        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ACCEPT,
            HeaderValue::from_static("text/html"),
        );
        assert!(wants_redirect(&headers));
    }

    #[test]
    fn json_does_not_redirect() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ACCEPT,
            HeaderValue::from_static("application/json"),
        );
        assert!(!wants_redirect(&headers));
        assert!(!wants_redirect(&HeaderMap::new()));
    }
}
