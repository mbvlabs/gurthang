# Rwf vs Gurthang: Feature Comparison

A feature-by-feature comparison of **Rwf** (Rust Web Framework,
[github.com/levkk/rwf](https://github.com/levkk/rwf)) against **Gurthang**. Both
are PostgreSQL-backed, MVC-shaped Rust web frameworks, so this is the closest
apples-to-apples comparison of the three (the others being Laravel and Loco).
Rwf is the "own everything, minimal dependencies" take; Gurthang is the
"compile-checked, schema-first, agent-first" take.

> Legend: **Full** = first-class equivalent, **Partial** = exists but narrower or
> different shape, **Absent** = no equivalent today, **Diverges** = solved a
> different way on purpose, **Gurthang+** = Gurthang is stronger.

---

## 0. Executive summary

Rwf and Gurthang agree on the big shape: MVC, PostgreSQL, a Postgres-backed job
queue using `FOR UPDATE SKIP LOCKED`, Argon2 password hashing, and CSRF by
default. They diverge sharply on three axes:

1. **Data layer.** Rwf ships its own ORM and its own SQL builder (a
   `Model` trait with scopes, joins, and `find_by_sql`). Gurthang ships **no
   ORM**: it uses SQLx `query!`/`query_as!` with compile-checked, offline SQL.
2. **View layer.** Rwf ships its own template engine plus deep **Hotwired
   Turbo** integration (server-rendered, server-driven SPA). Gurthang ships
   **Inertia React + Vite + Tailwind** with typed page contracts.
3. **Code generation.** Rwf has a thin `rwf-cli` (`setup`, `migrate`, `add/remove
   controller`, `package`). Gurthang has a large generator surface
   (`generate model|controller|scaffold|job|mailer|task`, `sync …`) that reads
   the **live database schema** and keeps frontend contracts in sync.

Rwf brings several things Gurthang lacks: a **REST framework with automatic
CRUD + pagination**, **WebSockets**, **cron/scheduled jobs**, **OIDC/basic
auth**, an **admin panel**, **encrypted cookie sessions**, and a
**package/bundle** command. Gurthang brings what Rwf lacks: **schema-first
generation**, **typed end-to-end contracts**, **Inertia SSR**, **offline
compile-checked SQL**, **drift detection (`sync --check`)**, and a
**machine-readable CLI** posture.

---

## 1. Positioning

| | Rwf | Gurthang |
|---|---|---|
| Tagline | Comprehensive MVC framework, few dependencies | Project pad for Rails-shaped Rust apps |
| Philosophy | Own the stack (own ORM, own templates) | Assemble best-in-class crates; compile-check everything |
| HTTP | Tokio + own server (HTTP/1.1 only) | Axum + Tower (HTTP/1.1; Tower HTTP/2 capable) |
| Database | PostgreSQL only (`tokio_postgres`) | PostgreSQL only (SQLx) |
| Frontend | Server-rendered templates + Hotwired Turbo | Inertia React + Vite + Tailwind |
| Maturity | Beta ("early adopters") | Proof of concept |

Both are **PostgreSQL-only** and both are honest about being early. Gurthang is
more opinionated about the *developer workflow*; Rwf is more opinionated about
the *runtime*.

---

## 2. CLI

| Capability | Rwf (`rwf-cli`) | Gurthang | Status |
|---|---|---|---|
| Scaffold project | `rwf-cli setup` (adds dirs, `cargo add rwf`) | `gurthang new <name>` (full app template) | Gurthang+ |
| Run dev server | `cargo run` (manual) | `gurthang run` (Vite + backend, live reload) | Gurthang+ |
| Add controller | `rwf-cli add controller --name` `[--page]` | `gurthang generate controller <Name> [actions]` | Gurthang+ |
| Remove controller | `rwf-cli remove controller --name` | none | Rwf |
| Model/migration generator | none (hand-written) | `generate model` / `generate migration` | Gurthang+ |
| Scaffold/resource | none | `gurthang generate scaffold <Name>` | Gurthang+ |
| Job/mailer/task generators | none | `generate job` / `mailer` / `task` | Gurthang+ |
| Migrations | `rwf-cli migrate add/run/revert/flush` | `gurthang db migrate up/status` (via sqlx-cli) | Partial |
| DB lifecycle | none (migrate only) | `db create/drop/nuke/rebuild/seed` | Gurthang+ |
| Routes listing | none | `gurthang routes` | Gurthang+ |
| Middleware listing | none | `gurthang middleware` | Gurthang+ |
| Package/deploy | `rwf-cli package` (bundle tar.gz) | `gurthang build` (release binary) | Partial |
| Toolchain check | none | `gurthang tools [check\|sync]` | Gurthang+ |
| Contract sync | none | `gurthang sync routes\|payloads\|model` `[--check]` | Gurthang+ |

Rwf's `package` command (bundling binary + templates + static + migrations into
`bundle.tar.gz`) is a genuinely useful idea Gurthang lacks; `gurthang build`
produces a binary but no bundle.

---

## 3. Routing & controllers

| Capability | Rwf | Gurthang | Status |
|---|---|---|---|
| Controller definition | `#[controller]` macro on `async fn -> Response` | struct + `impl` methods + `on(ctrl, method)` | Diverges |
| Route registration | `route!("/path" => ctrl)` into a `Vec` | `mount!` + `Route` consts | Diverges |
| Named routes | none (string paths) | `Route { name, path }` + `.url()`/`.url_with(id)` | Gurthang+ |
| Route params | string paths | `{id}` + `url_with` | Partial |
| Typed request parsing | `request.form::<T>()` / `json::<T>()` (typed via `macros::Form`) | axum extractors + `serde_urlencoded`/JSON in controllers | Partial |
| Response helpers | `.html/.json/.text/.redirect/.unauthorized/.not_found` | axum `Response` + Inertia renderer | Diverges |
| Custom error pages | `NotFound` wildcard (codes on roadmap) | `fallback` middleware (embedded 404) | Partial |
| Route groups / prefixes | `Controller::route` composition | router `merge` + middleware (T1) | Partial |
| REST/CRUD controllers | `RestController` / `ModelController` (`rest!`, `crud!`) + pagination | `generate scaffold` emits 7 REST routes | Rwf |
| Content negotiation | REST framework, JSON | Inertia XHR vs full HTML | Diverges |

Rwf's **`ModelController`/`crud!`** (auto CRUD + pagination at 25/page) is the
single biggest capability Gurthang lacks here — it maps directly to Gurthang
roadmap **T12 (Pagination + collections)** and would be cheap to add on top of
the existing `generate scaffold`.

---

## 4. Middleware

| Capability | Rwf | Gurthang | Status |
|---|---|---|---|
| Middleware trait | `Middleware::handle_request -> Outcome::{Forward,Stop}` | `MiddlewareLayer` (name/enabled/config/apply) | Full |
| Attach point | per-controller (`middleware()`) + global defaults | global stack; per-route in **T1** | Partial |
| Config-driven | built-in global (tracker + CSRF) | YAML `server.middlewares` | Gurthang+ |
| Reorder/replace by name | no | `MiddlewareStackExt` (T1) | Gurthang+ |
| Defaults | request tracker + CSRF | 15-layer default stack (T1: pre/post/wrap kinds) | Gurthang+ |
| Introspection | none | `gurthang middleware` | Gurthang+ |

Gurthang's middleware story is deeper and introspectable; Rwf's is minimal and
attached per controller. Gurthang's in-flight **T1** (Pavex-style pre/post/wrap
kinds + route/group registration) pushes this further.

---

## 5. Models / ORM / database

| Capability | Rwf | Gurthang | Status |
|---|---|---|---|
| ORM | own `Model` trait + SQL builder | **none** (SQLx `query!`/`query_as!`) | Diverges |
| Relationships | `#[belongs_to]` / `#[has_many]`, `join`/`join_left`/`join_nested` | none (explicit SQL) | Rwf |
| Scopes | `Scope<Model>` | plain `impl` methods | Partial |
| Query DSL | `filter/order/limit/group_by/select_aggregated`, operators | raw SQL in `models` crate | Diverges |
| Compile-checked SQL | no (runtime prepared statements) | **yes** (offline `.sqlx` metadata) | Gurthang+ |
| Migrations | paired up/down `.sql`, `migrate run/revert/flush` | SQLx migrator, `db migrate up` | Partial |
| Transactions | `Pool::transaction()` (auto-rollback) | SQLx transactions passed into models | Full |
| Connection pool | `Pool` (default 10, checkout timeout) | SQLx `PgPool` (`concurrency + 10`) | Full |
| Factories | none | generated typed factories | Gurthang+ |
| Query debugging | `to_sql()`, `.explain()` | SQLx + `EXPLAIN` by hand | Partial |
| Multiple DB engines | Postgres only | Postgres only | Equal |

This is the philosophical core. Rwf gives you an ActiveRecord-ish ORM with
relationships and a query DSL — fast to write, checked at runtime. Gurthang gives
you **no ORM** but compile-checked SQL — slower to write, caught at build time.
For the agent-optimized goal, Gurthang's choice is deliberate: an agent gets
compiler feedback instead of runtime 500s. Rwf's relationships/joins are the
feature Gurthang most conspicuously lacks.

---

## 6. Migrations

| Capability | Rwf | Gurthang | Status |
|---|---|---|---|
| Migration format | paired `<ts>_<name>.up.sql` / `.down.sql` | sequential `NNNN_slug.sql` (forward-only) | Rwf |
| Rollback | `migrate revert [--version]` | none (`db rebuild` only) | Rwf |
| Flush/rebuild | `migrate flush --yes` | `db rebuild` | Partial |
| Model generation from schema | none | `generate model` reads live DB | Gurthang+ |
| Offline query data | n/a | `cargo sqlx prepare` + `.sqlx` | Gurthang+ |

Rwf's **reversible migrations** (up/down) are a real gap in Gurthang, which is
forward-only.

---

## 7. Auth & sessions

| Capability | Rwf | Gurthang | Status |
|---|---|---|---|
| Session storage | encrypted cookie (AES-128), 4 weeks | Postgres `tower_sessions`, 7-day inactivity | Diverges |
| Guest vs authenticated session | `SessionId::{Guest,Authenticated}` | `axum-login` `AuthSession` | Full |
| Session auth guard | `SessionAuth::redirect("/login")` | `session_auth` middleware + `auth.user` | Full |
| Basic auth | `BasicAuth` | none | Rwf |
| OIDC | built-in config (`[oidc]`, `RWF_OIDC_*`) | none | Rwf |
| Password hashing | Argon2 (`crypto::hash`/`hash_validate`) | Argon2id in `spawn_blocking` | Full |
| Register/login/logout slice | manual | generated | Gurthang+ |
| Password reset / verify / magic link | none | none (T3) | Equal (both absent) |
| API tokens | none | none (T3) | Equal |
| Roles/permissions | none | none (T3) | Equal |

Both ship only a **basic** auth story (session login, hashing). Rwf adds Basic
auth and OIDC; Gurthang adds a generated, tested register/login slice with
CSRF-wired Inertia forms. Neither has password reset, email verification, or API
tokens — that is Gurthang roadmap **T3**.

---

## 8. Security (CSRF, encryption, hashing)

| Capability | Rwf | Gurthang | Status |
|---|---|---|---|
| CSRF | on by default; `X-CSRF-Token` / `rwf_csrf_token` field | on; `XSRF-TOKEN` cookie ↔ `x-xsrf-token` header | Full |
| CSRF exemptions | per-controller `skip_csrf`, `#[skip_csrf]` | per-stack (T1) | Partial |
| Cookie encryption | AES-128-GCM-SIV, private cookies | none (session payload MessagePack, not encrypted) | Rwf |
| Signed data | `crypto::encrypt/decrypt` | none (T13) | Rwf |
| Secret key management | `secret_key` (base64) / `RWF_SECRET_KEY` | config/env | Partial |

Rwf's **encrypted sessions and private cookies** are ahead of Gurthang here —
exactly what Gurthang roadmap **T13 (Cookie encryption + signed URLs)** targets.
Notably, Rwf encrypts the session cookie itself; Gurthang stores sessions
server-side in Postgres, so the risk profile differs (Gurthang's cookie is just
an opaque id), but Gurthang still lacks app-level encryption/signing primitives.

---

## 9. Views & frontend

| Capability | Rwf | Gurthang | Status |
|---|---|---|---|
| Templating | own engine (`<%= %>`, `<% %>`, partials, functions) | none (Inertia React) | Diverges |
| Server-driven SPA | Hotwired Turbo (+ Stimulus) | Inertia.js | Diverges |
| Turbo Streams over WebSockets | built-in `TurboStream` + `Comms` | none | Rwf |
| HMR | template hot reload via Turbo WS | Vite + React Fast Refresh | Full |
| SSR | always server-rendered | optional Node/Bun SSR, CSR fallback | Partial |
| Typed frontend contract | none | `ts-rs` props + `routes.ts` | Gurthang+ |
| Asset build | none (no bundler) | Vite 7 + Tailwind 4 | Diverges |
| Static files | `StaticFiles::serve("static")` | `static` middleware + `RustEmbed` | Full |

Rwf deliberately avoids a JS build step: templates + Turbo give an SPA feel
without a bundler. Gurthang goes all-in on React with a build pipeline and typed
contracts. These are opposite bets; Rwf is simpler to deploy, Gurthang gives
richer client state and compile-checked props.

---

## 10. Background jobs & cron

| Capability | Rwf | Gurthang | Status |
|---|---|---|---|
| Backing store | PostgreSQL (`rwf_jobs`) | PostgreSQL (`background_jobs`) | Full |
| Atomic claim | `FOR UPDATE SKIP LOCKED` | `FOR UPDATE SKIP LOCKED` | Full |
| Delivery semantics | at-least-once (idempotent handlers) | at-least-once (idempotent handlers) | Full |
| Poll interval | 1 second | configurable (default 1s) | Full |
| Wake-ups | polling | polling + `LISTEN`/`NOTIFY` | Gurthang+ |
| Leases / visibility timeout | restart reschedules | explicit expiring leases + reaper | Gurthang+ |
| Retries / backoff | not described | bounded retries + exponential backoff | Gurthang+ |
| Failed-job handling | jobs table | `status='failed'`, queryable | Full |
| Cron / scheduled jobs | `schedule(cron)` + `Worker::clock` | none (T5) | Rwf |
| Concurrency | one job per worker; spawn more | `WorkerConfig` concurrency via `JoinSet` | Gurthang+ |
| Transactional enqueue | — | `enqueue_in(&mut PgConnection, …)` | Gurthang+ |

Both use the same Postgres-queue pattern. Gurthang's queue is more
feature-complete (leases, backoff, NOTIFY, transactional enqueue), but Rwf ships
the **cron/scheduler** Gurthang still lacks (Gurthang roadmap **T5**).

---

## 11. REST & realtime

| Capability | Rwf | Gurthang | Status |
|---|---|---|---|
| REST framework | `RestController` (6 verbs, 405 fallback) | none (T12 could add) | Rwf |
| Auto CRUD | `ModelController` (`crud!`) | `generate scaffold` | Rwf |
| Pagination | built-in (25/page, `?page`, `?page_size`) | none (T12) | Rwf |
| JSON serialization | serde_json | serde/Inertia | Full |
| WebSockets | built-in `WebsocketController` + `Comms` push | none | Rwf |
| Server push from jobs | `Comms::websocket(...).send(...)` | none | Rwf |

Rwf is meaningfully ahead on realtime and REST scaffolding. Gurthang's plan
covers pagination (T12) but has no WebSocket story — worth adding to **T0**
(design study) if realtime matters to the target apps.

---

## 12. Configuration

| Capability | Rwf | Gurthang | Status |
|---|---|---|---|
| Config format | `rwf.toml` + env vars | YAML per environment | Diverges |
| Environment-specific | env overrides | `config/<env>.yaml` | Full |
| Secrets | `secret_key`, `RWF_*` env | `.env` (`DATABASE_URL`) + YAML | Partial |
| Middleware config | minimal | YAML `server.middlewares` | Gurthang+ |
| Pool/session tuning | `[database]`, `[general]` | `config.server/workers/session` | Full |

---

## 13. Observability

| Capability | Rwf | Gurthang | Status |
|---|---|---|---|
| Logging | `tracing` (`Logger::init`, `RUST_LOG`) | `tracing` (`RUST_LOG`, config level) | Full |
| Query logging | `log_queries` toggle | SQLx logs | Full |
| Request analytics | `rwf_requests` table (`track_requests`) | `request_id` + logger middleware | Rwf |
| Admin/dashboard | `rwf-admin` (activity, jobs, models) | none (T15) | Rwf |
| Metrics | request analytics only (WIP) | `MetricsRecorder` trait (T1) | Partial |

Rwf's **admin panel** and request-analytics table are ahead of Gurthang's current
observability, which is the focus of Gurthang roadmap **T15**.

---

## 14. Testing

| Capability | Rwf | Gurthang | Status |
|---|---|---|---|
| Test framework | Rust `#[test]`/tokio | `cargo test --workspace` | Full |
| Test harness helpers | none documented | none generated (T17) | Equal |
| Fixtures/seed | none | `db seed` + factories | Gurthang+ |
| Fakes (mail/queue) | none | none (T17) | Equal |

---

## 15. Scorecard (rough)

| Area | Rwf | Gurthang | Notes |
|---|---|---|---|
| CLI workflow | ★★★☆☆ | ★★★★★ | Gurthang's generator/sync surface is far larger |
| Routing/controllers | ★★★★☆ | ★★★★☆ | Rwf has REST+CRUD; Gurthang has named routes |
| Middleware | ★★★☆☆ | ★★★★☆ | Gurthang deeper + introspectable |
| ORM/models | ★★★★☆ | ★★★☆☆ | Rwf has relationships; Gurthang has compile-checked SQL |
| Migrations | ★★★★☆ | ★★★☆☆ | Rwf reversible; Gurthang forward-only |
| Auth | ★★★☆☆ | ★★★☆☆ | Rwf has OIDC/basic; Gurthang has generated slice |
| Security | ★★★★☆ | ★★★☆☆ | Rwf encrypts sessions; Gurthang lacks (T13) |
| Views/frontend | ★★★★☆ | ★★★★☆ | Opposite bets; both coherent |
| Queues | ★★★★☆ | ★★★★★ | Gurthang leases/backoff/NOTIFY; Rwf has cron |
| REST/realtime | ★★★★★ | ★★☆☆☆ | Rwf REST+CRUD+pagination+WebSockets |
| Config | ★★★★☆ | ★★★★☆ | Comparable |
| Observability | ★★★★☆ | ★★☆☆☆ | Rwf admin panel + analytics |
| **Agent-friendliness** | ★★☆☆☆ | ★★★★★ | Typed, schema-first, drift-checked |

---

## 16. Design lessons for Gurthang (feeds T0)

Rwf is a useful prior-art source for the **T0 framework design study**:

1. **`ModelController` / `crud!` + pagination.** Rwf shows how compact an
   auto-CRUD controller can be. Steal the shape for **T12**: a paginated REST
   controller layered on the existing `generate scaffold`, with `?page` /
   `?page_size` and a `Page<T>` envelope.
2. **Reversible migrations.** Rwf's paired up/down files and `migrate revert`
   are worth adopting (Gurthang is forward-only today).
3. **Encrypted sessions / private cookies.** Rwf's `crypto::encrypt/decrypt` and
   `cookies().add_private(...)` validate the **T13** direction.
4. **Cron built into the worker.** Rwf's `Worker::clock(schedule)` is a clean
   model for **T5 (scheduler)**.
5. **Admin panel.** `rwf-admin` is a concrete target for **T15**.
6. **`package`/bundle command.** Bundling binary + assets + migrations is a
   deployment nicety worth adding to `gurthang build`.
7. **WebSockets + server push.** Rwf's `Comms` push from controllers *and jobs*
   is a capability Gurthang has no answer for; decide in T0 whether realtime is
   in scope.

**Avoid:** Rwf's runtime-checked SQL (Gurthang's compile-checked SQLx is the
whole point) and its HTTP/1.1-only server (stay on Axum/Tower).

---

## 17. Bottom line

Rwf and Gurthang are **sibling Postgres-only MVC frameworks with opposite
centers of gravity**. Rwf owns its runtime (ORM, templates, Turbo, REST,
WebSockets, admin) and optimizes for a self-contained, minimal-dependency app
with no JS build step. Gurthang owns its *workflow* (schema-first generators,
typed contracts, drift checks) and optimizes for a compile-checked, agent-driven
app with a React frontend.

The most valuable things to take from Rwf are **REST/CRUD + pagination**,
**reversible migrations**, **cron in the worker**, **encrypted sessions**, and
**an admin/observability panel** — all of which already appear on Gurthang's
roadmap (T12, migrations, T5, T13, T15). The thing *not* to take is Rwf's
runtime-checked data layer; Gurthang's compile-checked SQL is the reason an agent
can trust its own edits.
