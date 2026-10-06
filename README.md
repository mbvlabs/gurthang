# Gurthang

Gurthang is a Linux-only project pad for Rails-shaped Rust web apps: Axum,
SQLx, Inertia React, and an application-owned PostgreSQL job queue.

It stands up an app and then grows it the way a pad should: write a migration,
generate a model, scaffold a resource, enqueue a job, migrate, and build.

```text
gurthang new <name> [--path DIR] [--dry-run]
gurthang run | r

gurthang generate migration <name>
gurthang generate model <Name>
gurthang generate controller <Name> [actions...]
gurthang generate scaffold <Name>
gurthang generate job <Name>

gurthang sync model <Name> [--check]
gurthang sync routes
gurthang sync payloads
gurthang sync factory <Name> [--check]
gurthang sync factories --check|--sync

gurthang db create | drop | nuke | rebuild
gurthang db migrate up | status
gurthang db seed [name] [--list]

gurthang build
```

The generated app is MVC (`models`, `controllers`, `views`, `services`, `jobs`)
plus an Andurel-style `routes/` layer that only names and wires URLs. Stable
protocol code lives in Gurthang crates (`gurthang-inertia`, `gurthang-jobs`,
`gurthang-http`). Controllers never own SQL.

`query!` / `query_as!` live in a separate `models` crate so a controller edit
does not rebuild database macros. Generate and sync refresh offline SQLx data
with `cargo sqlx prepare` and keep `SQLX_OFFLINE=true` in `.env`.

Linux only. Builds use [Mold](https://github.com/rui314/mold) through clang
(`-fuse-ld=mold`).

## Install

Development releases are a single GitHub prerelease on the `development` tag.
Install the CLI from [Releases](https://github.com/mbvlabs/gurthang/releases):

```bash
cargo install --git https://github.com/mbvlabs/gurthang --tag development --locked gurthang-cli
```

Reinstall with `--force` after that tag moves.

That build needs `clang` and `mold` on the machine (see Requirements). The
binary it installs is `gurthang`.

A Linux x86_64 binary is attached to each release as
`gurthang-x86_64-unknown-linux-gnu` if you would rather skip compiling.

From this tree, while working on Gurthang itself:

```bash
cargo install --path crates/gurthang-cli --locked --force
```

`gurthang new` path-depends generated apps on the Gurthang source used to
compile the CLI. Use `--path` from a clone when you want those apps to follow
this working tree.

## Requirements

- Linux
- A recent Rust toolchain with the 2024 edition
- `clang` and `mold`
- PostgreSQL and the SQLx CLI (`cargo install sqlx-cli`)
- Node.js 22 or newer and npm

## Quick start

```bash
gurthang new my-app
cd my-app
cp .env.example .env
npm install
gurthang db create
gurthang db migrate up
gurthang run
```

Open <http://127.0.0.1:3000>. `/` is an Inertia Welcome page. Register and
sign in at `/register` and `/login`. `/dashboard` is an authenticated SSR page.

`gurthang db migrate up` applies SQLx migrations and runs `cargo sqlx prepare`.
Day-to-day checks use `SQLX_OFFLINE=true`.

## Development releases

A development version is one moving `development` tag, not a crates.io publish.
Point that tag at the commit you want and push it:

```bash
git tag -f development
git push -f origin development
```

The workflow tests the workspace, builds `gurthang`, and creates or updates the
prerelease with the Linux binary attached. A push to `master` does not publish.

## Generated layout

```text
my-app/
  Cargo.toml              # workspace: app package + models crate
  gurthang.toml
  models/                 # query! / query_as! live only here
    src/user.rs
    .sqlx/
  src/
    main.rs               # thin
    lib.rs
    controllers/
    routes/               # paths, names, wiring
    views/
    services/
    jobs/
  resources/js/
  migrations/
```

There is no Tera, Datastar, scaffold `tests/` tree, or Tailwind standalone CLI.
Vite owns CSS.

## Growing the app

After a migration:

```bash
gurthang db migrate up
gurthang generate model Widget
gurthang generate scaffold Widget
```

`generate` / `sync` of models and jobs run `cargo sqlx prepare` by default.

```bash
gurthang generate job SendWelcome
gurthang sync routes
gurthang sync payloads
gurthang build
```

## Verification

```bash
cargo test --workspace
```
