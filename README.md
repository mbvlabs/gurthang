# Gurthang

Gurthang is a proof-of-concept Rust project initializer for testing the
iteration speed of an Axum application with explicit MVC boundaries. It
generates a self-contained application using PostgreSQL, SQLx, Tera, Datastar,
React, Tailwind CSS, and a typed Inertia.js v3 server adapter.

The goal is to evaluate the development experience, not to reproduce Andurel's
full framework and generator surface.

## Generate an application

From this repository:

```bash
cargo install --path crates/gurthang-cli
gurthang new my-app
cd my-app
cp .env.example .env
./bin/install-tailwindcli
npm install
npm run css:build
sqlx migrate run
```

Start the backend, Vite, and Tailwind watchers together:

```bash
gurthang run
```

`gurthang run` (alias `gurthang r`) owns the development lifecycle. It keeps
Vite and Tailwind running, restarts only the Cargo backend after Rust changes,
and cleans up every child process on exit. `cargo run` remains the raw Axum
server command.

The Tailwind installer pins the standalone CLI and verifies its SHA-256 digest
before writing `bin/tailwindcli`. It supports Linux and macOS on x86-64 and
ARM64.

## Generated layout

```text
my-app/
├── assets/css/              # Compiled CSS served to Tera/Datastar pages
├── bin/                     # Tailwind CLI installer and local executable
├── css/base.css             # Shared Tailwind source
├── migrations/              # Users and persistent sessions
├── resources/js/            # Inertia React entrypoint, pages, and TS contracts
│   ├── app.tsx
│   ├── generated/
│   └── Pages/
│       ├── Auth/
│       │   ├── Login.tsx
│       │   └── Register.tsx
│       └── Dashboard.tsx
├── src/
│   ├── controllers/
│   ├── models/
│   ├── services/
│   ├── views/               # Typed presentation DTOs and page contracts
│   └── web/                 # Inertia, Tera, Datastar, assets, and CSRF
├── templates/               # Tera layouts, pages, and fragments
└── tests/
```

`css/base.css` is the only authored application stylesheet. Vite imports it for
the Inertia React application, while `bin/tailwindcli` compiles it to
`assets/css/style.css` for server-rendered routes.

The source repository stores this application under `templates/project/`.
Running `gurthang new` copies it into the generated project's root and replaces
the scaffold placeholders.

## Typed Inertia pages

Rust structs under `src/views/inertia` define page prop contracts and implement
an `InertiaPage` trait containing the component name. `ts-rs` exports those
contracts to `resources/js/generated`, where the React pages consume them.

The generated project uses the official `@inertiajs/react` v3 client. Its local
Rust server adapter currently implements the protocol subset exercised by the
proof of concept: initial and subsequent visits, shared props, partial
include/exclude reloads, redirects, and asset-version refreshes.

It is not yet a complete Inertia v3 server adapter. Deferred, optional, merged,
once and infinite-scroll props, Precognition, history encryption, and SSR are
not implemented.

## Verification

Check the project initializer with:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

After generating and configuring an application, its primary checks are:

```bash
cargo test
npm run typecheck
npm run build
```

PostgreSQL integration tests run when `TEST_DATABASE_URL` points to a disposable
database. See `plan.md` for the implementation scope and
`docs/iteration-log.md` for the recorded iteration measurements.
