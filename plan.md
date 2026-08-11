# Gurthang Proof-of-Concept Implementation Plan

## 1. Purpose

Gurthang is a proof of concept for evaluating whether a Rust-based, Rails-like web application stack can provide acceptable day-to-day iteration speed without recreating Andurel's full generator and framework surface.

The project must provide a small project-initialization CLI and generate a self-contained application with:

- Axum and Tokio for HTTP and async execution.
- PostgreSQL and SQLx for persistence.
- A conventional MVC application structure.
- Tera for server-rendered HTML.
- Datastar for a minimal server-driven interactive path.
- A custom, typed Inertia.js v3 server adapter for a React application.
- React 19, Vite, and Tailwind CSS for the Inertia frontend.
- Rust-authored page contracts exported to TypeScript.
- Users, registration, login, logout, persistent sessions, CSRF protection, and a protected page.
- A repeatable way to measure cold and incremental development iteration time.

This is not intended to be a complete web framework. The implementation should stay deliberately small, readable, and editable by the generated application's owner.

## 2. Product Boundary

### Included

- A `gurthang` executable.
- A single project creation command: `gurthang new`.
- One generated application shape.
- PostgreSQL as the only database.
- React as the only Inertia client adapter.
- A minimal Inertia.js v3 protocol subset sufficient for the generated application.
- One public Tera page and one Datastar interaction.
- Authentication with persistent server-side sessions.
- Development frontend hot reload through Vite and React Fast Refresh.
- Production frontend assets resolved through a Vite manifest.
- Tests for the CLI, generated application, protocol behavior, and authentication.

### Explicitly excluded

- Model, controller, view, CRUD, migration, or resource generators.
- An upgrade command or generated-file ownership system.
- Multiple database engines.
- Multiple Inertia frontend adapters.
- Email verification, password reset, OAuth, MFA, remember-me, roles, or permissions.
- Background jobs, email delivery, telemetry exporters, or deployment packaging.
- Inertia SSR.
- Full Inertia v3 support for deferred, merged, once, infinite-scroll, history-encryption, Precognition, or SSR metadata.
- A custom procedural macro until the plain trait-based page contract proves too repetitive.
- General-purpose repository traits or an ORM abstraction over SQLx.
- A component library for either React or Tera.

## 3. Architectural Decisions

### 3.1 Database access

Use SQLx rather than SeaORM. Database models remain ordinary Rust structs with explicit SQL and domain behavior.

Initially pin the generated application to SQLx 0.8 because the current PostgreSQL integration for `tower-sessions` targets SQLx 0.8. Avoid two incompatible SQLx pool types and avoid writing a custom session store during the proof of concept. Isolate database access so a SQLx 0.9 upgrade can be evaluated independently later.

Use `#[derive(sqlx::FromRow)]` with `query_as::<_, T>()` initially. Do not require a live database during compilation and do not require `.sqlx` offline metadata in the first iteration. Runtime database integration tests provide query validation.

PostgreSQL is an intentional application constraint. Model query functions may accept `&mut PgConnection` rather than introduce highly generic executor signatures. Controllers acquire a connection; services begin transactions and pass their transaction connection into model operations.

### 3.2 MVC boundaries

- `models/`: database-shaped entities, domain validation, create/update inputs, password behavior, persistence, finders, and domain errors.
- `services/`: multi-step workflows, transactions, and coordination. Do not add pass-through services around one model call.
- `controllers/`: Axum extractors, request parsing, HTTP-specific validation mapping, redirects, sessions, and renderer selection.
- `views/`: presentation DTOs and page contracts for Inertia plus Tera-specific presentation values.
- `web/`: reusable framework plumbing such as Inertia protocol handling, Tera rendering, Datastar integration, assets, CSRF, and HTTP error responses.
- `routes.rs`: application route composition and paths.
- `app.rs`: application state and lifecycle assembly.

Models must not depend on Axum, Inertia, Tera, Datastar, cookies, or route names.

### 3.3 Generated code ownership

The project initializer copies all application and protocol code into the generated project. Generated applications own and may edit that code. Do not introduce a reusable Gurthang runtime crate in the first proof of concept.

The CLI embeds a template directory and performs a small, deterministic placeholder substitution. Do not use Tera to render the scaffold because generated Tera templates would collide with scaffold syntax. Use unmistakable placeholders such as:

- `__GURTHANG_PROJECT_NAME__`
- `__GURTHANG_CRATE_NAME__`
- `__GURTHANG_PACKAGE_NAME__`

### 3.4 Inertia page contracts

Rust presentation DTOs are the single source of truth for page props. Controllers must not pass an independent component string and an untyped JSON map.

The central contract should resemble:

```rust
pub trait InertiaPage: serde::Serialize {
    const COMPONENT: &'static str;
}
```

The renderer accepts `P: InertiaPage`, obtains the component name from `P::COMPONENT`, and serializes the value as page props.

Every page DTO derives `serde::Serialize` and `ts_rs::TS`. Generated TypeScript is consumed directly by the React page. Models are never serialized directly to the browser; safe presentation DTOs explicitly select exposed fields.

### 3.5 Authentication

Use `axum-login` with PostgreSQL-backed `tower-sessions`. Store password hashes as Argon2id PHC strings using the RustCrypto `argon2` crate. Execute password hashing and verification with `tokio::task::spawn_blocking` so memory-hard hashing does not block a Tokio worker.

Use the password hash as the user's session authentication hash so a future password change invalidates existing sessions.

Use Inertia v3's XSRF convention:

- Server sends an `XSRF-TOKEN` cookie readable by JavaScript.
- Inertia sends it back as `X-XSRF-TOKEN` for state-changing requests.
- Server verifies the header for `POST`, `PUT`, `PATCH`, and `DELETE`.
- The authentication session cookie is `HttpOnly`, `SameSite=Lax`, and `Secure` in production.
- Rotate the session identifier after successful login and registration.

Validation failures follow Inertia's redirect model: store errors in the session, redirect back, and share them through `page.props.errors`. Do not return `422` for normal Inertia form validation.

## 4. Repository Shape

The Gurthang source repository should become a Rust workspace:

```text
gurthang/
  Cargo.toml
  Cargo.lock
  README.md
  plan.md

  crates/
    gurthang-cli/
      Cargo.toml
      src/
        main.rs
        cli.rs
        new.rs
        project_name.rs
        renderer.rs
        error.rs

  templates/
    project/
      Cargo.toml.gurthang
      .env.example
      .gitignore.gurthang
      README.md.gurthang
      package.json
      tsconfig.json
      vite.config.ts
      index.html.gurthang-or-equivalent
      src/
      migrations/
      templates/
      resources/
      tests/

  tests/
    generated_project.rs
    fixtures/
```

The generated project should have this application shape:

```text
generated-app/
  Cargo.toml
  Cargo.lock
  .env.example
  .gitignore
  README.md
  package.json
  tsconfig.json
  vite.config.ts

  src/
    main.rs
    app.rs
    config.rs
    error.rs
    routes.rs

    controllers/
      mod.rs
      auth.rs
      dashboard.rs
      pages.rs

    models/
      mod.rs
      user.rs

    services/
      mod.rs
      auth.rs

    views/
      mod.rs
      public.rs
      inertia/
        mod.rs
        shared.rs
        auth.rs
        dashboard.rs

    web/
      mod.rs
      assets.rs
      csrf.rs
      datastar.rs
      tera.rs
      inertia/
        mod.rs
        middleware.rs
        page.rs
        props.rs
        response.rs

  migrations/
    0001_create_users.sql
    0002_create_sessions.sql

  templates/
    inertia.html
    layouts/base.html
    pages/home.html
    fragments/counter.html

  assets/
    css/
      style.css

  bin/
    tailwindcli

  css/
    base.css

  resources/
    js/
      app.tsx
      env.d.ts
      generated/
      Pages/
        Auth/Login.tsx
        Auth/Register.tsx
        Dashboard.tsx

  tests/
    auth.rs
    inertia_protocol.rs
    web.rs
```

The exact module split may be adjusted when Rust visibility or circular dependencies make a slightly different arrangement clearer. Preserve the responsibilities even if filenames move.

## 5. CLI Contract

### 5.1 Commands

The first version exposes only:

```text
gurthang new <name>
gurthang new <name> --path <directory>
gurthang new <name> --dry-run
gurthang --version
gurthang --help
```

Default output path is `./<name>`. When `--path` is provided, the application name still comes from `<name>` while files are written to the explicit path.

Use Clap's derive API for parsing.

### 5.2 Required behavior

- Validate the package/project name before writing.
- Accept ordinary Cargo package names and derive a valid snake_case Rust crate identifier when needed.
- Reject absolute or traversal-based names supplied as `<name>`.
- Refuse an existing non-empty destination.
- Permit an absent destination or an existing empty destination.
- Never overwrite user files.
- Render into a temporary sibling directory first.
- Rename the completed temporary directory into place atomically when possible.
- Clean up only the CLI-created temporary directory after a failure.
- Preserve bytes for non-text assets.
- Substitute only known placeholders in known text files.
- Rename scaffold-specific filenames such as `Cargo.toml.gurthang` and `.gitignore.gurthang` to their final names.
- Perform no package installation, network access, database creation, migration execution, Git initialization, or external command invocation.
- `--dry-run` prints the target and sorted file manifest without writing.
- Successful output prints concise next steps.

Suggested success output:

```text
Created my_app at /path/to/my_app

Next:
  cd /path/to/my_app
  cp .env.example .env
  npm install
  sqlx migrate run
  npm run dev
  cargo run
```

### 5.3 CLI tests

- Help and version output.
- Valid project creation.
- Hyphenated package-name substitution.
- Invalid package name.
- Traversal/path-like name rejection.
- Existing file at destination.
- Existing non-empty directory.
- Existing empty directory.
- Dry-run performs no writes.
- Placeholder substitution leaves no known tokens.
- Expected sorted file manifest.
- Template binary bytes are preserved.
- Failure does not leave a partially generated destination.

## 6. Generated Application Details

### 6.1 Configuration

Environment-backed configuration should include at minimum:

```text
APP_ENV=development
APP_HOST=127.0.0.1
APP_PORT=3000
APP_URL=http://127.0.0.1:3000
DATABASE_URL=postgres://...
SESSION_SECURE=false
VITE_DEV_SERVER_URL=http://127.0.0.1:5173
```

Configuration errors should return clear startup errors rather than panic deep in request handling. Secrets must never be logged.

### 6.2 Application state

Use a cloneable Axum `AppState` containing only long-lived shared dependencies, such as:

- `PgPool`
- application configuration
- Tera environment or a wrapper around it
- Inertia renderer configuration
- asset resolver

Use Axum's typed `State` extractor. Avoid a dependency-injection container.

### 6.3 Database and migrations

Embed and run migrations explicitly from a documented command or startup option. Do not silently mutate schema on every production startup.

`users` table minimum fields:

```text
id             uuid primary key
email          text not null unique
password_hash  text not null
created_at     timestamptz not null
updated_at     timestamptz not null
```

Normalize email to lowercase and trim surrounding whitespace before validation and persistence. Generate UUIDs in the application unless a database default is clearly simpler.

The sessions migration must match the pinned `tower-sessions-sqlx-store` schema and be covered by an integration test.

### 6.4 User model

The user model should provide only operations required by the vertical slice:

- `find_by_id`
- `find_by_email`
- `create`
- password hash generation
- password verification
- validation for email and password inputs

Use explicit input structs, for example:

```rust
pub struct CreateUserData {
    pub email: String,
    pub password: String,
}
```

Do not expose `password_hash` through presentation DTOs or serialized debug output. Implement a custom `Debug` or avoid deriving it on sensitive credential structures.

### 6.5 Authentication routes

Required routes:

```text
GET    /                 Tera public page
GET    /register         Inertia registration page
POST   /register         Create user, rotate/authenticate session, redirect
GET    /login            Inertia login page
POST   /login            Authenticate, rotate session, redirect
DELETE /logout           End session, redirect
GET    /dashboard        Protected Inertia page
GET or POST /demo/...    Minimal Datastar SSE interaction
```

Unauthenticated dashboard access redirects to `/login`. Authenticated users visiting login or registration may redirect to `/dashboard`.

Avoid account-enumeration detail in login errors. Return one generic invalid-credentials message and perform a dummy hash verification when the account does not exist if practical within the proof-of-concept scope.

### 6.6 Inertia v3 minimal server module

Implement and test the features used by the application:

#### Request handling

- Detect `X-Inertia: true`.
- Read the current URL.
- Read `X-Inertia-Version`.
- Read `X-Inertia-Partial-Component`.
- Read `X-Inertia-Partial-Data`.
- Read `X-Inertia-Partial-Except`.
- Read `X-Inertia-Error-Bag` if validation errors support bags.

#### Initial HTML response

- Render a complete HTML document.
- Include frontend assets through the environment-aware asset resolver.
- Emit the page object in the Inertia v3 JSON script element.
- Escape forward slashes inside embedded JSON so user data cannot close the script element.
- Do not HTML-entity encode the JSON script content.
- Provide the configured root mount element.

#### Inertia JSON response

- Return the page object as JSON.
- Set `X-Inertia: true`.
- Set `Vary: X-Inertia` without destroying other `Vary` values.
- Return the correct JSON content type.

#### Page object

Support:

- `component`
- `props`
- `url`
- `version`
- `encryptHistory` only if needed as a fixed false/omitted value
- `clearHistory` only if needed as a fixed false/omitted value

`props.errors` defaults to an empty object.

#### Shared props

At minimum:

- authenticated user or null
- validation errors
- flash message(s)

Shared props should be typed Rust structures and exported to TypeScript where consumed by the UI.

#### Partial reloads

- Apply partial filtering only when `X-Inertia-Partial-Component` matches the target component.
- Honor both include and exclude lists, with exclusions taking precedence.
- It is acceptable for the first version to filter already-resolved serializable props rather than implement lazy closures.

#### Redirects and asset versions

- Convert post-mutation redirects to `303 See Other` where required.
- On asset-version mismatch for an Inertia GET request, return `409 Conflict` with `X-Inertia-Location` set to the current URL.
- Provide a helper for external location responses using the same `409` plus location header convention.

#### Not implemented initially

- Deferred props
- Optional props
- Merge/append/prepend metadata
- Once props
- Infinite scroll metadata
- Precognition
- History encryption
- SSR

Document these omissions in the generated module and README.

### 6.7 Rust-to-TypeScript contracts

Use `ts-rs`.

Page contracts should be concrete structs such as:

```rust
#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoginProps {
    pub email: Option<String>,
}

impl InertiaPage for LoginProps {
    const COMPONENT: &'static str = "Auth/Login";
}
```

Required workflow:

- `cargo test export_bindings` writes bindings under `resources/js/generated`.
- React pages import those bindings rather than redefining interfaces.
- CI/test workflow regenerates bindings before TypeScript checking.
- If generated files are committed, a test/check detects stale output.
- A Rust test or explicit page registry verifies that every component name maps to an existing `.tsx` file.

Do not create a procedural macro initially. If repetition is later demonstrated, a derive such as `#[derive(InertiaPage)]` may implement only the trait and component constant; type export remains owned by `ts-rs`.

### 6.8 React, Vite, and Tailwind

Use:

- React 19
- `@inertiajs/react` v3
- Vite
- `@vitejs/plugin-react`
- Tailwind CSS with `@tailwindcss/vite`
- `@inertiajs/vite` if it materially simplifies v3 page resolution without coupling to a server framework

Development behavior:

- The Axum-rendered initial document loads Vite's development client and application entrypoint.
- React Fast Refresh and Tailwind updates work without restarting Axum.
- `css/base.css` is the single source stylesheet imported by the Inertia entrypoint and compiled for Tera routes.
- A pinned, checksum-verified standalone Tailwind executable is installed at `bin/tailwindcli` by the generated setup workflow.
- The standalone CLI emits `assets/css/style.css` for server-rendered pages and supports a watch mode.
- CORS/dev-server origin handling is explicit and limited to development.
- Tera/backend changes may require a Rust restart initially.
- Document `cargo-watch` as an optional backend restart tool; do not make it a required application dependency.

Production behavior:

- `npm run build` creates hashed assets and a Vite manifest.
- The Rust asset resolver reads the manifest and emits script/style tags.
- Axum serves the built static assets with appropriate content types.
- Missing manifest or entrypoints produce actionable startup/render errors.

Tailwind must scan React sources and Tera templates. Add explicit Tailwind v4 `@source` directives when automatic discovery is insufficient.

### 6.9 Tera and Datastar

The Tera path exists to prove that server-rendered MVC and Inertia can coexist by route group.

Implement:

- A base layout.
- A public home page.
- One fragment template.
- One small Datastar interaction using the official Rust SDK's Axum SSE support.

The interaction can be a counter or similarly isolated demonstration. It must prove:

- reading Datastar signals or request data
- rendering a Tera fragment
- returning a Datastar `PatchElements` SSE event

Do not mix Datastar and Inertia behavior on the same page in the initial implementation.

## 7. Error Handling and Security Baseline

### Error handling

- Define an application error type with `IntoResponse`.
- Keep internal errors out of production responses.
- Log errors through `tracing` with request context where available.
- Distinguish not-found, validation, authentication, CSRF, database, template, asset, and internal failures.
- Render appropriate Tera or Inertia errors based on the request mode where practical.

### Security requirements

- No plaintext passwords or reversible password storage.
- Argon2id PHC strings with per-password random salts.
- Hash/verify work off the async executor.
- Parameterized SQL only.
- Email normalization plus unique constraint.
- Session fixation protection through identifier rotation.
- Persistent sessions stored server-side.
- Authentication cookie is `HttpOnly` and `SameSite=Lax`; `Secure` in production.
- CSRF token cookie/header verification on all state-changing web routes.
- Sensitive values are not logged.
- Initial Inertia page JSON is safe against `</script>` termination.
- CLI does not overwrite existing files.
- Production errors do not expose database or template internals.

Rate limiting is desirable but not required for the first proof of concept. Document its absence.

## 8. Testing Strategy

### 8.1 CLI tests

Cover the CLI contract in Section 5.3 using temporary directories. Tests must never write outside their assigned temporary directory.

### 8.2 Generated project structural test

Generate a project into a temporary directory and assert:

- exact or intentionally normalized file manifest
- no remaining scaffold placeholders
- expected Cargo package name and Rust crate identifier
- expected npm package name
- migrations and application modules exist
- generated documentation contains working commands

### 8.3 Generated project compilation checks

At an appropriate integration level:

- `cargo fmt --check`
- `cargo check`
- `cargo clippy --all-targets --all-features -- -D warnings` when practical
- `cargo test`
- TypeScript binding export
- `npm run typecheck`
- `npm run build`

Avoid network-dependent checks in normal unit tests. A repository bootstrap step may install dependencies before these checks.

### 8.4 Inertia protocol tests

Use Axum/Tower service tests to cover:

- initial non-Inertia HTML response
- safe JSON embedding and slash escaping
- subsequent Inertia JSON response
- component, props, URL, and version fields
- empty errors object
- shared auth/flash/errors props
- `X-Inertia` and `Vary` headers
- partial include filtering
- partial exclude filtering
- mismatched component ignores partial filtering
- mutation redirect becomes 303
- version mismatch becomes 409 location response
- external location response
- missing React component registry failure

### 8.5 Authentication tests

Use a disposable PostgreSQL database or transaction-isolated database test setup to cover:

- user creation and normalization
- duplicate email rejection
- password is stored as Argon2id PHC rather than plaintext
- successful password verification
- invalid password verification
- registration authenticates and redirects
- login succeeds and rotates/authenticates session
- nonexistent account receives generic failure
- invalid password receives the same generic failure
- unauthenticated dashboard redirects
- authenticated dashboard returns Inertia page with safe user DTO
- logout clears authentication
- session persists across requests
- missing or invalid CSRF token is rejected
- valid XSRF cookie/header pair succeeds
- password hash is never serialized to frontend props

### 8.6 Tera/Datastar tests

- Public page renders through Tera.
- HTML escaping is active.
- Datastar endpoint returns `text/event-stream`.
- SSE payload is a valid element patch containing the rendered fragment.

## 9. Development Workflow

The generated README should document a direct setup:

```bash
cp .env.example .env
./bin/install-tailwindcli
npm install
npm run css:build
sqlx migrate run
```

Run the Tailwind watcher, frontend, and backend in separate terminals:

```bash
npm run css:dev
```

```bash
npm run dev
```

```bash
cargo run
```

Optional backend auto-restart:

```bash
cargo watch -x run
```

Contract synchronization:

```bash
cargo test export_bindings
npm run typecheck
```

Production-style frontend check:

```bash
npm run build
cargo run --release
```

Do not hide these commands behind a large Gurthang command surface in the proof of concept. The experiment should expose the actual Rust, SQLx, npm, and Vite workflows being evaluated.

## 10. Iteration-Speed Evaluation

Create `docs/iteration-log.md` in the generated project or the Gurthang repository. Record environment details and at least:

- clean/cold `cargo check`
- warm no-change `cargo check`
- incremental controller-only change
- incremental model/query change
- incremental Inertia props DTO change plus binding export/typecheck
- React component-only change through Vite HMR
- Tailwind class-only change
- Tera template-only change
- migration plus model change
- full Rust test suite
- frontend typecheck
- production frontend build

For each representative change record:

- wall-clock time
- commands required
- number of files intentionally touched
- whether a database had to be running
- whether a manual regeneration step was required
- quality and locality of compiler/runtime error messages

The proof of concept is successful only if it produces enough evidence to judge the workflow, even if the conclusion is that Rust iteration is not acceptable.

## 11. Implementation Milestones

### Milestone 1: CLI skeleton

- Initialize workspace and CLI crate.
- Add Clap parser and `new` command types.
- Add project-name validation.
- Embed a minimal template directory.
- Implement safe/dry-run generation.
- Add CLI filesystem tests.

Exit criteria: `gurthang new demo` creates a minimal compilable placeholder project without overwriting existing content.

### Milestone 2: Generated Axum MVC shell

- Add the full generated directory structure.
- Add configuration, tracing, Axum state, routing, Tera, static assets, and errors.
- Add PostgreSQL pool and migrations.

Exit criteria: generated app starts, connects to PostgreSQL, renders a Tera home page, and has a clear README.

### Milestone 3: Frontend asset pipeline

- Add React, Vite, Tailwind, and development asset resolver.
- Add production manifest resolver and static serving.
- Prove Fast Refresh and production build.

Exit criteria: an Axum-delivered document boots React in development and production asset modes.

### Milestone 4: Typed Inertia v3 core

- Add `InertiaPage` and page-object types.
- Add initial HTML and JSON response paths.
- Add shared props, partial filtering, redirects, and asset versions.
- Add `ts-rs` export and React page registry.
- Add protocol tests.

Exit criteria: typed dashboard placeholder navigates through Inertia v3, and changing its Rust props produces a frontend type error until React is updated.

### Milestone 5: Users and authentication

- Add users and sessions migrations.
- Add user model and safe user view.
- Add password hashing and authentication service/backend.
- Add registration, login, logout, protected dashboard, sessions, errors, flash, and CSRF.
- Add database-backed integration tests.

Exit criteria: a user can register, remain authenticated across requests, log out, and be denied protected access afterward.

### Milestone 6: Datastar vertical slice

- Add public Tera fragment and Datastar SSE endpoint.
- Add one browser interaction and tests.

Exit criteria: the public page updates a server-rendered fragment through a Datastar element patch.

### Milestone 7: Generated-project verification and measurement

- Scaffold from the actual CLI into a clean temporary directory.
- Run formatting, Rust checks/tests, binding export, frontend typecheck, and frontend build.
- Fix generated instructions and error messages.
- Record iteration measurements.

Exit criteria: the CLI creates the tested application from embedded templates, all agreed checks pass, and the iteration log contains representative results.

## 12. Definition of Done

The proof of concept is complete when all of the following are true:

1. `gurthang new sample_app` creates the complete project without external commands or network access.
2. The generated project has the documented MVC boundaries and contains no unresolved scaffold tokens.
3. PostgreSQL migrations create users and persistent session storage.
4. The public Tera page renders and its Datastar interaction returns a working SSE patch.
5. React uses the shared root Tailwind source through Vite, while Tera uses the standalone Tailwind CLI output with CSS watch mode in development.
6. Production frontend assets build and are resolved from the Vite manifest.
7. Inertia v3 initial visits and subsequent visits work for the implemented protocol subset.
8. Rust page DTOs are the source of truth for generated TypeScript page contracts.
9. Registration, login, protected dashboard access, and logout work with secure session and CSRF behavior.
10. Passwords are hashed with Argon2id and sensitive model fields never reach frontend props.
11. CLI, protocol, auth, and Datastar tests pass.
12. A clean generated project passes Rust formatting/check/tests and frontend typecheck/build.
13. The implemented Inertia v3 omissions are clearly documented.
14. Iteration-speed measurements are recorded well enough to inform whether the experiment should continue.

## 13. Guidance for a Fresh Implementing Agent

Treat this file as the source of truth. Begin with Milestone 1 and proceed in order unless a later requirement forces a small structural adjustment. Favor the smallest implementation that satisfies the current milestone and preserves the boundaries above.

Before changing files:

- Inspect the workspace and any `AGENTS.md` instructions.
- Preserve pre-existing user changes.
- Confirm currently compatible dependency versions from primary documentation or Cargo resolution.

While implementing:

- Keep the plan's scope exclusions intact.
- Do not add generators beyond `gurthang new`.
- Do not create framework abstractions without a demonstrated use in the generated app.
- Add tests alongside each milestone rather than postponing all verification.
- Update this plan only when an implementation discovery materially changes a documented decision; record the reason.

At handoff:

- Report completed milestones.
- Report exact verification commands and results.
- List any remaining plan items or intentional deviations.
- Keep `docs/iteration-log.md` current once timing work begins.
