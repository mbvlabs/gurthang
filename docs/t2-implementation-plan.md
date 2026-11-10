# T2 implementation plan

Implements [`t2-validation-plan.md`](t2-validation-plan.md) and
[`t2-cli-plan.md`](t2-cli-plan.md) on `master` @ `7592187`. The locked
decisions from both plans are not reopened here. This doc covers **how** each
one lands, file by file, plus the few choices those plans leave open.

Nothing gets committed until you ask.

## Already written (pre-gate, not compiled yet)

These went in before this gate. They follow the locked shape. They stay or
get changed based on your review.

- `crates/gurthang-parse/`: a zero-dependency leaf crate holding `ErrorBag`
  plus unit tests and a doctest.
- Workspace `Cargo.toml` member, `crates/gurthang/Cargo.toml` dependency.
- `gurthang/src/lib.rs`: `pub use gurthang_parse as parse;`,
  `pub use gurthang_parse::ErrorBag;`, `pub use invalid::invalid;`.
  The prelude exports `ErrorBag` and `invalid`.
- `gurthang/src/invalid.rs`: `invalid(&Session, destination, ErrorBag)`.

## 1. `gurthang-parse`

```rust
pub struct ErrorBag { errors: BTreeMap<String, String>, old: BTreeMap<String, String> }

impl ErrorBag {
    pub fn field<T, E: Display>(&mut self, field: &str, r: Result<T, E>) -> Option<T>;
    pub fn add(&mut self, field, message);        // first message per field wins
    pub fn keep_old(&mut self, field, value);     // opt-in; never secrets
    pub fn with_error(self, ..) -> Self;  pub fn with_old(self, ..) -> Self;
    pub fn is_empty(&self) -> bool;  errors();  old();  into_parts();
}
```

- Collects every field error; it does not stop at the first failure.
  "Field mapping" is `errors.field("email", Email::parse(..))`: the error's
  `Display` becomes the Inertia message.
- Old input is **opt-in** per field, so a password can never be flashed by
  accident.
- `TryFrom` shape, with no `unwrap`:

```rust
let mut errors = ErrorBag::new();
errors.keep_old("email", &payload.email);
let email = errors.field("email", Email::parse(payload.email));
let password = errors.field("password", NewPassword::parse(payload.password));
let (Some(email), Some(password)) = (email, password) else { return Err(errors) };
Ok(Self { email, password })
```

## 2. `gurthang::invalid`

```rust
pub async fn invalid(session: &tower_sessions::Session, destination: impl AsRef<str>, errors: ErrorBag)
    -> gurthang::Result<Response>
```

Writes `errors` (the existing `SharedProps.errors` key) and `old.<field>`
for each kept value, then `mutation_redirect` (303). It is a plain function,
not an extractor. The template's `AppError` gains
`Gurthang(#[from] gurthang::Error)` so `?` works.

## 3. Template domain types (`models/src/user.rs`)

Models crate: no dependency on `gurthang-parse`. Each newtype has its own
`thiserror` error whose `Display` is the user-facing message.

| Type | `parse(String)` rules | Error message(s) | Notes |
| --- | --- | --- | --- |
| `Email` | trim + lowercase; ≤254; exactly one `@`, non-empty local + domain; no whitespace | "Enter a valid email address." | `AsRef<str>`, `Clone`, `Debug` |
| `NewPassword` | non-empty; ≥12 chars (`chars().count()`) | "Password is required." / "Password must be at least 12 characters." | `Debug` prints `[REDACTED]` |
| `PasswordAttempt` | non-empty only | "Password is required." | `Debug` prints `[REDACTED]` |

```rust
pub struct UserCreateData { pub email: Email, pub password: NewPassword }

impl User {
    pub async fn create(conn: &mut PgConnection, email: &Email, password_hash: String) -> Result<Self, UserError>;
    pub async fn find_by_email(pool: &PgPool, email: &Email) -> sqlx::Result<Option<Self>>;
    pub async fn hash_password(password: &NewPassword) -> Result<String, UserError>;
    pub async fn verify_password(&self, attempt: &PasswordAttempt) -> Result<bool, UserError>;
}
```

- Row `User.email` stays `String`. The SQL text is unchanged, so the
  existing `.sqlx` offline files stay valid.
- Delete `CreateUserData`, `validate()`, and `normalize_email()`
  (`Email::parse` normalizes).
- Duplicate email stays `UserError::DuplicateEmail` (DB unique violation).
- Unit tests in `user.rs`: `Email::parse` / `NewPassword::parse` /
  `PasswordAttempt::parse`, valid and invalid cases, plus redacted `Debug`.
- A `compile_fail` doctest on `User::create` shows that passing a `String`
  email is rejected.
- `UserFactory::data()` runs its strings through `parse` (see Q3).

## 4. Template HTTP layer

New (T0 reserved layout): `src/http/mod.rs`, `src/http/requests/mod.rs`,
`src/http/requests/user_registration.rs`, `src/http/requests/user_login.rs`,
plus `pub mod http;` in `src/lib.rs`.

- `UserRegistrationPayload { email: String, password: String }`, with
  `#[serde(default)]` so a missing field becomes a collected error instead
  of a 400. It derives no `Debug`, so the password cannot be logged.
- `impl TryFrom<UserRegistrationPayload> for UserCreateData` (`Error = ErrorBag`).
- `UserLoginPayload` → `services::auth::Credentials { email: Email, password: PasswordAttempt }`.
- `controllers/auth.rs`:
  - `parse_auth_form` becomes a generic `read_payload::<T>(request)` in
    `controllers/shared.rs` (JSON or urlencoded, as today).
  - Register: `match UserCreateData::try_from(payload)`; on `Err` →
    `invalid(&auth_session.session, auth::REGISTER, errors)`.
    `DuplicateEmail` → `invalid` with an `ErrorBag` holding `email` + old email.
  - Login: parse failure → `invalid(.., auth::LOGIN, errors)` (see Q2).
    Auth failure → the generic `credentials` message, as today.
  - The local `invalid` fn and `RegistrationError::Validation` are deleted.
- `services/auth.rs`: `register(UserCreateData)` hashes and creates, with
  no validation left inside it.
- `Pages/Auth/Login.tsx`: render `errors.email` / `errors.password` (only
  when Q2 = field errors).

## 5. `gurthang generate request <Name> [--dry-run]`

- The name is pascal/snake-cased. A trailing `Payload` is stripped, so both
  `WidgetCreate` and `WidgetCreatePayload` work.
- Writes `src/http/requests/<snake>.rs`. Ensures `pub mod <snake>;` in
  `src/http/requests/mod.rs`, `pub mod requests;` in `src/http/mod.rs`, and
  `pub mod http;` in `src/lib.rs`, creating them if missing (older apps).
  Errors if the file already exists. There is no `sync request`.
- Stub (see Q1):

```rust
use gurthang::ErrorBag;
use serde::Deserialize;

// Import the domain type this request parses into (AGENTS.md, Models), e.g.
// use crate::models::widget::WidgetCreateData;

/// Form fields exactly as the browser sends them. Strings, no invariants.
#[derive(Deserialize)]
pub struct WidgetCreatePayload {
    // #[serde(default)]
    // pub name: String,
}

impl TryFrom<WidgetCreatePayload> for WidgetCreateData {
    type Error = ErrorBag;
    fn try_from(payload: WidgetCreatePayload) -> Result<Self, Self::Error> {
        let mut errors = ErrorBag::new();
        // errors.keep_old("name", &payload.name);
        // let name = errors.field("name", WidgetName::parse(payload.name));
        // let Some(name) = name else { return Err(errors) };
        // Ok(Self { name })
        let _ = (payload, &mut errors);
        todo!("parse WidgetCreatePayload into WidgetCreateData")
    }
}
```

- stdout: `Wrote …`, `Updated …`, then
  `Next: define/import WidgetCreateData, fill try_from, call gurthang::invalid on Err`.
- clap: `/// Write an HTTP *Payload and its TryFrom into a domain *Data.`
- Root README CLI block: `gurthang generate request <Name>`.
- Tests: render unit test, plus a tempdir write test (file + mod lines,
  idempotent, refuses overwrite, dry-run writes nothing).

## 6. Agent teaching (no `generate model` rewrite)

- `generate model` clap `after_help`: "Schema types only. HTTP input is
  parsed in a *Payload: `gurthang generate request <Name>Create`. See
  AGENTS.md (Models)."
- stdout after `generate model` (dry-run too), exactly as in
  `t2-cli-plan.md`:

```text
HTTP parse boundary: gurthang generate request WidgetCreate
Do not pass form strings into Widget::create — parse a *Payload first.
See AGENTS.md (Models).
```

- `generate scaffold`: the same hint on stdout, plus a comment above the
  generated `impl From<WidgetForm> for CreateWidgetData` saying it binds form
  strings straight into `::create` and should be replaced by a parsed
  `*Payload` (`gurthang generate request WidgetCreate`). The scaffold body is
  otherwise unchanged.
- New template `AGENTS.md` (added to the `scaffold!` manifest): the Models
  section from `t2-cli-plan.md` verbatim, plus three lines on layout and the
  worked example (`models/src/user.rs`, `src/http/requests/user_registration.rs`).
- App README "Growing the app": migrate → model → **request** → scaffold,
  with one sentence pointing at AGENTS.md.
- Tests: `cli.rs` asserts `generate model --help` mentions
  `generate request` + `AGENTS.md`, and `generate --help` lists `request`.
  `generated_project.rs` asserts that `AGENTS.md` and the
  `src/http/requests/*` files exist, `user.rs` has `create(` with
  `email: &Email`, and there is no `validate(` or `CreateUserData`.

## 7. Tests for the redirect path

- `crates/gurthang/tests/invalid.rs` (runs in `cargo test --workspace`): an
  axum router with a `tower_sessions::MemoryStore` session layer. A
  register-shaped handler parses a form into a payload via `TryFrom` → `ErrorBag`
  → `invalid`. POST with a bad email and a short password must give a 303 to
  `/register`. A probe route reads the session: `errors.email`,
  `errors.password` (both, so collection works), `old.email` == the typed
  value, and no `old.password`.
- Template `src/controllers/auth.rs` `#[cfg(test)]`: the same assertions
  against the real `Auth::register_user`, with the router built from
  `controllers::auth::routes` + MemoryStore + axum-login on a lazy pool
  (parse fails before any DB call).
- `gurthang-cli/tests/generated_project.rs` gains an `#[ignore]`d e2e:
  `gurthang new` in a tempdir, then `SQLX_OFFLINE=true cargo test` inside the
  app. I run it by hand for this PR and report the result. It is too slow
  for the default suite.

## 8. Docs

- `docs/validation.md`: parse-don't-validate, citing Palmieri's article.
  Payload → `TryFrom` → `*Data` → `Model::create`, `ErrorBag` + `invalid`,
  the User worked example, `generate request`, and what stays out
  (validator/garde, sqlx `Decode`, the redirecting extractor).
- `docs/roadmap.md`: T1 status **done** (docs only; `--timings` out). T2
  status updated to point at `validation.md`.
- ADR `framework-shape.md`: not touched beyond this note. Its "`Raw`"
  wording predates the naming lock, so I'll correct `TryFrom<Raw>` →
  `TryFrom<*Payload>` there (one line), unless you object.

## Verification

`cargo test --workspace` (zig-cc shim lives in the scratchpad only, not
committed), plus the manual generated-app run from §7. Then
`plannotator review --tailscale` on the diff.

## Out

`generate model` rewrite, validator/garde, sqlx `Decode` on the row, an
auto-redirect extractor, T3 verify/reset, T18 check/`--json`/manifest,
free-function SQL, T1 `--timings`.

---

:::question
`generate request` stub: what should force the agent to finish it?

The plan says "stub will not compile until then". The stub names
`<Name>Data`, which usually does not exist yet, so the build fails until the
agent imports or creates it. The body is a `todo!()` with a commented example.

- [ ] Unresolved `<Name>Data` + `todo!()` body — fails to compile until the type exists; then panics if the body is left as-is
- [ ] Unresolved `<Name>Data` + `compile_error!` in the body — never compiles until the agent rewrites the body
- [ ] Add `--data <Type>` flag so the stub imports a real type; body `todo!()`

Recommended: Unresolved `<Name>Data` + `todo!()` body
:::

:::question
Login: what does a malformed email or empty password show?

Auth failure stays the generic credentials message (locked). This is only
about input that fails `Email::parse` / `PasswordAttempt::parse` before any
DB lookup. Field errors do not leak whether an account exists.

- [ ] Field errors (`errors.email`, `errors.password`); Login.tsx renders them
- [ ] Map any parse failure to the generic `credentials` message too

Recommended: Field errors (`errors.email`, `errors.password`); Login.tsx renders them
:::

:::question
`UserFactory::data()` once its strings go through `parse`?

- [ ] `data() -> UserCreateData` that panics with a clear message on bad factory input (test fixture)
- [ ] `data() -> Result<UserCreateData, …>` (callers `?`/`unwrap`)

Recommended: `data() -> UserCreateData` that panics with a clear message on bad factory input (test fixture)
:::

:::question
Where does the shared body reader (`read_payload::<T>`, JSON or form) live?

- [ ] App template `controllers/shared.rs` — no new framework surface
- [ ] Facade `gurthang::read_payload` — generated requests and scaffolds can reuse it

Recommended: App template `controllers/shared.rs` — no new framework surface
:::
