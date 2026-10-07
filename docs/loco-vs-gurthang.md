# Loco vs Gurthang: Feature Comparison

A feature-by-feature comparison of **Loco** ([loco.rs](https://loco.rs),
"Rust on Rails") against **Gurthang**. This comparison matters more than the
others because **Gurthang was deliberately restructured around Loco's layout**
(commit `f76a7b1 chore: restructure around loco-rs layout`): both use an
Axum-based `Hooks` app-definition trait, per-environment YAML config, a
`generate`/`cargo loco generate` CLI, background workers, and queued mailers.
Gurthang is best read as "a Loco-shaped app, minus SeaORM/Tera, plus
compile-checked SQL and an Inertia/agent-first frontend."

> Legend: **Full** = first-class equivalent, **Partial** = exists but narrower or
> different shape, **Absent** = no equivalent today, **Diverges** = solved a
> different way on purpose, **Gurthang+** = Gurthang is stronger.

---

## 0. Executive summary

Loco and Gurthang share an architectural skeleton:

- Both are **Axum-based** and expose a **`Hooks` trait** (`app_name`, `boot`,
  `routes`, `connect_workers`, `register_tasks`, `initializers`, `middlewares`,
  `before_routes`/`after_routes`, `on_shutdown`).
- Both generate an app with `config/<env>.yaml`, a `src/` MVC-ish tree, workers,
  mailers, tasks, and initializers.
- Both generate code from the CLI and run background jobs out of a queue.

They diverge on the two biggest layers:

1. **Data:** Loco wraps **SeaORM** (ActiveRecord, relationships, migrations DSL,
   multi-DB). Gurthang uses **SQLx** with compile-checked, offline SQL and
   **Postgres only**.
2. **Frontend:** Loco wraps **Tera** (SSR) or a REST + Vite/React/TanStack SPA.
   Gurthang is **Inertia React** with `ts-rs`-typed page contracts.

Loco is dramatically more feature-complete: **full auth (JWT, reset, verify,
magic link, API tokens), scheduler, storage (OpenDAL), cache, testing harness,
pagination/query DSL, jobs-management CLI, multi-tenancy, and deployment
generators.** Gurthang's roadmap (T3, T5, T10, T12, T15, T17) is, in large part,
"add the Loco batteries."

What Gurthang has that Loco does not: **schema-first generation** (reads the live
DB rather than a stub), **compile-checked SQL**, **Inertia + typed page
contracts**, **drift detection (`sync --check`)**, **non-destructive generation**,
and a **machine-readable CLI posture** aimed at agents.

---

## 1. Positioning

| | Loco | Gurthang |
|---|---|---|
| Tagline | "Rust on Rails" / "one-person framework" | Project pad for Rails-shaped Rust apps |
| Maturity | 1.0 (July 2026), active | Proof of concept |
| HTTP | Axum 0.8 | Axum 0.8 |
| ORM | SeaORM 2.0 (ActiveRecord) | none (SQLx) |
| Templates | Tera | Inertia React |
| Database | SQLite, Postgres (MySQL via SeaORM) | PostgreSQL only |
| Edition | 2024 | 2024 |
| Optimized for | Solo developer productivity | Agent-driven, compile-checked development |

Gurthang copied Loco's *shape* but replaced its *substance* (SeaORM → SQLx,
Tera → Inertia) and narrowed scope (Postgres only) to buy compile-time
correctness.

---

## 2. CLI

| Capability | Loco | Gurthang | Status |
|---|---|---|---|
| New app | `loco new` (interactive; SSR/SPA/API/lightweight) | `gurthang new <name>` (one shape) | Loco |
| Run server | `cargo loco start` `[--worker\|--scheduler\|--all]` | `gurthang run` (Vite + backend) | Full |
| Watch | `cargo loco watch` | built into `gurthang run` | Full |
| Generate model | `cargo loco generate model` (applies migration) | `gurthang generate model` (reads live DB) | Diverges |
| Generate migration | `cargo loco generate migration` | `gurthang generate migration` | Full |
| Scaffold | `cargo loco generate scaffold` (auth by default) | `gurthang generate scaffold` | Full |
| Generate controller | `cargo loco generate controller` `[--auth]` | `gurthang generate controller <Name> [actions]` | Full |
| Generate task | `cargo loco generate task` | `gurthang generate task` | Full |
| Generate worker/job | `cargo loco generate worker` | `gurthang generate job` | Full |
| Generate mailer | `cargo loco generate mailer` | `gurthang generate mailer` | Full |
| Generate scheduler | `cargo loco generate scheduler` | none (T5) | Loco |
| Generate data/fixtures | `cargo loco generate data` | none (T17) | Loco |
| Generate deployment | `cargo loco generate deployment docker\|nginx\|lambda` | none | Loco |
| Field mini-language | `name:type` with `!`/`^`/`references` | schema introspection (no mini-language) | Diverges |
| DB lifecycle | `cargo loco db create/migrate/down/reset/status/truncate/seed/schema` | `gurthang db create/drop/nuke/rebuild/migrate/seed` | Full |
| DB entities (debug) | `cargo loco db entities` | n/a (models are the entities) | Diverges |
| Routes | `cargo loco routes` (tree) | `gurthang routes` (table) | Full |
| Middleware | `cargo loco middleware` | `gurthang middleware` | Full |
| Tasks | `cargo loco task [name]` | `gurthang task [name]` | Full |
| Jobs management | `cargo loco jobs cancel/tidy/purge/dump/import/requeue/retry` | none (T8/T15) | Loco |
| Scheduler run | `cargo loco scheduler [--list]` | none (T5) | Loco |
| Doctor/diagnose | `cargo loco doctor` | `gurthang tools check` | Partial |
| Version | `cargo loco version` | — | Partial |
| Contract sync | none | `gurthang sync routes\|payloads\|model` `[--check]` | Gurthang+ |
| Toolchain install | none | `gurthang tools sync` | Gurthang+ |
| Build | `cargo build` | `gurthang build` (Vite + sqlx prepare + release) | Gurthang+ |

Loco's CLI is broader (jobs management, scheduler, deployment, fixtures,
doctor). Gurthang's is narrower but adds **drift-checked regeneration** and
**toolchain provisioning** — the agent-oriented pieces.

---

## 3. Hooks & app lifecycle

Both expose a `Hooks` trait; Gurthang's is nearly a subset of Loco's.

| Hook | Loco | Gurthang |
|---|---|---|
| `app_name` / `app_version` | yes | `app_name` (version via `CARGO_PKG_VERSION`) |
| `boot(mode, env, config)` | yes | `boot(mode, config)` |
| `routes(ctx)` | yes (`AppRoutes`) | yes (`AppRoutes`) |
| `connect_workers(ctx, queue)` | yes | `connect_workers(ctx)` |
| `register_tasks(&mut Tasks)` | yes | yes |
| `initializers` | yes | yes |
| `middlewares` | yes | yes (`default_middleware_stack`) |
| `before_routes` / `after_routes` | yes | yes |
| `after_context` (mutate `AppContext`) | yes | `Initializer::before_run(&mut Context)` |
| `on_shutdown` | yes | yes |
| `truncate` / `seed` (DB) | yes | no (via `db seed` bin) |
| `dump` | yes | no |
| `serve` / `init_logger` / `load_config` | overridable | fixed |

**Gurthang's `Hooks` is essentially Loco's**, which is exactly the review note
("This was taken from Loco RS — I'm not entirely sure I love the hooks design").
Gurthang roadmap **T14** (Hooks rework) and **T0** (framework design study) are
where this gets revisited. A key difference: Loco's `after_context` can replace
`AppContext` fields via a builder (`AppContext` is `#[non_exhaustive]`), whereas
Gurthang mutates `Context` directly in `Initializer::before_run`.

---

## 4. Models / ORM / database

| Capability | Loco | Gurthang | Status |
|---|---|---|---|
| ORM | SeaORM 2.0 (ActiveRecord, "fat models") | **none** (SQLx `query!`/`query_as!`) | Diverges |
| Relationships | SeaORM relations + `references` in generator | none (explicit SQL) | Loco |
| Migrations DSL | `loco_rs::schema::*` over sea_query | raw `.sql` files | Diverges |
| Migration application | generator applies immediately (`SKIP_MIGRATION` to defer) | you migrate, then generate | Diverges |
| Primary keys | i64 auto-increment default; UUID option | schema-driven (UUID in template) | Diverges |
| Query DSL | `query::condition()` (~18 operators) | raw SQL | Diverges |
| Pagination | `PaginationQuery` + `PageResponse` + `PagerMeta` | none (T12) | Loco |
| Compile-checked SQL | no | **yes** (offline `.sqlx`) | Gurthang+ |
| Multi-DB | SQLite, Postgres (MySQL via SeaORM) | Postgres only | Loco |
| Multi-tenancy | opt-in feature (`TenantEntity`, `in_tenant`) | none | Loco |
| Factories/fixtures | `generate data` + fixtures + `seed` | generated factories + `db seed` | Full |
| Model errors | `ModelError`/`ModelResult` | `sqlx::Result` + domain errors | Partial |

This is the defining trade. Loco's SeaORM gives relationships, a query DSL, and
pagination with almost no SQL; Gurthang gives **no ORM** but compile-checked SQL.
For agents, Gurthang's bet is that **the compiler is a better reviewer than the
developer** — a wrong column is a build error, not a 500. Loco's pagination and
relationships are the capabilities Gurthang most needs (T12, and a models
rework in T2).

---

## 5. Controllers & routing

| Capability | Loco | Gurthang | Status |
|---|---|---|---|
| Controller | plain `async fn -> Result<Response>`, no macro | struct + `impl` methods + `on(ctrl, method)` | Diverges |
| State | `AppContext` via `State` | `Context` via `State` + `FromRef` | Full |
| Route builder | `Routes`/`AppRoutes` (`.prefix/.add/.nest/.merge/.layer`) | `AppRoutes` + `mount!` | Partial |
| Named routes | no | `Route { name, path }` + `.url()`/`.url_with(id)` | Gurthang+ |
| Route tree | `cargo loco routes` | `gurthang routes` | Full |
| Default routes | `/_ping`, `/_health`, `/_readiness` | none (T15) | Loco |
| Scaffold auth | scaffold authenticated by default | scaffold unauthenticated | Loco |
| Error mapping | `Error` enum + `ErrorDetail` JSON | per-app `AppError` | Partial |
| Response helpers | `format::json/text/html/yaml/render/redirect` | axum `Response` + Inertia | Diverges |

Gurthang's named routes + `mount!` are a nice, greppable alternative to Loco's
builder. Loco's built-in health/readiness routes and its authenticated-by-default
scaffold are worth copying (T15, T3).

---

## 6. Views & frontend

| Capability | Loco | Gurthang | Status |
|---|---|---|---|
| SSR engine | Tera (`assets/views/`) | Inertia React (Node/Bun SSR optional) | Diverges |
| View abstraction | `ViewRenderer` trait, `ViewEngine<TeraView>` | `InertiaPage`/props + `InertiaRenderer` | Diverges |
| SPA option | Vite + React + TanStack Query | Inertia React (default) | Partial |
| Typed frontend contract | `ts-rs` DTOs (SPA mode) | `ts-rs` page props + `routes.ts` | Full |
| Embedded assets | `embedded_assets` feature | `RustEmbed` + embedded SSR bundle | Full |
| Vite/Tailwind | yes (SPA mode) | yes (default) | Full |
| CSS | Tailwind (SPA mode) | Tailwind 4 | Full |

Both generate TypeScript from Rust DTOs via `ts-rs` — a notable shared choice.
The difference is the rendering model: Loco defaults to Tera SSR (with a SPA
option), Gurthang defaults to Inertia (SPA-feel with optional SSR). Gurthang's
Inertia page contract (`InertiaPage` + shared props) is more prescriptive than
Loco's `ViewRenderer` trait.

---

## 7. Auth & authorization

| Capability | Loco | Gurthang | Status |
|---|---|---|---|
| Auth suite | register/login/current/verify/forgot/reset/magic-link/resend | register/login/logout | Loco |
| Token model | JWT (`auth::JWT`, `auth::JWTWithUser`) | session (`axum-login`) | Diverges |
| API tokens | `ApiToken<T>` | none (T3) | Loco |
| Email verification | built-in endpoint + rule | none (T3) | Loco |
| Password reset | built-in | none (T3) | Loco |
| Magic link | built-in | none | Loco |
| Session vs JWT location | JWT in Bearer/Query/Cookie (configurable) | HttpOnly session cookie + CSRF | Diverges |
| Password hashing | Argon2id (`loco_rs::hash`) | Argon2id (`spawn_blocking`) | Full |
| CSRF | not central (JWT-based) | XSRF double-submit, on by default | Gurthang+ |
| Authorization (roles/policies) | not first-class | none (T3) | Equal |

Loco's auth is far ahead: it ships the whole suite (reset, verify, magic link,
API tokens) out of the box. Gurthang has a generated login/register/logout slice
with server-side sessions and CSRF, but none of the rest — which is exactly
Gurthang roadmap **T3 (Auth + authorization completeness)**. Note the trade:
Loco leans JWT (stateless), Gurthang leans server-side sessions + CSRF.

---

## 8. Background workers & scheduler

| Capability | Loco | Gurthang | Status |
|---|---|---|---|
| Worker trait | `BackgroundWorker<A>` (`perform`, `perform_later`) | `PerformJob` (`perform`) | Full |
| Modes | `BackgroundQueue` (durable), `BackgroundAsync` (in-process), `ForegroundBlocking` | worker process / in-process (`StartMode`) | Full |
| Queue backends | Redis, Postgres, SQLite (`queue.kind`) | Postgres only | Loco |
| Postgres queue | `pg_loco_queue`, `FOR UPDATE SKIP LOCKED` | `background_jobs`, `FOR UPDATE SKIP LOCKED` | Full |
| Wake-ups | polling | polling + `LISTEN`/`NOTIFY` | Gurthang+ |
| Retries/backoff | manual (`jobs retry`); no auto backoff | bounded retries + exponential backoff | Gurthang+ |
| Leases / reaper | `reaper` (opt-in) | expiring leases + reaper built in | Gurthang+ |
| Priority | full i32 | ordering by `available_at` | Partial |
| Batches | `perform_all_later` (atomic) | none | Loco |
| Job management CLI | `cargo loco jobs …` | none (T8/T15) | Loco |
| Scheduler | `config/scheduler.yaml`, English + cron | none (T5) | Loco |
| Tags/queues | `queue()`, tags, named queues | single queue | Loco |

Both run durable Postgres queues on `FOR UPDATE SKIP LOCKED`. Gurthang's queue is
more robust (backoff, leases, NOTIFY, transactional enqueue) but Loco has
**scheduler, batches, priority, tags, and a jobs-management CLI** — all of which
Gurthang's roadmap (T5, T8, T15) wants. Interesting inversion: **Loco has no
automatic retry/backoff**, while Gurthang does.

---

## 9. Mail, storage, cache

| Capability | Loco | Gurthang | Status |
|---|---|---|---|
| Mailer trait | `Mailer` (`opts`, templates) | `Mailer` (`opts`, `mail`/`deliver_now`) | Full |
| Templates | Tera `subject.t`/`html.t`/`text.t` | Askama `html`/`text` | Full |
| Queued delivery | `MailerWorker` on `"mailer"` queue | `MailerEnvelope` job | Full |
| SMTP | `mailer.smtp` (starttls/implicit/none) | lettre (starttls/implicit/none) | Full |
| Stub (test) | `mailer.stub: true` | stub transport | Full |
| Storage | OpenDAL (local/mem/null/S3/Azure/GCP) + strategies | none (T11, parked) | Loco |
| Cache | `InMem`/`Redis`/`Null` (`cache.kind`) | none (T10) | Loco |

Mail parity is close. **Storage and cache are entirely missing in Gurthang**
(T11 parked; T10 planned). Loco's OpenDAL abstraction and pluggable cache are the
reference designs.

---

## 10. Configuration

| Capability | Loco | Gurthang | Status |
|---|---|---|---|
| Per-env config | `config/{env}.yaml` + `{env}.local.yaml` | `config/{development,test,production}.yaml` | Full |
| Env resolution | `LOCO_ENV` → `RAILS_ENV` → `NODE_ENV` → dev | `APP_ENV` | Partial |
| Templated YAML | Tera in YAML (`<%= get_env(...) %>`) | plain YAML + `.env` | Loco |
| Secrets | `get_env` template fn | `.env` (`DATABASE_URL`) | Partial |
| Sections | logger/server/database/cache/queue/auth/workers/mailer/scheduler | server/session/inertia/workers/logger/mailer/database | Full |
| Middleware config | `server.middlewares` | `server.middlewares` (15-layer stack) | Full |

Gurthang copied Loco's per-env YAML config. Loco adds **Tera-templated YAML**
for secret injection — a small but useful ergonomic Gurthang lacks.

---

## 11. Testing

| Capability | Loco | Gurthang | Status |
|---|---|---|---|
| Test harness | `testing` feature: `request::<App>()`, `boot_test` | none generated (T17) | Loco |
| DB lifecycle | `boot_test_with_create_db`, truncation strategies | `db rebuild` (manual) | Partial |
| Fixtures/seed | `seed::<App>()` + `src/fixtures` | factories + `db seed` | Partial |
| Snapshot testing | `insta` integration | none | Loco |
| Mail assertions | `deliveries()` | none (T17) | Loco |
| HTTP assertions | `axum-test` (`assert_status_ok`, `assert_json`, CSS asserts) | none | Loco |

Loco's testing story is a real differentiator and maps to Gurthang roadmap
**T17 (Testing scaffold + fakes)**.

---

## 12. Observability

| Capability | Loco | Gurthang | Status |
|---|---|---|---|
| Logging | `logger` (level/format/file rotation, pretty backtrace) | `tracing` + config level | Partial |
| Health/readiness | `/_ping`, `/_health`, `/_readiness` | none (T15) | Loco |
| Job inspector | `cargo loco jobs` | none (T15) | Loco |
| Metrics | — | `MetricsRecorder` trait (T1) | Gurthang+ |
| Doctor | `cargo loco doctor` | `gurthang tools check` | Partial |

Loco's health endpoints and jobs CLI are the practical pieces Gurthang's **T15**
should adopt; Gurthang's `MetricsRecorder` trait is a head start Loco lacks.

---

## 13. Scorecard (rough)

| Area | Loco | Gurthang | Notes |
|---|---|---|---|
| CLI breadth | ★★★★★ | ★★★★☆ | Loco has jobs/scheduler/deploy/fixtures |
| Hooks/lifecycle | ★★★★☆ | ★★★★☆ | Gurthang's is a Loco subset |
| ORM/models | ★★★★★ | ★★★☆☆ | SeaORM vs compile-checked SQLx |
| Migrations | ★★★★★ | ★★★☆☆ | SeaORM DSL vs raw SQL |
| Controllers/routing | ★★★★☆ | ★★★★☆ | Comparable |
| Auth | ★★★★★ | ★★☆☆☆ | Loco ships the full suite |
| Frontend | ★★★★☆ | ★★★★☆ | Tera/SPA vs Inertia; both ts-rs |
| Queues | ★★★★☆ | ★★★★★ | Gurthang backoff/leases/NOTIFY |
| Scheduler | ★★★★★ | ★☆☆☆☆ | Gurthang T5 |
| Storage/cache | ★★★★★ | ★☆☆☆☆ | Gurthang T10/T11 |
| Testing | ★★★★★ | ★★☆☆☆ | Gurthang T17 |
| Config | ★★★★★ | ★★★★☆ | Loco templated YAML |
| Observability | ★★★★☆ | ★★☆☆☆ | Gurthang T15 |
| **Agent-friendliness** | ★★★☆☆ | ★★★★★ | Schema-first, compile-checked, drift-checked |

---

## 14. Design lessons for Gurthang (feeds T0)

Loco is the richest prior-art source, and Gurthang already borrowed its shape.
For **T0 (framework design study)**:

1. **Keep the `Hooks` skeleton, but settle it.** Gurthang's `Hooks` *is* Loco's;
   decide in T0/T14 whether to keep, rename, or simplify it, and document the
   lifecycle. Loco's `after_context` builder pattern (replace context fields
   rather than mutate) is one concrete alternative to consider.
2. **Generator field mini-language.** Loco's `name:type` with `!`/`^`/`references`
   is ergonomic for hand-authored models. Gurthang's schema-introspection is
   better for accuracy; consider a hybrid (introspect, but allow overrides).
3. **Pagination + query DSL shape.** Loco's `PaginationQuery` / `PageResponse` /
   `PagerMeta` is a ready blueprint for **T12**.
4. **Full auth suite as the target.** Loco's `/api/auth/*` surface (reset, verify,
   magic link, API tokens) is the concrete scope for **T3**.
5. **Scheduler design.** `config/scheduler.yaml` with English phrases + cron is a
   proven design for **T5**.
6. **Storage/cache traits.** OpenDAL-backed `StoreDriver` and the
   `InMem`/`Redis`/`Null` cache are the reference for **T10/T11**.
7. **Testing harness.** `request::<App>()` / `boot_test` / fixtures / `insta` is
   the blueprint for **T17**.
8. **Health/readiness routes + jobs CLI.** Adopt for **T15**.
9. **`doctor` command.** Gurthang's `tools check` is close; consider merging the
   idea (connectivity + version diagnostics) into one command.

**Avoid:** SeaORM's runtime-checked queries (keep SQLx compile-checking), the
multi-DB abstraction (Postgres-only is a deliberate constraint), and Tera (keep
Inertia). Loco's **lack of automatic retry/backoff** is a step down from
Gurthang's queue; keep Gurthang's.

---

## 15. Bottom line

Loco is what Gurthang would look like if it prioritized **breadth and solo
developer productivity**: SeaORM for data, Tera for views, and a full set of
batteries (auth, scheduler, storage, cache, testing, jobs CLI). Gurthang kept
Loco's *shape* (`Hooks`, per-env config, generators, workers, mailers) and
swapped its *substance* for a compile-checked, Postgres-only, Inertia/React,
agent-first stack.

The practical read: **Gurthang's roadmap is largely "adopt the Loco batteries
Gurthang skipped"** — auth (T3), scheduler (T5), cache (T10), storage (T11),
pagination (T12), observability (T15), testing (T17). The parts Gurthang should
*not* copy are the ones that would trade away its edge: the runtime-checked ORM,
multi-DB, and Tera. Keep the compiler as the reviewer; borrow Loco's ergonomics
everywhere else.
