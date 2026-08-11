use std::sync::Arc;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use __GURTHANG_CRATE_NAME__::{
    app::AppState,
    config::Config,
    routes,
    web::{assets::AssetResolver, inertia::InertiaRenderer, tera::TeraEngine},
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

fn app() -> axum::Router {
    let database = PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@127.0.0.1/gurthang_test")
        .unwrap();
    let config = Arc::new(Config {
        app_env: "development".into(),
        app_host: "127.0.0.1".into(),
        app_port: 3000,
        app_url: "http://127.0.0.1:3000".into(),
        database_url: "postgres://unused".into(),
        session_secure: false,
        vite_dev_server_url: Some("http://127.0.0.1:5173".into()),
    });
    let templates = TeraEngine::load("templates/**/*.html").unwrap();
    let inertia = InertiaRenderer::new(
        templates.clone(),
        AssetResolver::Development {
            server_url: "http://127.0.0.1:5173".into(),
        },
        "test",
    );
    routes::router(AppState::new(database, config, templates, inertia))
}

#[tokio::test]
async fn public_page_uses_tera_and_sets_readable_xsrf_cookie() {
    let response = app()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(cookie.starts_with("XSRF-TOKEN="));
    assert!(cookie.contains("SameSite=Lax"));
    assert!(!cookie.contains("HttpOnly"));
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("Gurthang proof of concept"));
    assert!(html.contains("data-on:click"));
}

#[tokio::test]
async fn csrf_rejects_state_changes_without_matching_cookie_and_header() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/login")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("email=a%40b.test&password=not-a-secret"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn datastar_endpoint_returns_a_valid_patch_elements_event() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/demo/counter?datastar=%7B%22count%22%3A4%7D")
                .header("datastar-request", "true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let event = String::from_utf8(body.to_vec()).unwrap();
    assert!(event.contains("event: datastar-patch-elements"));
    assert!(event.contains("selector #counter"));
    assert!(event.contains("Server-rendered count: <strong>5</strong>"));
}
