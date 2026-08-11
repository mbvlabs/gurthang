use std::sync::Arc;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use __GURTHANG_CRATE_NAME__::{
    app::AppState,
    config::Config,
    models::user::{User, UserError, normalize_email},
    routes,
    web::{assets::AssetResolver, inertia::InertiaRenderer, tera::TeraEngine},
};
use sqlx::PgPool;
use tower::ServiceExt;

async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = PgPool::connect(&url).await.ok()?;
    sqlx::migrate!().run(&pool).await.ok()?;
    Some(pool)
}

fn test_app(pool: PgPool) -> axum::Router {
    let config = Arc::new(Config {
        app_env: "development".into(),
        app_host: "127.0.0.1".into(),
        app_port: 3000,
        app_url: "http://127.0.0.1:3000".into(),
        database_url: "postgres://unused".into(),
        session_secure: false,
        vite_dev_server_url: Some("http://127.0.0.1:5173".into()),
        inertia_ssr_runtime: "node".into(),
        inertia_ssr_timeout_ms: 5_000,
    });
    let templates = TeraEngine::load("templates/**/*.html").unwrap();
    let inertia = InertiaRenderer::new(
        templates.clone(),
        AssetResolver::Development {
            server_url: "http://127.0.0.1:5173".into(),
        },
        "test",
    );
    routes::router(AppState::new(pool, config, templates, inertia))
}

fn response_cookie(response: &axum::response::Response, name: &str) -> Option<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with(&format!("{name}=")))
        .and_then(|value| value.split(';').next())
        .map(str::to_owned)
}

#[tokio::test]
async fn postgres_user_round_trip_hashes_password_and_enforces_normalized_uniqueness() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping PostgreSQL integration test; set TEST_DATABASE_URL");
        return;
    };
    let email = format!("gurthang-{}@example.test", uuid::Uuid::new_v4());
    let normalized = normalize_email(&format!("  {}  ", email.to_uppercase()));
    let hash = User::hash_password("correct horse battery staple".into())
        .await
        .unwrap();
    assert!(hash.starts_with("$argon2id$"));

    let mut transaction = pool.begin().await.unwrap();
    let user = User::create(&mut transaction, normalized.clone(), hash)
        .await
        .unwrap();
    assert_eq!(user.email, normalized);
    assert!(
        user.verify_password("correct horse battery staple".into())
            .await
            .unwrap()
    );
    assert!(!user.verify_password("wrong password".into()).await.unwrap());

    let duplicate_hash = User::hash_password("another safe password".into())
        .await
        .unwrap();
    let duplicate = User::create(&mut transaction, normalized, duplicate_hash).await;
    assert!(matches!(duplicate, Err(UserError::DuplicateEmail)));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn sessions_migration_matches_the_postgres_store_columns() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping PostgreSQL integration test; set TEST_DATABASE_URL");
        return;
    };
    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = 'tower_sessions' \
         ORDER BY column_name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(columns, ["data", "expiry_date", "id"]);
}

#[tokio::test]
async fn registration_persists_session_and_logout_revokes_protected_access() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping PostgreSQL integration test; set TEST_DATABASE_URL");
        return;
    };
    let email = format!("flow-{}@example.test", uuid::Uuid::new_v4());
    let app = test_app(pool.clone());

    let register_page = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/register")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(register_page.status(), StatusCode::OK);
    let xsrf_cookie = response_cookie(&register_page, "XSRF-TOKEN").unwrap();
    let xsrf = xsrf_cookie.split_once('=').unwrap().1;

    let body = format!(
        "email={}&password=correct-horse-battery-staple",
        email.replace('@', "%40")
    );
    let registered = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/register")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, &xsrf_cookie)
                .header("x-xsrf-token", xsrf)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(registered.status(), StatusCode::SEE_OTHER);
    assert_eq!(registered.headers()[header::LOCATION], "/dashboard");
    let session_cookie = response_cookie(&registered, "gurthang.sid").unwrap();
    let cookies = format!("{xsrf_cookie}; {session_cookie}");

    let dashboard = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/dashboard")
                .header(header::COOKIE, &cookies)
                .header("x-inertia", "true")
                .header("x-inertia-version", "test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dashboard.status(), StatusCode::OK);
    let dashboard_body = to_bytes(dashboard.into_body(), usize::MAX).await.unwrap();
    let page: serde_json::Value = serde_json::from_slice(&dashboard_body).unwrap();
    assert_eq!(page["props"]["user"]["email"], email);
    assert!(page.to_string().find("password_hash").is_none());

    let logout = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/logout")
                .header(header::COOKIE, &cookies)
                .header("x-xsrf-token", xsrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout.status(), StatusCode::SEE_OTHER);

    let denied = app
        .oneshot(
            Request::builder()
                .uri("/dashboard")
                .header(header::COOKIE, &cookies)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::SEE_OTHER);
    assert_eq!(denied.headers()[header::LOCATION], "/login");

    sqlx::query("DELETE FROM users WHERE email = $1")
        .bind(&email)
        .execute(&pool)
        .await
        .unwrap();
}
