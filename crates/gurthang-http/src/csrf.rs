use axum::{
    extract::{Request, State},
    http::{Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

const COOKIE_NAME: &str = "XSRF-TOKEN";

pub async fn protect(State(session_secure): State<bool>, request: Request, next: Next) -> Response {
    let token = cookie_value(request.headers(), COOKIE_NAME);
    if is_state_changing(request.method()) && !token_matches(request.headers(), token.as_deref()) {
        return (StatusCode::FORBIDDEN, "CSRF verification failed").into_response();
    }

    let needs_cookie = token.is_none();
    let mut response = next.run(request).await;
    if needs_cookie {
        let secure = if session_secure { "; Secure" } else { "" };
        let value = format!(
            "{COOKIE_NAME}={}; Path=/; SameSite=Lax{secure}",
            Uuid::new_v4()
        );
        if let Ok(value) = value.parse() {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
    }
    response
}

fn is_state_changing(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

fn token_matches(headers: &axum::http::HeaderMap, token: Option<&str>) -> bool {
    let supplied = headers
        .get("x-xsrf-token")
        .and_then(|value| value.to_str().ok());
    token.is_some() && supplied == token
}

fn cookie_value(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .find_map(|(cookie_name, value)| (cookie_name == name).then(|| value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_exact_cookie_name() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            header::COOKIE,
            "other=1; XSRF-TOKEN=abc-123".parse().unwrap(),
        );
        assert_eq!(
            cookie_value(&headers, COOKIE_NAME).as_deref(),
            Some("abc-123")
        );
    }

    #[test]
    fn valid_cookie_and_header_pair_is_accepted() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-xsrf-token", "abc-123".parse().unwrap());
        assert!(token_matches(&headers, Some("abc-123")));
        assert!(!token_matches(&headers, Some("different")));
        assert!(!token_matches(&headers, None));
    }
}
