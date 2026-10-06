use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Router,
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::get,
};
use gurthang::{
    Config, Context, Environment, StartMode,
    controller::middleware::{
        MiddlewareKind, MiddlewareLayer, MiddlewareStack, apply_stack, timeout,
    },
    inertia::InertiaRenderer,
    jobs::JobQueue,
};
use tower::ServiceExt;

const SAMPLE: &str = r#"
server:
  host: 127.0.0.1
  port: 3000
  url: http://127.0.0.1:3000
session:
  secure: false
inertia:
  ssr_runtime: node
  ssr_timeout_ms: 5000
workers:
  concurrency: 1
  poll_interval_ms: 1000
  lease_seconds: 10
  timeout_seconds: 5
logger:
  level: info
"#;

fn context() -> Context {
    let db = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@localhost/gurthang")
        .expect("lazy pool");
    let config = Arc::new(Config::from_yaml(SAMPLE).unwrap());
    let (_, shutdown) = tokio::sync::watch::channel(false);
    Context::new(
        Environment::Test,
        db.clone(),
        config,
        InertiaRenderer::new("", "test", ""),
        JobQueue::new(db),
        None,
        StartMode::All,
        shutdown,
    )
}

/// A middleware that records when it runs and can reject a request early.
struct Recorder {
    name: &'static str,
    kind: MiddlewareKind,
    log: Arc<Mutex<Vec<String>>>,
    reject: Option<StatusCode>,
}

impl MiddlewareLayer for Recorder {
    fn name(&self) -> &'static str {
        self.name
    }

    fn kind(&self) -> MiddlewareKind {
        self.kind
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        Ok(serde_json::Value::Null)
    }

    fn apply(&self, app: Router<Context>) -> gurthang::Result<Router<Context>> {
        let log = Arc::clone(&self.log);
        let name = self.name;
        let kind = self.kind;
        let reject = self.reject;
        Ok(app.layer(axum::middleware::from_fn(
            move |request: Request, next: Next| {
                let log = Arc::clone(&log);
                async move {
                    match kind {
                        MiddlewareKind::Wrap => {
                            log.lock().unwrap().push(format!("{name}:in"));
                            let response = next.run(request).await;
                            log.lock().unwrap().push(format!("{name}:out"));
                            response
                        }
                        MiddlewareKind::Post => {
                            let response = next.run(request).await;
                            log.lock()
                                .unwrap()
                                .push(format!("{name}:{}", response.status().as_u16()));
                            response
                        }
                        MiddlewareKind::Pre => {
                            log.lock().unwrap().push(name.to_string());
                            match reject {
                                Some(status) => status.into_response(),
                                None => next.run(request).await,
                            }
                        }
                    }
                }
            },
        )))
    }
}

fn recorder(
    name: &'static str,
    kind: MiddlewareKind,
    log: &Arc<Mutex<Vec<String>>>,
    reject: Option<StatusCode>,
) -> Box<dyn MiddlewareLayer> {
    Box::new(Recorder {
        name,
        kind,
        log: Arc::clone(log),
        reject,
    })
}

async fn send(router: Router<Context>, ctx: Context, uri: &str) -> Response {
    router
        .with_state(ctx)
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn wrap_nests_around_pre_and_post_in_order() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let handler_log = Arc::clone(&log);

    let router = Router::<Context>::new().route(
        "/",
        get(move || {
            let handler_log = Arc::clone(&handler_log);
            async move {
                handler_log.lock().unwrap().push("handler".into());
                (StatusCode::OK, "ok")
            }
        }),
    );

    let stack = MiddlewareStack::new()
        .with(recorder("wrap", MiddlewareKind::Wrap, &log, None))
        .with(recorder("pre", MiddlewareKind::Pre, &log, None))
        .with(recorder("post", MiddlewareKind::Post, &log, None));

    let router = apply_stack(router, stack).unwrap();
    let response = send(router, context(), "/").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        *log.lock().unwrap(),
        vec!["wrap:in", "pre", "handler", "post:200", "wrap:out"]
    );
}

#[tokio::test]
async fn pre_early_return_skips_handler_but_post_still_runs() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let handler_log = Arc::clone(&log);

    let router = Router::<Context>::new().route(
        "/",
        get(move || {
            let handler_log = Arc::clone(&handler_log);
            async move {
                handler_log.lock().unwrap().push("handler".into());
                (StatusCode::IM_A_TEAPOT, "teapot")
            }
        }),
    );

    let stack = MiddlewareStack::new()
        .with(recorder("wrap", MiddlewareKind::Wrap, &log, None))
        .with(recorder(
            "authn",
            MiddlewareKind::Pre,
            &log,
            Some(StatusCode::UNAUTHORIZED),
        ))
        .with(recorder("post", MiddlewareKind::Post, &log, None));

    let router = apply_stack(router, stack).unwrap();
    let response = send(router, context(), "/").await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    // The handler never ran, but the post middleware still observed the final
    // (early) status and the wrapping middleware still completed.
    assert_eq!(
        *log.lock().unwrap(),
        vec!["wrap:in", "authn", "post:401", "wrap:out"]
    );
}

#[tokio::test]
async fn post_sees_the_handler_status() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let router = Router::<Context>::new().route(
        "/",
        get(|| async { (StatusCode::IM_A_TEAPOT, "teapot") }),
    );

    let stack =
        MiddlewareStack::new().with(recorder("post", MiddlewareKind::Post, &log, None));

    let router = apply_stack(router, stack).unwrap();
    let response = send(router, context(), "/").await;
    assert_eq!(response.status(), StatusCode::IM_A_TEAPOT);
    assert_eq!(*log.lock().unwrap(), vec!["post:418"]);
}

#[tokio::test]
async fn wrap_timeout_fires() {
    let router = Router::<Context>::new().route(
        "/",
        get(|| async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            (StatusCode::OK, "late")
        }),
    );

    let stack = MiddlewareStack::new().with(Box::new(timeout::TimeOut {
        enable: true,
        timeout: 20,
    }));

    let router = apply_stack(router, stack).unwrap();
    let response = send(router, context(), "/").await;
    assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
}

#[derive(Clone, Debug)]
struct TestUser;

impl axum_login::AuthUser for TestUser {
    type Id = i64;

    fn id(&self) -> Self::Id {
        1
    }

    fn session_auth_hash(&self) -> &[u8] {
        b"hash"
    }
}

#[derive(Clone)]
struct TestBackend;

#[derive(Clone)]
struct TestCredentials;

#[derive(Debug, thiserror::Error)]
#[error("test auth error")]
struct TestAuthError;

impl axum_login::AuthnBackend for TestBackend {
    type User = TestUser;
    type Credentials = TestCredentials;
    type Error = TestAuthError;

    async fn authenticate(
        &self,
        _credentials: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        Ok(None)
    }

    async fn get_user(
        &self,
        _user_id: &axum_login::UserId<Self>,
    ) -> Result<Option<Self::User>, Self::Error> {
        Ok(None)
    }
}

fn session_layer() -> tower_sessions::SessionManagerLayer<tower_sessions::MemoryStore> {
    tower_sessions::SessionManagerLayer::new(tower_sessions::MemoryStore::default())
        .with_secure(false)
}

#[tokio::test]
async fn authn_redirects_unauthenticated_requests() {
    let stack = MiddlewareStack::new().with(Box::new(
        gurthang::controller::middleware::authn::RequireAuth::<TestBackend>::new(
            gurthang::controller::middleware::authn::Config {
                enable: true,
                public_paths: vec!["/".into(), "/login".into()],
                redirect: Some("/login".into()),
                status: 401,
            },
        ),
    ));

    let router = Router::<Context>::new()
        .route("/dashboard", get(|| async { "dashboard" }))
        .route("/login", get(|| async { "login" }));

    // The session wrap is outermost, exactly as it is in the default stack.
    let auth_layer = axum_login::AuthManagerLayerBuilder::new(TestBackend, session_layer()).build();
    let router = apply_stack(router, stack).unwrap().layer(auth_layer);

    let protected = send(router.clone(), context(), "/dashboard").await;
    assert_eq!(protected.status(), StatusCode::SEE_OTHER);
    assert_eq!(protected.headers().get("location").unwrap(), "/login");

    let public = send(router, context(), "/login").await;
    assert_eq!(public.status(), StatusCode::OK);
}

