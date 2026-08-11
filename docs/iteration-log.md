# Gurthang iteration log

## Environment

- Recorded: 2026-08-11
- Platform: Linux
- Rust: rustc 1.97.1, cargo 1.97.1
- Node.js: 26.2.0
- npm: 11.13.0
- Database required for these measurements: no

## Baseline observations

| Scenario | Wall time | Command | Intentional files | Database | Regeneration | Result |
| --- | ---: | --- | ---: | --- | --- | --- |
| CLI workspace tests, warm | 0.7 s | `cargo test --workspace` | 0 | no | no | 13 tests passed; errors were local and actionable during development |
| Generated app tests, cold dependency build | ~29 s | `cargo test` | 0 | no | bindings exported by tests | 17 tests passed after dependencies compiled |
| Generated app tests, warm | 1.8 s | `cargo test` | 0 | no | bindings exported by tests | 17 tests passed |
| Frontend dependency install | ~22 s | `npm install` | 0 | no | no | 86 packages installed; zero reported vulnerabilities |
| Frontend typecheck plus production build, warm | 3.6 s | `npm run typecheck && npm run build` | 0 | no | bindings already current | typecheck passed; Vite built 565 modules in 1.30 s |
| Generated app clippy, warm | 0.9 s | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | no | no | passed |

## Vertical-slice observations

| Scenario | Wall time | Command/workflow | Files | Database | Regeneration | Observation |
| --- | ---: | --- | ---: | --- | --- | --- |
| Clean generated app check | 32.6 s | `cargo check` | 0 | no | no | First build after dependency fetch; compiler feedback was local |
| Warm no-change app check | <1 s | `cargo check` | 0 | no | no | Immediate success |
| Controller-only change | ~1 s | edit handler; `cargo check` | 1 | no | no | Extractor/type errors point to the handler |
| Model/query change | ~1 s | edit query; `cargo check` | 1 | no | no | Dynamic SQL needs the PostgreSQL integration suite for validation |
| DTO contract change | ~2 s | `cargo test export_bindings`; `npm run typecheck` | 2 | no | yes | Stale-binding test and TypeScript consumer make drift visible |
| React/Tailwind HMR | not browser-timed | edit under `npm run dev` | 1 | no | no | Pipeline is configured; browser timing remains machine/operator work |
| Tera template | restart required | edit template; restart backend | 1 | no | no | Templates load at startup; error includes template context |
| Migration + model | not DB-timed | migrate; run `TEST_DATABASE_URL=... cargo test` | 2 | yes | no | Disposable PostgreSQL was unavailable in this environment |
| Generated Rust suite, warm | ~2 s | `cargo test` | 0 | no* | yes | 30 tests; DB-only tests are gated on `TEST_DATABASE_URL` |
| Frontend typecheck + build | 4.1 s | `npm run typecheck && npm run build` | 0 | no | no | Typecheck passed; Vite built 567 modules in 1.42 s |

`*` The full register/session/logout and schema tests execute when a disposable
`TEST_DATABASE_URL` is provided. The explicitly untimed HMR and database rows
are the remaining local-environment measurements, rather than inferred numbers.
