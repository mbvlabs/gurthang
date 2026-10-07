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
- Presence in the stack means the layer runs. YAML does not turn layers on or
  off and does not supply middleware values.

## Default stack

`default_middleware_stack` is constructed in Rust from the environment
(`session.secure` still comes from session config, for the CSRF cookie flag).
`cors`, `compression`, and `secure_headers` are absent until an app `push`es
them. `gurthang middleware` prints this default stack (kind + name). Runtime
uses `Hooks::middlewares`, which may `replace` placeholders.

| Kind   | Name              | Default | Notes |
|--------|-------------------|---------|-------|
| `wrap` | `catch_panic`     | on      | catches panics from the rest of the pipeline |
| `wrap` | `session_auth`    | placeholder | session load/persist; the app swaps in `SessionAuthLayer<B>` |
| `wrap` | `csrf`            | on      | XSRF check + cookie issuance |
| `wrap` | `request_id`      | on      | ensures and propagates `x-request-id` |
| `wrap` | `telemetry`       | on      | tracing span over the rest of the pipeline |
| `wrap` | `timeout_request` | on      | 30s default |
| `wrap` | `limit_payload`   | on      | 2 MB default body limit |
| `wrap` | `static`          | on      | development asset mount |
| `wrap` | `fallback`        | non-prod | omitted in production |
| `post` | `etag`            | on      | |
| `post` | `powered_by`      | on      | |
| `post` | `metrics`         | on      | `MetricsRecorder` trait, `TracingMetrics` default |
| `post` | `logger`          | on      | method, path, status, duration, request id |
| `pre`  | `remote_ip`       | on      | reads `x-forwarded-for` |
| `pre`  | `rate_limit`      | non-test | omitted in test; 100 requests / 1s per IP |
| `pre`  | `authn`           | placeholder | the app swaps in `RequireAuth<B>` |
| `pre`  | `authz`           | placeholder | the app swaps in `RequireAuthz` plus an `Authorizer` |

Telemetry and timeout are on by default. Rate limiting is on outside test.
Authorization is a no-op until an app installs an authorizer.

`gurthang middleware --routes` adds a `scope` column. Today that prints
`global` for the Rust default stack. Group scopes show up when an app uses
`RouteGroup::add_mw`; wiring the command to `Hooks::middlewares` is a later
follow-up.

## Group attach

Catalog `Route` stays `{ name, path }`. Bind a verb and handler to get a
`BoundRoute`, collect those in a `RouteGroup`, then `add_mw` once. That wraps
the group's sub-router (T0); it is not a per-route layer. Welcome/auth keep
`mount!` when they have no middleware.

```rust
pub fn routes(ctx: &Context) -> Router<Context> {
    let dashboard = Dashboard { inertia: ctx.inertia.clone() };
    RouteGroup::new()
        .add(crate::routes::dashboard::DASHBOARD.get(dashboard, Dashboard::show))
        .add_mw(authn::RequireAuth::<AuthBackend>::new(auth::LOGIN.path))
        .into_router()
}
```

Two groups in one controller: two `RouteGroup`s, then `Router::merge`.
`add_mw` comes from the prelude (`RouteGroupExt` in `gurthang`; it needs
`Context`). `wrap_router` is the helper underneath. Global
`Hooks::middlewares` still `replace`s `session_auth`; `authn` stays a
placeholder on the global stack.

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
```

`RequireAuth` is “must be logged in”: wrap the routes that need a user.
Inertia and HTML requests redirect to the login path you pass in; other
clients get 401. There is no public-path skip list — unauthenticated
routes simply are not wrapped.

Authorization is opt-in: implement `authz::Authorizer` and `replace` the
placeholder with `authz::RequireAuthz` to reject authenticated-but-forbidden
requests.
