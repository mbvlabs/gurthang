# RequireAuth = must be logged in

Yes — expose a **Route wiring primitive** from `gurthang-http`. The catalog
constant stays `{ name, path }` (that’s what `sync routes` greps and emits
to TypeScript). Binding a verb + handler produces a different type that
can take middleware.

## Two types

```rust
// catalog — already exists, unchanged
pub struct Route { name: &'static str, path: &'static str }
pub const DASHBOARD: Route = Route { name: "dashboard", path: "/dashboard" };

// framework primitive (new)
DASHBOARD.get(ctrl, Dashboard::show)  // -> BoundRoute  (also post/put/patch/delete)
RouteGroup::new().add(bound).add_mw(layer).into_router()
```

`add_mw` lives on **`RouteGroup`** (and a group of one). That wraps a
sub-router **once** (T0). Not `DASHBOARD.add_mw(...)` on the catalog
constant — there is still no handler there.

## High-level

```rust
pub fn routes(ctx: &Context) -> Router<Context> {
    let dashboard = Dashboard { inertia: ctx.inertia.clone() };
    RouteGroup::new()
        .add(crate::routes::dashboard::DASHBOARD.get(dashboard, Dashboard::show))
        // .add(OTHER.get(dashboard, Dashboard::other))
        .add_mw(authn::RequireAuth::<AuthBackend>::new(auth::LOGIN.path))
        .into_router()
}
```

Two groups in one controller: two `RouteGroup`s, `Router::merge`.
Welcome/auth keep `mount!` (no mw). `mount!` stays; this is additive.

## RequireAuth

```rust
RequireAuth::new(login_path)
// logged in → handler
// Inertia / HTML → 303 login
// else → 401
```

No runtime `public_paths` / `enable` / `status`. Global stack still only
`replace`s `session_auth`.

## Tests

Grouped `/dashboard`: HTML → 303 `/login`, JSON → 401. Unwrapped `/login` → 200.

Out of this: generator scaffold, `add_mw` on the catalog `Route`, per-route
layer types, moving session off `Hooks::middlewares`.
