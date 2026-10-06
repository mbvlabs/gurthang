# Middleware

Gurthang middleware follows Pavex's three kinds. Each kind answers a different
question about *when* a layer runs relative to the handler.

| Kind   | Starts         | Completes      | Pavex name       | Use it for |
|--------|----------------|----------------|------------------|------------|
| `wrap` | before handler | after handler  | wrapping         | owning the rest of the pipeline: tracing spans, timeouts, sessions |
| `post` | after handler  | after handler  | post-processing  | reacting to the response: logging, metrics, headers |
| `pre`  | before handler | before handler | pre-processing   | rejecting or redirecting early: auth, authorization, rate limits |

## Ordering

The pipeline is composed as `wrap -> post -> pre -> handler`, outermost first:

```
wrap_1 .. wrap_n        (registration order, first is outermost)
  post_1 .. post_m      (run after the handler in registration order)
    pre_1 .. pre_k      (run before the handler in registration order)
      handler
```

Rules:

- Registration order only matters **within** a kind.
- `pre` middleware run before the handler and may return an early response
  without calling the handler.
- `post` middleware wrap the `pre` middleware and the handler, so they observe
  the final response even when a `pre` middleware returns early.
- `wrap` middleware enclose everything, so a timeout or tracing span covers the
  whole pipeline.
- `MiddlewareStack::validate` runs at startup and rejects duplicate names or
  operations against unknown names, so mis-wiring fails fast instead of
  silently doing nothing.

## Default stack

`default_middleware_stack` (built from `config.server.middlewares`) registers:

| Kind   | Name              | Default | Notes |
|--------|-------------------|---------|-------|
| `wrap` | `catch_panic`     | on      | catches panics from the rest of the pipeline |
| `wrap` | `session_auth`    | on      | session load/persist; the app swaps in `SessionAuthLayer<B>` |
| `wrap` | `csrf`            | on      | XSRF check + cookie issuance |
| `wrap` | `request_id`      | on      | ensures and propagates `x-request-id` |
| `wrap` | `telemetry`       | on      | tracing span over the rest of the pipeline |
| `wrap` | `timeout_request` | on      | 30s default |
| `wrap` | `cors`            | off     | |
| `wrap` | `limit_payload`   | on      | 2 MB default body limit |
| `wrap` | `static`          | on      | development asset mount |
| `wrap` | `fallback`        | dev     | |
| `post` | `compression`     | off     | |
| `post` | `etag`            | on      | |
| `post` | `powered_by`      | on      | |
| `post` | `secure_headers`  | off     | |
| `post` | `metrics`         | on      | `MetricsRecorder` trait, `TracingMetrics` default |
| `post` | `logger`          | on      | method, path, status, duration, request id |
| `pre`  | `remote_ip`       | off     | reads `x-forwarded-for` |
| `pre`  | `rate_limit`      | on      | 100 requests / 1s per IP |
| `pre`  | `authn`           | off     | the app swaps in `RequireAuth<B>` |
| `pre`  | `authz`           | off     | the app swaps in `RequireAuthorizer` |

Telemetry and timeout are on by default. Rate limiting is on with a generous
ceiling; authorization is off until an app installs an authorizer. Run
`gurthang middleware` to print the composed stack with kind, name, and status.

## Configuring

`config.server.middlewares` is YAML. Each middleware reads its own section, for
example:

```yaml
server:
  middlewares:
    timeout_request:
      enable: true
      timeout: 30000
    rate_limit:
      enable: true
      max_requests: 100
      window_ms: 1000
    authn:
      enable: true
      public_paths: ["/", "/login", "/register", "/assets"]
      redirect: /login
```

## Adding a custom middleware

Implement `MiddlewareLayer` and choose a kind. `apply` is the escape hatch onto
Axum/Tower: it receives the router and returns it with the layer installed.

```rust
use gurthang::prelude::*;
use axum::{extract::Request, middleware::Next, response::Response};

struct MyPre;

impl MiddlewareLayer for MyPre {
    fn name(&self) -> &'static str {
        "my_pre"
    }

    fn kind(&self) -> MiddlewareKind {
        MiddlewareKind::Pre
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        Ok(serde_json::Value::Null)
    }

    fn apply(&self, app: Router<Context>) -> gurthang::Result<Router<Context>> {
        Ok(app.layer(axum::middleware::from_fn(
            |request: Request, next: Next| async move {
                // run before the handler; return early to short-circuit
                next.run(request).await
            },
        )))
    }
}
```

Register it in `Hooks::middlewares`, which receives the default stack and lets
you insert, replace, or delete by name:

```rust
fn middlewares(ctx: &Context) -> MiddlewareStack {
    let mut stack = default_middleware_stack(ctx);
    stack
        .insert_before("authn", Box::new(MyPre))
        .replace("logger", Box::new(my_logger()));
    stack
}
```

## Authentication and authorization

`session_auth` is a `wrap` because it must load the session before the handler
and persist it afterwards. It is generic over the application's auth backend, so
the app installs it in `Hooks::middlewares`:

```rust
stack.replace(
    "session_auth",
    Box::new(session_auth::SessionAuthLayer::new(AuthBackend::new(ctx.db.clone()), ctx)),
);
stack.replace(
    "authn",
    Box::new(authn::RequireAuth::<AuthBackend>::new(authn::Config {
        enable: true,
        public_paths: vec!["/".into(), "/login".into(), "/register".into(), "/assets".into()],
        redirect: Some(crate::routes::auth::LOGIN.path.to_string()),
        ..Default::default()
    })),
);
```

`RequireAuth` rejects unauthenticated requests with a redirect (or a status when
`redirect` is unset). Authorization is opt-in: implement `authz::Authorizer` and
install `authz::RequireAuthz` to reject authenticated-but-forbidden requests.
