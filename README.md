# Gurthang

Gurthang is a proof-of-concept project initializer for a small, Rails-shaped
Rust web application. It generates a self-contained Axum application with
explicit MVC boundaries, PostgreSQL persistence, server-rendered and React
frontends, authentication, and durable background jobs.

The project currently consists of two pieces:

- `gurthang new` renders the application embedded in this repository into a new
  directory. The generated code is owned by the application; there is no
  Gurthang runtime dependency.
- `gurthang run` supervises the generated application's Rust backend, Vite
  server, and Tailwind watcher during development.

Gurthang is intended to evaluate the development experience and iteration
speed of this stack. It is not a general-purpose framework or a complete
replacement for Andurel's generators.

## What the generated application includes

- Axum and Tokio with a conventional `models`, `services`, `controllers`,
  `views`, and `web` structure.
- PostgreSQL and SQLx, with explicit migrations and application-owned queries.
- A public Tera page and a Datastar counter rendered as a `PatchElements` SSE
  response.
- React 19, Vite 7, Tailwind CSS 4, and the official Inertia React v3 client.
- A local typed Inertia v3 server adapter. Rust page DTOs define both the
  component and rendering mode, and `ts-rs` exports their TypeScript contracts.
- Per-page Inertia SSR through a supervised Node.js or Bun process. The
  dashboard uses SSR; login and registration use client rendering.
- Registration, login, logout, a protected dashboard, Argon2id password hashes,
  PostgreSQL-backed sessions, and XSRF protection.
- An application-owned PostgreSQL job queue with typed payloads, scheduled
  execution, concurrent workers, expiring leases, bounded exponential retries,
  `LISTEN`/`NOTIFY` wake-ups, and polling fallback.
- Development template reload and browser refresh, React Fast Refresh, and
  backend restart after Rust or configuration changes.
- Production builds that embed browser assets, compiled CSS, Tera templates,
  and the Inertia SSR bundle in the Rust executable.

## Requirements

- A recent Rust toolchain with Rust 2024 edition support.
- PostgreSQL and the SQLx CLI (`sqlx`).
- Node.js 22 or newer and npm. Bun can replace Node only as the Inertia SSR
  runtime.
- `curl` and either `sha256sum` or `shasum` for the Tailwind installer.

The pinned standalone Tailwind CLI installer supports Linux and macOS on x86-64
and ARM64.

## Quick start

Install the CLI from this repository:

```bash
cargo install --path crates/gurthang-cli
```

Create a PostgreSQL database for the application, then generate and configure
the project:

```bash
gurthang new my-app
cd my-app
cp .env.example .env
./bin/install-tailwindcli
npm install
npm run css:build
sqlx migrate run
```

Update `DATABASE_URL` and the other values in `.env` as needed. Migrations are
always explicit; the application does not run them at startup.

Start the complete development environment:

```bash
gurthang run
```

Open <http://127.0.0.1:3000>.

`gurthang run` can be invoked from the project root or one of its subdirectories.
It starts `npm run dev`, `npm run css:dev`, and `cargo run`; restarts only the
backend after changes to Rust files, `Cargo.toml`, or `.env`; and terminates all
child process groups on exit. A compiler error leaves the supervisor running so
the next Rust edit can retry the backend.

Tera templates reload inside the running backend and refresh the browser. React
and shared Tailwind changes are handled by Vite and React Fast Refresh.

## CLI

```text
gurthang new <NAME> [--path <DIRECTORY>] [--dry-run]
gurthang run
gurthang r
```

`new` accepts ASCII letters, digits, hyphens, and underscores, renders through a
temporary directory, and refuses to overwrite files or non-empty directories.
`--path` selects a destination independent of the project name. `--dry-run`
prints the destination and sorted file manifest without writing anything.

## Generated routes

| Method | Path | Implementation |
| --- | --- | --- |
| `GET` | `/` | Public Tera page |
| `GET` | `/demo/counter` | Datastar SSE counter |
| `GET`, `POST` | `/register` | Client-rendered Inertia registration |
| `GET`, `POST` | `/login` | Client-rendered Inertia login |
| `DELETE` | `/logout` | Session logout |
| `GET` | `/dashboard` | Authenticated, server-rendered Inertia page |

The session cookie is `HttpOnly`, `SameSite=Lax`, and controlled by
`SESSION_SECURE`. The readable `XSRF-TOKEN` cookie must match the
`X-XSRF-TOKEN` header for state-changing requests. Set `SESSION_SECURE=true`
when deploying behind HTTPS. Rate limiting is not included and should be added
before exposing the authentication endpoints to untrusted traffic.

## Generated project layout

```text
my-app/
├── assets/css/              # Compiled CSS for server-rendered pages
├── bin/                     # Verified Tailwind CLI installer and executable
├── css/base.css             # Shared Tailwind source
├── migrations/              # Users, sessions, and background jobs
├── resources/js/            # Inertia React client, SSR entry, pages, TS types
├── src/
│   ├── controllers/         # Axum request and response handling
│   ├── jobs/                # Typed PostgreSQL queue and worker
│   ├── models/              # Database-shaped entities and queries
│   ├── services/            # Transactions and multi-step workflows
│   ├── views/               # Presentation DTOs and page contracts
│   └── web/                 # Assets, CSRF, Datastar, Inertia, SSR, and Tera
├── templates/               # Tera layouts, pages, and fragments
└── tests/                   # HTTP, authentication, and job tests
```

The source scaffold lives in `templates/project/`. The CLI embeds that directory
at compile time, copies it into the destination, removes `.gurthang` suffixes,
and substitutes project, Cargo crate, and package names.

`css/base.css` is the only authored application stylesheet. Vite imports it for
the React application, while the standalone Tailwind watcher compiles it to
`assets/css/style.css` for Tera and Datastar routes.

## Inertia and SSR

Rust structs under `src/views/inertia` implement `InertiaPage`, which associates
serialized props with a React component and either client or server rendering.
Those structs derive `ts-rs::TS`; their committed TypeScript bindings live under
`resources/js/generated`.

The local adapter implements the protocol subset used by the scaffold:

- initial HTML and subsequent JSON visits;
- shared props and validation errors;
- partial include and exclude reloads;
- mutation redirects and external locations;
- asset-version refreshes; and
- optional React SSR on initial visits.

SSR starts lazily when an SSR page is first requested. The default runtime is
Node.js 22 or newer; set `INERTIA_SSR_RUNTIME=bun` to use Bun. Worker startup and
rendering use `INERTIA_SSR_TIMEOUT_MS`, and failures fall back to normal client
rendering. During development, build `dist-ssr/ssr.mjs` before visiting an SSR
page:

```bash
npm run build:ssr
```

Deferred, optional, merged, once, and infinite-scroll props, Precognition, and
history encryption are not implemented.

## Background jobs

`cargo run` starts the web server and job worker together. The same application
artifact also supports isolated roles:

```bash
cargo run -- web
cargo run -- worker
cargo run -- all
```

Jobs are Serde-tagged variants in `src/jobs/mod.rs`. They can be enqueued
immediately, scheduled for later, or inserted on an existing SQLx transaction
so the application change and job commit atomically. Delivery is at least once,
so handlers must be idempotent.

The worker is configured with `JOB_WORKERS`, `JOB_POLL_INTERVAL_MS`,
`JOB_LEASE_SECONDS`, and `JOB_TIMEOUT_SECONDS`. The job timeout must be shorter
than the lease. Completed and permanently failed rows are retained for
inspection; the scaffold does not include an automatic retention policy.

## Production build

From a generated application:

```bash
npm run release
APP_ENV=production VITE_DEV_SERVER_URL= target/release/my-app
```

Run `npm run release` whenever frontend assets or templates change so they are
embedded again. A deployment still needs PostgreSQL and, when it serves an SSR
page, Node.js or Bun. Database migrations remain a separate deployment step.

The production process accepts `all` (the default), `web`, or `worker`, which
allows the web and job-worker roles to be scaled independently:

```bash
target/release/my-app web
target/release/my-app worker
```

## Verification

Check the CLI workspace with:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

After generating and configuring an application, run:

```bash
cargo test
npm run typecheck
npm run build
cargo clippy --all-targets --all-features -- -D warnings
```

`cargo test export_bindings` regenerates the Rust-authored TypeScript contracts.
PostgreSQL integration tests run when `TEST_DATABASE_URL` points to a disposable
database; without it, database-only tests report that they were skipped while
unit and HTTP tests continue to run.

See `plan.md` for the original proof-of-concept scope and architectural
decisions, and `docs/iteration-log.md` for the recorded iteration measurements.
