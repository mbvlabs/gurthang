# Gurthang Roadmap: Laravel Parity, One Topic at a Time

A work plan derived from the Laravel-vs-Gurthang comparison and the review notes
on it. Each **topic is a standalone unit of work** (one PR, one branch, one
review) so it can be picked up independently. Topics list their dependencies so
the order is explicit, but none require finishing the whole list first.

Guiding constraint, from the project brief: **mimic Laravel's workflow, but
optimize for the agentic area.** Every topic below names its "agentic angle" —
the property that makes the result easier for an autonomous developer to use,
verify, and reason about than the Laravel equivalent would be.

Legend: **P0** = unblocks other work, **P1** = high value, **P2** = valuable
later, **P3** = nice to have.

Recurring references: **Luca Palmieri** — *Zero to Production in Rust* and the
"Zero to Production" blog series (parse-don't-validate, type-driven domain
modelling). Used for T2 and T15.

---

## 0. How to use this plan

- Pick a topic, read its section, treat it as a spec. Each ends with
  **Acceptance** (done means these pass) and **Agentic angle**.
- Topics marked P0 are foundations: finishing them makes later topics smaller.
- The **sequencing** section at the end gives a dependency-ordered path.
- Anything that adds a generator also adds the matching `gurthang generate *`
  command, `--dry-run`, `sync`/`--check` where regeneration is meaningful, and a
  line in the README CLI block. This is the house style; do not ship a
  capability without its generator.

### Merges and deferrals (from review)

- **T6 (Notifications) merged into T5.** Yes — notifications share the dispatch
  and registration model with events/listeners/observers, so they are one topic.
- **T9 (Scheduler) folded into T5.** Yes — scheduled tasks are the time-triggered
  facet of the same "what runs when" list; they reuse `Task` and `register_*` on
  `Hooks`. Schedule is **Rust + worker clock**, not `scheduler.yaml`.
- **T4 (Authorization) merged into T3.**
- **T11 (File storage)** — not important currently; parked (see T11).
- **T16 (Search)** — not planned (see T16).
- **T19 → T0**, promoted to first priority and completed as a design lock
  ([`docs/adr/framework-shape.md`](adr/framework-shape.md)).

---

## T0 — Framework layout & design study: `rwf` + "Laravel in Rust" (P0, DONE)

**Status:** done (design lock). Accepted ADR:
[`docs/adr/framework-shape.md`](adr/framework-shape.md). Boot order:
[`docs/lifecycle.md`](lifecycle.md). No generated-app reshuffle in T0;
template and generator changes land when T1/T2/T5/T14 touch those files.

**Why.** Settle layout and architecture before T1 (middleware), T5
(registration), T8 (queue), and T14 (Hooks). Inputs: Laravel workflow, Loco
folders, Rwf attach/worker clock, Andurel `register_*` wiring, Pavex
middleware kinds.

**Locked.**
- Keep the current generated tree (`models/`, `src/{app,controllers,routes,services,workers,tasks,mailers,initializers}`). No `providers/`, no Laravel PHP paths, no `workers/` → `jobs/` rename.
- Apps depend on the `gurthang` facade; primitives stay in `gurthang-http`, `gurthang-inertia`, `gurthang-jobs`.
- App-definition API: more `register_*` **Hooks methods in `src/app.rs`** (Andurel/fx shape, no DI container, no `AppRegistry` bag).
- Middleware: Pavex `Wrap`/`Post`/`Pre`; membership in `Hooks::middlewares`; YAML does not influence middleware.
- Scheduler: `register_schedule` in Rust + worker clock. No `scheduler.yaml`.
- Queue: stay app-owned Postgres; extract `JobBackend`; keep `enqueue_in`.
- MVC: controllers may call models with `PgPool`; they must not embed `query!`. Services hold application logic.

**Depends on.** Nothing. Feeds T1, T5, T8, T14.

**Acceptance.** ADR accepted; lifecycle documented; per-topic locks written into T1/T5/T8/T14 below.

**Agentic angle.** A deliberate layout and `register_*` in `app.rs` is what
lets an agent place new code correctly and extend the framework without
guessing.

---

## T1 — Middleware v2: kinds + route/group registration (P0)

**Status:** in progress on `feat/pavex-middleware` (PR #2).

**Why.** PR #2 already reworks middleware into Pavex-style `pre` / `post` /
`wrap` kinds with a fixed `wrap -> post -> pre -> handler` composition, adds
telemetry, rate limiting, `authn::RequireAuth`, and `authz::RequireAuthz`, and
moves `session_auth` out of the initializer into `Hooks::middlewares`. T1
adds opt-in group attach and takes YAML out of middleware.

**Locked (T0, amended).** Keep PR #2 `MiddlewareKind` + `MiddlewareStack`.
Stack membership lives in `Hooks::middlewares` (Rust). YAML does not
influence middleware. Reuse `MiddlewareLayer` for group scope — no second
trait. Attach with `RouteGroup::add_mw(layer)` on a **sub-router once**,
then merge; global `apply_stack` once after merge.
`gurthang middleware --routes` prints names + kinds + scope.

**Scope.**
- `RouteGroup` / `BoundRoute` in `gurthang-http`; `RouteGroupExt::add_mw`
  in `gurthang` (wraps a sub-router once). Apps call it from
  `controllers/*/routes()` if they want. Not a `MiddlewareStack` in
  `routes()`. `mount!` stays thin for unwrapped controllers.
- The generated app keeps global `session_auth` `replace`. `RequireAuth`
  wraps dashboard routes (`RouteGroup::add_mw`). No generator scaffold for
  other resources (later topic).
- Default stack is Rust-only. Drop YAML `server.middlewares` from
  templates. `cors` / `compression` / `secure_headers` stay off until
  `push`. Placeholders for `session_auth`, `authn`, `authz`.
- `gurthang middleware` prints kind + name; `--routes` adds `scope`
  (`global` for the default stack this PR). Do not boot Postgres.

**Compile-time note (raised in review).** Does `mount!` cost compile time? Today
it expands to `Router::new().route(path, axum::routing::<verb>(on(ctrl, method)))`,
and `on` builds a `ControllerMethod<C, F>` that implements axum's `Handler` for
0–8 extractor args via `impl_controller_method!`. Each controller method is a
distinct monomorphized handler type, so compile time scales with the number of
handlers, not with `mount!` itself; `mount!` is a thin loop. The real risk is
per-route middleware *wrappers* multiplying monomorphized layer types. T0
locks wrapping a **group** router once over wrapping each route. Before and
after T1, measure with `cargo build --timings` (and `cargo build -Z
time-passes` on nightly) on the template app. Record the numbers in the PR.

**Files/areas.** `crates/gurthang/src/controller/middleware/mod.rs`,
`crates/gurthang/src/boot.rs`, template YAML + `src/app.rs`,
`docs/middleware.md`.

**Depends on.** T0 is done.

**Acceptance.** `RouteGroup::add_mw` is public and tested (grouped HTML 303
/ JSON 401, unwrapped sibling 200); `gurthang middleware --routes` prints a
`scope` column; `cargo test --workspace` passes; YAML does not influence
the stack; compile-time impact is measured and reported. Generator group
scaffolding is **out**.

**Agentic angle.** Per-route introspection (`middleware --routes`) lets an agent
see the real request lifecycle without reading source, and named kinds make
"add auth to this route" a one-line, greppable edit.

---

## T2 — Type-driven validation + models rework (P0, merged)

**Status:** gap. Only hand-rolled `validate` exists; failures follow the Inertia
redirect model. Review note: **merge this with a models rework**, and take
inspiration from **Luca Palmieri** ("Zero to Production" article + book) — this
is the parse-don't-validate / type-driven domain modelling approach the project
wants to adopt.

**Why.** Laravel's `FormRequest` + rule DSL is a top-used feature, and the
stronger Rust-native answer is to make **invalid states unrepresentable**: raw
input becomes a validated type only if every invariant holds, and the validated
type is the only thing downstream code can accept. This pairs naturally with a
models rework because the validated type should flow straight into the model's
`Create`/`Update` inputs.

**Current state.** `models/src/user.rs` has a free `validate` fn and
`UserError`; controllers store `errors`/`old.*` in session and redirect back.
`generate model` emits `Create<Model>Data`/`Update<Model>Data` and CRUD from the
live schema. No reusable validator, no `generate request`.

**Scope.**
- Introduce a `Validate`/`TryFrom<Raw>` convention plus a small `Validator`
  combinator set (required, email, min/max length, matches, custom closure) that
  collects field-keyed errors rather than failing on the first.
- Make the **validated type the boundary**: controllers parse `Raw -> Validated`
  once; services/models take `Validated` and may assume invariants. Document the
  pattern in `docs/validation.md`, citing Palmieri's article.
- **Models rework (merged):** have `generate model`/`sync model` emit validated
  constructor types alongside `Create`/`Update` data, so a model can only be
  constructed from validated input. Reconsider whether the `models` crate should
  own validation newtypes per column (e.g. `Email`, `Password`) so the DB layer
  and the domain layer agree on invariants.
- Map validation failures to the existing Inertia `errors` bag + `old.*` flash
  automatically (one helper, e.g. `invalid(...)` generalized).
- Add `gurthang generate request <Name>` producing a typed request struct with a
  `parse` fn and a stub rule set; wire it so controllers can extract it.
  Generated types live in `src/http/requests/` (T0 reserved layout).
- Keep it dependency-light; do not adopt a heavy validation crate unless it
  preserves the typed-boundary property.

**Files/areas.** new `crates/gurthang-validate/` (or `gurthang::validation`),
`crates/gurthang-generate/src/{model.rs,request.rs,templates/*}`,
`crates/gurthang-new/templates/{models,src/controllers,src/http/requests}/*`,
`docs/validation.md`.

**Depends on.** Nothing (T1 optional for auth-route demo).

**Acceptance.** Auth register/login use the typed boundary; a generated model
and a generated request compile and reject bad input with the same `errors`
shape as today; `cargo test` covers the combinators and the redirect path;
`docs/validation.md` shows the parse-don't-validate pattern with a worked
example from the article.

**Agentic angle.** The compiler enforces the invariant, so an agent cannot
forget a check; the error shape is uniform, so the agent's fix is deterministic.

---

## T3 — Auth + authorization completeness (P1, splittable)

**Status:** core register/login/logout exists; most of the rest is absent.
Review note: **do T3 (auth) and T4 (authorization) together** — this topic
absorbs the former T4.

**Why.** The review note: "we really should get the Absent ones in there as
well." These are the classic Laravel auth batteries, and authorization is the
access-control layer directly above them.

**Scope (ship in this order):**
1. **Session rotation hardening + remember-me.** Rotate on login (exists);
   add a remember-me token with its own rotating selector/validator.
2. **Email verification.** Signed verification URL, `verified` middleware/guard,
   `MustVerifyEmail` equivalent, resend endpoint.
3. **Password reset.** Reset tokens table, request + reset endpoints, token
   expiry, invalidation of sessions on reset (session auth hash already keys on
   the password hash, so this is mostly wiring).
4. **Authorization (former T4).** Define an `Ability`/`Authorizer` trait (PR #2
   already sketches `authz::RequireAuthz` with a pluggable `Authorizer`, off by
   default), a registration surface (policies per resource), `generate policy
   <Name>` with `viewAny/view/view/create/update/delete` stubs, an
   `authorize!`-style helper usable in controllers and as a pre-middleware, and
   optionally roles/permissions.
5. **API tokens (Sanctum-lite).** Personal access tokens table, bearer
   authentication guard, ability scopes; a `TokenAuth` pre-middleware (reuses
   T1 kinds).
6. **OAuth (social) + MFA.** Optional/later; OAuth via a provider trait, MFA via
   TOTP with recovery codes.
- Add generators: `generate mailer` already exists for the emails; consider
  `generate auth` or extend `generate scaffold` for the reset/verify pages, and
  `generate policy` for authorization.

**Files/areas.** `crates/gurthang-new/templates/src/{controllers,services,models}`,
`crates/gurthang-new/templates/migrations/*`, `crates/gurthang-http/src/*`
(new guard), `crates/gurthang/src/controller/middleware/{authn,authz}.rs`,
`crates/gurthang-generate/{policy.rs,templates/policy.rs}`, template app example.

**Depends on.** T1 (guards/middleware), T2 (forms + models rework), T7 (mail).

**Acceptance.** A fresh app can: verify email, reset password, stay logged in
with remember-me, authorize a policy-guarded action, and authenticate an API
call with a token; each has an integration test.

**Agentic angle.** Auth and authz are the most common agent tasks; every flow
ships as a generated, tested slice so the agent extends rather than invents.

---

## T5 — Dispatch: events, listeners, observers, notifications, scheduler (P1)

**Status:** absent. Absorbs the former **T6 (Notifications)** and **T9
(Scheduler)** per review ("should notifications be merged into T5? give me
yes/no" → **yes**; "isn't this [scheduler] also part of T5?" → **yes**). Review
note: "focus ... on policy/event/listener/observer/notification" and "would be
good to focus on soon."

**Why.** These are all "what runs when, and who reacts" — one registry, one
dispatch story. Decouples side effects from request handling the Laravel way.

**Scope.**
- A typed in-process **event bus**: `Event` trait, `dispatch(event)`, typed
  listeners. Async listeners may enqueue jobs (ties to T8).
- **Model observers**: after-create/update/delete hooks emitted by the generated
  model CRUD (the model template already knows its columns).
- **Notifications (former T6):** a `Notification` trait with `Mail` (via the
  existing `Mailer`), `Database` (a `notifications` table + Inertia shared
  `notifications` prop), and later `Slack`/webhook channels; a `Notifiable` trait
  on `User`; `generate notification <Name>`; an in-app bell/list in the template.
- **Scheduler (former T9):** `register_schedule` on `Hooks` in `src/app.rs`.
  The worker process hosts the clock (Rwf). `gurthang schedule` / `--list` for
  agents. Each entry dispatches a `Task` or a job. **No `config/scheduler.yaml`.**
- Generators: `generate event|listener|observer|notification <Name>`, patching
  the matching `register_*` method in `src/app.rs` (same insertion style as
  today's `Hooks::routes` / `register_tasks` patches).
- Document ordering/at-least-once semantics and how an event fans out to
  listeners + queued jobs.
- **Registration (locked by T0):** more `register_*` methods on `impl Hooks
  for App` in `src/app.rs`. The trait method *is* the list. Do not add an
  `AppRegistry` / `AppBuilder` bag, a `providers/` directory, or
  `docs/adr/registration.md` as a second design. Empty `workers::x::register`
  stubs go away; listing the type on `register_workers` is enough. Rename
  `Hooks::routes` → `register_routes` when this topic (or T14) touches `app.rs`.

**Files/areas.** new `crates/gurthang-events/` (and
`crates/gurthang-notifications/`), `crates/gurthang-generate/src/
{event.rs,listener.rs,observer.rs,notification.rs}` + templates,
`crates/gurthang/src/app.rs` (`Hooks`), template `src/app.rs`, model template
emit points, worker clock in `gurthang-jobs`.

**Depends on.** T8 (for queued listeners/delivery), T7 (mail channel). T0 is
done.

**Acceptance.** Dispatching an event runs its listeners; a model create fires an
observer; a notification writes a DB row and/or enqueues mail; a scheduled task
fires on its cadence via the worker clock and can be listed; generators patch
`register_*` in `app.rs`; tests cover fan-out; at least two subsystems use the
same `register_*` shape.

**Agentic angle.** One registry for "what runs when" turns every side effect into
a registered, greppable unit an agent can add without touching the original code
path.

---

## T7 — Mail hardening (P3)

**Status:** mailers exist (queued + sync, SMTP + stub). Review note: "take
inspiration from my Go framework that lets us use Tailwind classes in emails."

**Why.** Minor gaps relative to Laravel: no mail previews, no local "view in
browser" catcher, and — per review — no styled email templates.

**Scope.** A dev mail viewer (log/DB-backed), attachment support, a
`Mailer::fake`-style test double, and **Tailwind-in-email**: allow authoring
email HTML with Tailwind utility classes and compile them to inline styles (the
Go framework's approach), so mailers reuse the same styling vocabulary as the
web app instead of hand-written CSS. Consider rendering via a headless pass that
inlines computed styles, and ship a default styled layout.

**Depends on.** Nothing (T3 verification/reset emails benefit from it).

**Acceptance.** Dev can inspect rendered mail; tests can assert mail was queued
without sending; an email authored with Tailwind classes renders with inlined
styles in major clients.

**Agentic angle.** A `Mail::fake`-style double makes agent-authored tests
deterministic; one styling vocabulary (Tailwind) means the agent styles email
the same way it styles pages.

---

## T8 — Queue/jobs: `JobBackend` on the Postgres queue (P0 for T5)

**Status:** app-owned Postgres queue exists (`background_jobs`,
`FOR UPDATE SKIP LOCKED`, leases, retries, backoff, LISTEN/NOTIFY). T0 locks
this as the path; T8 still extracts a swappable backend and the inspector.

**Why.** T5 queued listeners need a stable enqueue API. Fang/apalis stay
possible later behind a trait; they are not the v1 rewrite.

**Scope.**
- **Decision record** (`docs/adr/queue.md`): record the T0 lock (app-owned
  Postgres) and the Fang second-pool evaluation so the fork is closed, not
  reopened.
- Extract a `JobBackend` trait so storage can change without rewriting
  callers.
- Preserve `enqueue_in(&mut PgConnection, …)` (business write + enqueue in
  one transaction).
- Add **`gurthang jobs`**, then batches, chains, and job middleware (rate
  limit / uniqueness).

**Files/areas.** `crates/gurthang-jobs/*`, `crates/gurthang-cli` (`jobs`
command), `docs/adr/queue.md`.

**Depends on.** Nothing; unblocks T5.

**Acceptance.** `docs/adr/queue.md` records the Postgres lock; `JobBackend` is
extracted; transactional enqueue and leases are preserved and tested;
`gurthang jobs` reports queue state.

**Agentic angle.** A `gurthang jobs` inspector plus stable semantics let an
agent debug async work through a command, not a debugger.

---

## T10 — Caching (P2)

**Status:** absent (in-process only). Review note: "agree — let's just use
Postgres for now and try to standardize it."

**Why.** `Cache::remember` is ubiquitous.

**Scope.** A single, standardized `Cache` trait with a **Postgres-backed**
default (unlogged table or dedicated `cache` table), `get/put/remember/forget`,
TTL; a `gurthang cache clear` command. Do not build a multi-driver abstraction
yet; standardize on Postgres so caching has one shape across the framework.

**Depends on.** Nothing.

**Acceptance.** `remember` caches and expires; a command clears; tests cover TTL.

**Agentic angle.** One standardized cache keeps agents from hand-rolling
invalidation or inventing a second storage story.

---

## T11 — File storage (parked)

**Status:** review note: "Not important currently." Parked; not in the active
plan. Revisit only when an app needs uploads.

**If picked up later.** A `Storage` trait (local default, S3 later),
upload/download/delete, signed URLs (ties to T13), a dev local disk mapping, and
`generate` for a file migration.

---

## T12 — Pagination + collections (P2)

**Status:** absent.

**Scope.** A `Page<T>` result type, a `paginate` query helper, Inertia
`links`/`meta` props, and a React pagination component. Optionally a small
`Collection`-style extension set on `Vec`.

**Agentic angle.** Standard pagination removes a repetitive, error-prone
re-invention per list endpoint.

---

## T13 — Cookie encryption + signed URLs (P2)

**Status:** review note: "this should likely go in soon." Cookies carry the
session id and `XSRF-TOKEN`; session payloads are MessagePack-encoded, not
encrypted; no signed URLs.

**Scope.**
- Encrypt cookie payloads with an app key (AEAD), keyed from config/env, with
  rotation support.
- Signed URLs for verification/reset/download (HMAC + expiry) and a
  `signed` guard.
- Wire `session_auth`/`csrf` to use the encrypted cookie; document key handling.

**Files/areas.** `crates/gurthang-http/src/{session.rs,csrf.rs}`,
`crates/gurthang/src/config.rs` (app key), template `.env.example`.

**Depends on.** Nothing.

**Acceptance.** Cookies are encrypted; a signed URL validates and expires;
existing session/CSRF tests pass.

**Agentic angle.** Crypto handled in the framework means the agent cannot ship a
plaintext-cookie mistake.

---

## T14 — Rework the Hooks lifecycle (P3, pushed back)

**Status:** design locked by T0; code deferred. Current vs target is in
[`docs/lifecycle.md`](lifecycle.md). Review note: "let's push this back so I
can focus on this later" / Loco-shaped Hooks.

**Why.** `Hooks` today mixes catalogs (`routes`, `export_payloads`) with
lifecycle (`before_routes`, two `after_routes`). T0 keeps Hooks and adds
`register_*`; T14 applies that in code.

**Scope (deferred; apply T0, do not redesign).**
- Keep [`docs/lifecycle.md`](lifecycle.md) in sync with `boot.rs`.
- Default `boot` to `create_app::<Self>`.
- Collapse `before_routes` / `Hooks::after_routes` / `Initializer::after_routes`
  to **one** post-merge router hook (template asset mount).
- Initializers mutate `Context` only (view engine). Session/auth is
  `Hooks::middlewares` (T1/PR #2).
- Move `export_payloads` off `Hooks` (`gurthang sync payloads` / export bin).
- Rename `routes` → `register_routes` and `connect_workers` →
  `register_workers` if T5 has not already. Keep the `register_*` family;
  do not add more Loco lifecycle hooks.

**Depends on.** T0 (done). Can land before or after T5; T5 adds more
`register_*` slots on the same trait.

**Acceptance (when done).** Template uses the target API in
[`docs/lifecycle.md`](lifecycle.md); `export_payloads` is not on `Hooks`;
one post-merge router hook; no behavior regressions.

**Agentic angle.** An explicit, documented lifecycle is what an agent needs to
extend an app correctly on the first try.

---

## T15 — Observability (P1, focus area)

**Status:** `tracing` logs only; PR #2 adds telemetry middleware + a
`MetricsRecorder` trait. Review note: "we should focus entirely on
observability. Luca Palmieri's book can be used here once we are ready."

**Why.** The review elevates this to a focus area. Observability is also the
agent's eyes: without machine-readable runtime state, an agent cannot diagnose a
running app.

**Scope.**
- Adopt the Palmieri (*Zero to Production*) observability stack: structured
  `tracing` with spans per request and per job, `request_id` correlation, and a
  layered subscriber (stdout for dev, JSON for prod).
- **Metrics**: make the `MetricsRecorder` trait concrete with a Prometheus/OTLP
  exporter; instrument requests, DB, jobs, and mail.
- **Job/queue inspector**: `gurthang jobs` (queued/running/failed counts, retry,
  purge) — depends on T8.
- A Telescope/Pulse-lite dev panel (requests, queries, jobs, mail) if useful.
- Health/readiness endpoints.
- Everything queryable as structured output (ties to T18 `--json`).

**Depends on.** T1 (telemetry middleware), T8 (job inspector).

**Acceptance.** A running app emits correlated structured logs, request/job
metrics, and a health endpoint; `gurthang jobs` reports queue state; the dev
panel shows recent requests and jobs.

**Agentic angle.** Machine-readable runtime state is how an agent debugs without
a human at the keyboard.

---

## T16 — Search (not planned)

**Status:** absent. Review note: "Not important. This can be implemented by the
user if they want."

**Decision.** Out of scope. Postgres FTS is available to app authors directly;
Gurthang will not ship a Scout-style abstraction. Revisit only if a pattern
emerges across apps.

---

## T17 — Testing scaffold + fakes (P1)

**Status:** `cargo test --workspace` documented; no generated tests; no fakes.

**Scope.** `generate test <Name>` (HTTP + model templates), a test harness that
boots the app against a test DB, and fakes for mail/queue/notifications
(`Mailer::fake`, `JobQueue::fake`, `Notification::fake`). `db rebuild`/test.yaml
already exist as building blocks.

**Depends on.** T7/T8/T5 for fakes.

**Acceptance.** A generated test runs and asserts; fakes intercept side effects.

**Agentic angle.** A generated, runnable test is the agent's own verification
loop; fakes make it deterministic.

---

## T18 — Agent-facing DX (P0, continuous)

**Status:** partial. CLI is scriptable; `sync --check` exists; generators have
`--dry-run`. No JSON output, no aggregate check, no manifest.

**Why.** This is the "optimized for the agentic area" mandate made concrete.

**Scope.**
- `--json` on `routes`, `middleware`, `db migrate status`, `sync --check`,
  `jobs`, `tools check`.
- `gurthang check`: one command running `sync * --check`, `cargo check`,
  `cargo sqlx prepare --check`, and `tsc --noEmit`, returning unified pass/fail
  with a drift diff.
- `gurthang manifest`: machine-readable description of every command, flag, and
  generated file (self-documenting framework).
- Make `--dry-run` emit a **diff/manifest**, not just a file list, so a change
  can be reviewed before applying.
- Publish the "house style" (every capability ships with a generator + check +
  docs) as a contributor doc.

**Depends on.** Nothing; do incrementally alongside other topics.

**Acceptance.** `gurthang check` gives a single pass/fail; `--json` parses on
the core commands; `gurthang manifest` describes the CLI.

**Agentic angle.** This is the whole point: introspection, structured output,
and a one-shot consistency check.

---

## Sequencing

Dependency-ordered path (parallelizable where noted):

```
Phase 1 (foundations)
  T0  Framework layout & design study                          [DONE — adr/framework-shape.md]
  T1  Middleware kinds + route/group registration               [PR #2 in flight]
  T2  Type-driven validation + models rework                    [Palmieri]
  T18 Agent-facing DX (continuous, start early)
  T8  JobBackend on Postgres queue (unblocks T5)

Phase 2 (access + side effects)
  T3  Auth + authorization completeness (merged T3+T4)
  T5  Dispatch: events / listeners / observers / notifications / scheduler
      (merged T5+T6+T9; implement register_* + worker clock)

Phase 3 (conveniences)
  T13 Cookie encryption + signed URLs
  T12 Pagination + collections
  T10 Caching (Postgres, standardized)
  T17 Testing scaffold + fakes
  T15 Observability (focus area)                    [Palmieri; after T1, T8]

Phase 4 (breadth / later)
  T7  Mail hardening (Tailwind emails)
  T14 Hooks lifecycle rework                        [apply T0 ADR + lifecycle.md]
  T11 File storage                                  [parked]
  T16 Search                                        [not planned]
```

**Highest leverage first:** T1, T2, T8, T18 — T0 is done and they shrink every
later topic.

---

## Cross-cutting rules for every topic

1. **Ship the generator.** A capability without `gurthang generate *` (and
   `sync --check` where regeneration applies) is half-done.
2. **Add `--dry-run` and, where relevant, `--json`.**
3. **Non-destructive by default.** New files refuse to clobber; `sync` is the
   overwrite path.
4. **Compile-checked where possible.** Prefer types and `sqlx::query!` over
   runtime validation.
5. **Document the mental model** (`docs/*.md`) with a worked example.
6. **Test the happy path and the redirect/error path.**
7. **Record decisions** in `docs/adr/` when a fork is taken. Framework shape
   and registration (`register_*` on Hooks) are locked in
   [`docs/adr/framework-shape.md`](adr/framework-shape.md). Queue backend
   still writes [`docs/adr/queue.md`](adr/queue.md) in T8.
