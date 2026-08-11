# Iteration log

Initial Gurthang measurements were recorded on Linux with Rust 1.97.1, Node.js
26.2.0, and npm 11.13.0. Replace or extend these values on the deployment
machine; especially repeat the HMR rows in a real browser session.

| Scenario | Wall time | Commands | Files touched | Database running? | Regeneration? | Error quality |
| --- | ---: | --- | ---: | --- | --- | --- |
| Clean/cold `cargo check` | 32.6 s | `cargo check` | 0 | no | no | Dependency/build errors were local; first run included dependency compilation |
| Warm no-change `cargo check` | <1 s | `cargo check` | 0 | no | no | Immediate success |
| Controller-only change | ~1 s | edit controller; `cargo check` | 1 | no | no | Rust diagnostic points at handler/extractor mismatch |
| Model/query change | ~1 s | edit model SQL; `cargo check` | 1 | no | no | SQL is runtime-checked by integration tests, not compile time |
| Inertia DTO plus binding/typecheck | ~2 s | edit DTO; `cargo test export_bindings`; `npm run typecheck` | 2 | no | yes | TypeScript points at the consuming React prop |
| React component through HMR | not browser-timed | edit `.tsx` while `gurthang run` runs | 1 | no | no | Vite Fast Refresh is automatic; repeat with browser DevTools |
| Shared Tailwind CSS | not browser-timed | edit `css/base.css` while `gurthang run` runs | 1 | no | no | Vite updates Inertia and the compiled Tera stylesheet triggers browser refresh; repeat with browser DevTools |
| Tera template change | not browser-timed | edit `.html` while `gurthang run` runs | 1 | no | no | Valid templates replace the in-memory set and refresh the browser; invalid edits retain the last valid set |
| Migration plus model change | not DB-timed | add migration + edit model; `sqlx migrate run`; `cargo test` | 2 | yes | no | `TEST_DATABASE_URL` tests validate runtime SQL/schema |
| Full Rust test suite, warm | ~2 s | `cargo test` | 0 | no* | bindings exported by tests | 32 tests; DB tests skip unless `TEST_DATABASE_URL` is set |
| Frontend typecheck | ~2 s | `npm run typecheck` | 0 | no | bindings must be current | Direct TypeScript errors |
| Production frontend build | 1.4 s | `npm run build` | 0 | no | no | 567 modules; manifest and hashed assets emitted |

`*` Set `TEST_DATABASE_URL` to include the PostgreSQL integration and full
authentication-flow tests. The untimed browser/database rows are explicit gaps,
not zero-cost claims; they should be the first measurements repeated locally.
