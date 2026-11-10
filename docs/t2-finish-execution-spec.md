# Orchestrator entry point: finish, verify, review, and commit T2

## 1. Objective and authority

Complete the pending type-driven parsing milestone in Gurthang, demonstrate it in generated registration/login, ship the request-generation teaching workflow, verify actual generated output, and make a local commit after final review. This is an execution spec, not an invitation to redesign the milestone.

Approved human-facing context:

- `docs/t2-finish-review.md`: approved through Plannotator on 2026-11-10; its four questions were skipped, not resolved by that approval.
- `docs/t2-finish-decisions.md`: revised finish-and-commit package explicitly approved through Plannotator on 2026-11-10 with no annotations. This settles the four defaults and commit boundary. The earlier still-open four-question review is superseded; any later feedback on that older session must be reconciled, not silently treated as authority over this approval.
- Review evidence at planning time: `/tmp/opencode/gurthang-t2-finish-review.log` and `/tmp/opencode/gurthang-t2-finish-and-commit.log`. They are local evidence, not required runtime inputs; preserve the approval facts above if logs disappear.

**Subsequent authorization:** the user requested “finish the spec, commit using conventional commits and push.” This authorizes committing and pushing this planning package to the existing `origin/master`, separately from unfinished implementation. It does not make the T2 implementation verified or authorize publishing its eventual commit. The implementation's local-commit boundary and final verification/review requirements below remain in force.

The user explicitly requested finishing and committing the current work, then moving to the Andurel v2-rc.1 vision and exploring a type-driven `gurthang verify` command, potentially using macros. **That next milestone is a follow-up investigation, not part of T2 implementation or this commit.** Do not implement `verify`, introduce macros for it, or reinterpret an unavailable Andurel vision.

### Repository checkpoint (revalidate before acting)

- Repository and only observed worktree: `/home/mbv/work/gurthang`.
- Branch: `master`; implementation baseline: `7592187fe58a1c54bf17f22f78da6a3c7c33703f` (middleware PR #2 merge). The planning package is committed afterward; a documentation-only descendant is expected, not an implementation-baseline mismatch. Record the actual execution HEAD separately.
- No new execution worktree or branch has been created or authorized. Execute in this existing worktree unless the user separately authorizes a change. Never switch away with dirty work or copy only tracked changes to another checkout.
- Initially modified: workspace manifest, facade manifest, facade library exports and prelude. Initially untracked: `crates/gurthang-parse/`, facade `src/invalid.rs`, three T2 draft plans, and `docs/inertia-omega-vs-gurthang.md`.
- During investigation, an inspection agent ran unlocked Cargo tests and added five lockfile lines: the facade's local parsing dependency and its zero-dependency package entry. That delta was retained and disclosed; validate it rather than reverting blindly.
- Planner added the two approval documents and this execution spec. No implementation has been completed by this planning session.
- No staged changes were observed. Treat any newly staged work, changed HEAD, or additional user edits as mutable state requiring reconciliation.

### Source-of-truth precedence

1. Current project instructions and user authorization boundaries.
2. The two approved finish documents, with the later decisions document taking precedence; this spec concretizes them.
3. `docs/t2-validation-plan.md`, `docs/t2-cli-plan.md`, `docs/t2-implementation-plan.md` for consistent details only. Their unanswered/recommended options are historical, not additional decisions.
4. `docs/adr/framework-shape.md`, `docs/lifecycle.md`, `docs/t1-finish-plan.md` for established framework boundaries and preceding milestone limitations.
5. Existing source/tests for regression contracts. `docs/roadmap.md` and root `plan.md` contain superseded scope and must not expand this task.

## 2. Locked decisions and behavioral contract

- Browser input is `*Payload`; parsed domain write inputs are `*Data`. Keep model associated functions, not free-function SQL.
- `gurthang-parse` is a zero-dependency leaf with `ErrorBag`; facade re-exports it as `parse`, and exports `ErrorBag` and `invalid` at root/prelude. Models do not depend on the parsing crate or HTTP.
- ErrorBag uses ordered string maps: `field` collects a parse failure's Display message and returns `None`; success returns `Some(T)`. First error per field wins. Old input is opt-in via `keep_old`/builders; emptiness refers to errors, not retained input. Preserve existing public accessors/builders and doctest intent.
- `invalid(&Session, destination: impl AsRef<str>, ErrorBag) -> gurthang::Result<Response>` writes the map at `errors`, retained values at `old.<field>`, then returns the established 303 mutation redirect. Session write failures propagate; do not issue a success redirect after a failed write. Do not add transactional/session rollback semantics or a generic old-input lifecycle system.
- Only Email and password newtypes' parsers construct them; fields private, no mutable inner accessor or unchecked String conversion. `User.email` remains String. SQL text/schema and `.sqlx` metadata remain unchanged.
- Email parses an owned String: trim and lowercase, normalized length at most 254 bytes (preserve existing length convention), exactly one `@`, nonempty local/domain, no internal whitespace. This is deliberately not an RFC email validator; do not add dot/TLD/network checks. Message: `Enter a valid email address.`
- NewPassword: nonempty, at least 12 Unicode characters; messages `Password is required.` / `Password must be at least 12 characters.` PasswordAttempt: nonempty only, same required message. Never trim or normalize passwords. Debug redacts both password types. Read-only access needed for hashing may use `AsRef<str>`; no new secret logging/serialization.
- `UserCreateData { email: Email, password: NewPassword }`; `User::create(&mut PgConnection, &Email, password_hash: String)`, `find_by_email(&PgPool, &Email)`, `hash_password(&NewPassword)`, `verify_password(&PasswordAttempt)`. Clone owned secret bytes/string inside the model before `spawn_blocking`, not a borrowed value escaping its lifetime. Retain Argon2 and off-executor hashing.
- Login parse failures show field errors; unknown account and wrong password both retain `The email or password is incorrect.` Preserve dummy verification for unknown accounts. Login has no new minimum length restriction.
- Factory `data()` returns UserCreateData directly and fails with a clear fixture-specific panic if configured strings cannot parse. It must not bypass constructors.
- Shared JSON/form reading stays in generated `controllers/shared.rs`, not a public facade API. Preserve the existing 64 KiB body limit, content-type detection, malformed-body 400 behavior, and both body formats. Missing payload fields default to empty strings and produce collected field errors rather than a deserialization 400.
- Generated request stubs contain an unresolved domain `<Name>Data` plus `compile_error!` explaining that parsing must be implemented. Do not ship a `todo!()`-only stub. Removing the blocker and supplying a real domain conversion makes the app compile; generation itself does not infer business rules.
- Do not rewrite model/sync/scaffold CRUD bodies into typed domain generators. Teach through help/output, request generation, AGENTS guidance, README and scaffold comments. General generated schema DTOs remain schema types; clearly document that limitation.

## 3. Permissions and prerequisites

Allowed under the approved task: targeted implementation, tests, docs, generated verification fixtures in temporary directories, Cargo lockfile update for in-scope dependencies, and a local Conventional Commit after verification and final approval of the diff. Commit on the existing branch only after revalidating it. Do not use `git add .` or include unrelated work.

Not authorized: push, force-push, tags/releases, deployment, history rewriting, branch/worktree changes, destructive cleanup, database create/migrate/reset/seed against any live database, global tool installation/configuration, or reading secrets. Never run `just rd` / `just release-development`: they force-update and push the development tag. Ask before npm dependency download/install if no usable dependencies exist; use only a disposable generated app and explain that package install executes scripts. Do not run framework tool synchronization as an implicit installation step.

Read applicable AGENTS instructions before editing, including `/home/mbv/.dotfiles/opencode/.config/opencode/AGENTS.md`; no repository-local AGENTS was found at planning time. Use dedicated read/patch tools for file work, not inline programming-language scripts. Do not read `.env`, credentials, or secret-valued environment variables. Public template examples are not credentials, but no copying a real `.env` is required for offline tests.

**Known blocker:** `.cargo/config.toml` configures clang with `-fuse-ld=mold` for Linux. Neither `clang` nor `mold` was found on PATH. `zig` and `npm` were found, but presence of zig is not proof of an equivalent working toolchain. The older plan's scratchpad zig shim was not located or verified. Ask the user to provide/authorize the required compiler setup before claiming tests can pass. Do not edit committed linker config or invent a shim to conceal the blocker. Read-only work and scoped implementation can proceed while blocked; final commit may not.

Dependencies: Rust/Cargo/rustfmt, clang+mold configured as above (or a separately authorized alternative), cached/download-authorized Cargo dependencies, Node/npm and generated frontend dependencies. SQLx verification uses existing offline metadata, not a database. Stop if new queries require metadata regeneration or live DB permission.

## 4. Work graph and ownership

`T2-00 → T2-10 → {T2-20, T2-30} → T2-40 → T2-50 → T2-60 → T2-70`

T2-20 and T2-30 may run concurrently with non-overlapping ownership below. One orchestrator/integrator owns all Cargo manifests/lockfile, generated-project manifest, generated-project integration tests, README and roadmap edits, and Git staging/commit. Workers submit requested shared-file changes to that integrator instead of racing. Workspace-wide formatting and final tests run after workers stop writing. Serialize tests changing process cwd, using existing test harness conventions. Do not introduce agent-definition/configuration changes.

Each item below is a self-contained delegated brief: supply this spec plus its inputs to the responsible role; require exact changed paths, test output, and unresolved blockers in the worker report. Workers must not commit, install tools, touch secrets, expand scope, or overwrite other workers' changes.

### T2-00 — Revalidate and preserve the checkpoint

**Role:** orchestrator. **Inputs:** sections 1–3 and current worktree. **Dependencies:** none.

1. Run `pwd`, `git status --short`, `git branch --show-current`, `git rev-parse HEAD`, `git diff --stat`, `git diff --cached --stat`, `git worktree list`. Read current tracked/untracked in-scope files and new guidance without reading secrets.
2. Save a starting inventory and diff evidence under `/tmp/opencode/gurthang-t2-evidence/` using shell/dedicated tools. Do not reset/stash/clean user work. Record unknown concurrent edits and reconcile before proceeding.
3. Verify compiler/tool prerequisites without installing anything; surface known clang/mold blocker and request setup permission if still missing.
4. Attribute original T2 files and the investigator-generated lockfile delta. Reserve the Inertia comparison as unrelated; preserve and exclude it.

**Output:** checkpoint/ownership/blocker record. **Verification/completion:** baseline SHA and branch explicitly recorded; every starting edit classified; no user data lost. Stop if baseline differs materially and task ownership is unclear.

### T2-10 — Finish reusable parsing and redirect foundation

**Role:** framework worker; integrator owns manifest changes. **Inputs:** T2-00 inventory and section 2. **Dependencies:** T2-00.

**Targets:** existing `crates/gurthang-parse/{Cargo.toml,src/lib.rs}`, `crates/gurthang/src/invalid.rs`, facade `src/{lib,prelude}.rs`; new `crates/gurthang/tests/invalid.rs`; workspace/facade manifests and lockfile through integrator.

1. Review rather than replace the pending ErrorBag implementation. Preserve its public contract and zero-dependency boundary; test aggregation, duplicate-field first-wins, success mapping, opt-in old values, error-only emptiness, accessors/into_parts, and doctest example.
2. Review invalid helper session error mapping and exports. Use existing facade error conventions; no extractor or new flash system.
3. Add an Axum/MemoryStore cookie-preserving integration test. POST a register-shaped payload failing two parsers; assert 303 + exact location, both errors and unchanged submitted email in the persisted session, absence of old.password. Exercise success/failure mapping and session-error propagation where practical; report an uninjectable store-error coverage gap instead of claiming it is tested.
4. Ensure test-only MemoryStore/Tower utilities are dev dependencies/features only when required. Integrator reviews lockfile delta; format touched Rust.

**Outputs:** working public foundation plus integration tests. **Tests:** `cargo test -p gurthang-parse --locked`, `cargo test -p gurthang --test invalid --locked`. **Complete when:** those pass with doctests and dependency boundary intact; otherwise report exact blocked status, not a pass. The API contract is stable for downstream workers.

### T2-20 — Migrate generated User and auth end-to-end

**Role:** application-template worker. **Inputs:** foundation contract, selected defaults in section 2. **Dependencies:** T2-10 API contract; may parallelize with T2-30.

**Owned targets:** under `crates/gurthang-new/templates/`: `models/src/user.rs`, `models/src/factories/user.rs`, `src/services/auth.rs`, `src/controllers/{auth,shared}.rs`, `src/error.rs`, new `src/http/{mod.rs,requests/mod.rs,requests/user_registration.rs,requests/user_login.rs}`, and `resources/js/Pages/Auth/Login.tsx`. Send `src/lib.rs` module and renderer manifest additions to integrator T2-40.

1. Implement types/signatures in section 2; remove CreateUserData, free validate and normalize_email. Bind `email.as_ref()` in unchanged SQL. Preserve User Debug redaction and unique-email mapping.
2. Add constructor tests (valid/invalid normalized email; zero/multiple @; whitespace; boundaries; empty/short/multibyte registration passwords; short-but-nonempty login password; Debug never prints secrets). Add a compile_fail doctest proving a raw String cannot call User::create; use the rendered models crate's actual import name/placeholder so failure is caused by the type mismatch, not an unresolved import. Include a compiling positive example/test of the intended signature.
3. Parse factory strings through constructors with clear expectations. Search seed/task/other template callers for old model signatures and update all in-scope callers; do not change schema factory/sync generator design.
4. Introduce Deserialize-only registration/login payloads with default empty fields. Collect every field parse error with ErrorBag, keeping only the raw submitted email before normalization. Implement TryFrom to UserCreateData / typed Credentials. Never derive Debug exposing passwords.
5. Move generic `read_payload<T: DeserializeOwned>` into shared controller utilities retaining 64 KiB/JSON/form behavior. Remove AuthForm, parse_auth_form and local invalid. Add transparent conversion from gurthang::Error in AppError for `?`, keeping production responses safe.
6. Controllers parse before backend access, use facade invalid on parse/duplicate-email/auth failure; preserve success flow, session login/rotation, flash, welcome-mail enqueue, logout and destinations. Capture safe email before moving data when necessary. Retain duplicate message `Email has already been registered.`
7. Backend register takes typed data and performs hash/transaction/create, not validation. Credentials carry Email/PasswordAttempt. Adapt unknown-user dummy hashing to construct a valid NewPassword and copy the attempted password into the existing blocking verification task without weakening redaction or adding parsing dependencies to models.
8. Login UI renders email/password field errors as well as credentials errors. Keep props/export shapes stable.
9. Add tests in the real template auth controller using routes, MemoryStore, axum-login and a lazy PgPool. Verify both registration field errors and login parse policy without DB access, JSON and form bodies, missing fields vs malformed/oversized bodies, cookie round-trip, Inertia GET props and their one-shot consumption (second GET clears errors/email). Build required test context from current Context fields rather than booting database initializers. Keep security middleware covered by existing tests; do not disable it in production to ease testing.

**Outputs:** complete template slice, tests, list of manifest/caller additions for integrator. **Verification:** T2-40 generated-app checks execute these tests; review SQL and metadata diff stays empty. **Complete when:** rendered app compiles and tests prove typed boundary, aggregation/non-retention/consumption and unchanged successful auth wiring. Document that lazy-pool tests do not prove live successful DB auth/session rotation; retain existing coverage and report gaps.

### T2-30 — Request generator and CLI teaching

**Role:** generator/CLI worker. **Inputs:** section 2 and existing task/controller/model generator patterns. **Dependencies:** T2-10 API contract; parallel with T2-20.

**Targets:** new `crates/gurthang-generate/src/request.rs` and optional matching template; `src/lib.rs` generator export; `src/naming.rs` only if safely reusable; generator model/scaffold output paths and controller template conversion comment; `crates/gurthang-cli/src/{cli,lib}.rs`, `tests/cli.rs`. Integrator owns shared README/agent template/manifest additions.

1. Export `generate_request(name, GenerateOptions, out)` and dispatch `gurthang generate request <Name> [--dry-run]`; no DB schema lookup, prepare command, sync request, or new option required.
2. Normalize Pascal/snake naming using existing conventions; remove one trailing Pascal `Payload` before appending Payload. Both WidgetCreate and WidgetCreatePayload target `widget_create.rs` and `WidgetCreatePayload`, with domain `WidgetCreateData`. Reject empty/invalid/path-like identifiers before any writes; suffix-only Payload must not yield an empty identifier.
3. Use project-root discovery. Write `src/http/requests/<snake>.rs`, ensure `pub mod <snake>;` in requests/mod.rs, requests in http/mod.rs, http in src/lib.rs, creating missing layout for older apps. Refuse existing destination without changing anything; dry-run writes nothing, including directories. Preflight parent module layout; if an existing alternate `src/http.rs`/`src/http/requests.rs` conflicts with the prescribed directory layout, return a clear non-destructive error rather than creating ambiguous Rust modules. Preserve other content and avoid duplicate declarations.
4. Stub imports ErrorBag/Deserialize, defines string-field example comments, describes importing domain `<Name>Data`, and includes commented first-error collection/keep_old example. Body has `compile_error!("implement parsing ...")` rather than runnable TODO. Do not derive Debug. Include useful next steps with the generated type name.
5. Print Wrote/Updated (or Would write/update for dry-run), then define/import-domain, fill TryFrom, call gurthang::invalid-on-Err guidance. Do not pretend unfinished output compiles.
6. Add model help pointing to generate request and AGENTS.md (Models). For model and scaffold output, including dry-run, print the existing T2 CLI plan's parse-boundary command and do-not-pass-form-strings warning. Avoid duplicate hints during scaffold composition. Add a comment above scaffold's Form→CreateData conversion noting it currently binds strings and should be replaced by a parsed Payload; otherwise preserve scaffold body and existing SQL/model generation behavior.
7. Tests: render naming/suffix/type/blocker; tempdir write with module wiring and existing content; distinct second request does not duplicate parent declarations; same destination refuses overwrite byte-for-byte; dry-run tree unchanged; older-app missing directories; invalid names/conflicting module files no writes. Use existing cwd serialization or root-accepting internal helper for tests.
8. CLI tests assert request listing/options, model help's request/AGENTS hints, and dispatch behavior. Prove stub still fails compilation after supplying a resolvable dummy target domain type; assert diagnostic is its deliberate compile_error, not merely missing imports. A render-only string assertion is not sufficient proof of the chosen blocker semantics.

**Outputs:** generator, CLI wiring/tests, requested guidance/manifest edits for integrator. **Tests:** `cargo test -p gurthang-generate --locked`, `cargo test -p gurthang-cli --test cli --locked`. **Complete when:** tests pass and non-destructive behavior plus deliberate compile blocker are evidenced. No changes to sync/model data semantics.

### T2-40 — Integrate templates, guidance, and real generated-project tests

**Role:** integrator. **Inputs:** T2-20/T2-30 outputs. **Dependencies:** both complete enough to integrate; sole shared-file writer.

**Targets:** `crates/gurthang-new/src/renderer.rs` scaffold! manifest, template `src/lib.rs`, new template `AGENTS.md`, template `README.md.gurthang`, `crates/gurthang-cli/tests/generated_project.rs`, Cargo manifests/lock as necessary.

1. Register all new template files and pub mod http. Add generated AGENTS Models section verbatim from `docs/t2-cli-plan.md` plus concise layout/worked-example pointers. This is generated-app guidance, not a change to worker agent definitions. README growing sequence: migrate → model → request → scaffold, with a statement that schema types do not replace business parsing.
2. Extend exact-manifest and structural tests for new files, User typed create boundary, absence of validate/CreateUserData, no parse dependency in models, selected naming/exports and guidance. Preserve all existing unrelated assertions.
3. Add ignored integration test named `generated_app_passes_offline_tests` in generated_project.rs: generate into a fresh tempfile via `Source::Path` pointing to this checkout, then run `SQLX_OFFLINE=true cargo test --workspace` inside it, capture output and fail with diagnostics on nonzero exit. No database setup, environment-secret reads, tool install, or network operations inside ordinary default tests. Document its explicit invocation and dependency prerequisites. Use a dedicated target directory if nested Cargo locking would contend with the parent test process.
4. Run fresh generated-app verification below. Resolve legitimate template/import/lifetime/manifest errors within scope, not by removing tests. The unfinished request generator is tested in a separate copy so its intentional compile failure does not contaminate the valid default app.

**Outputs:** fully rendered app and integration evidence. **Tests:** `cargo test -p gurthang-cli --test generated_project --locked`; explicit ignored command in section 5. **Complete when:** structural/default suites pass and ignored nested app tests pass; frontend checks pass after separately approved dependency setup. Stop at environment blockers before final commit.

### T2-50 — Reconcile docs and record milestone evidence

**Role:** integrator/documentation worker (serialized shared edits). **Inputs:** integrated implementation and test results. **Dependencies:** T2-40.

**Targets:** new `docs/validation.md`, root `README.md`, `docs/roadmap.md`, one wording correction in `docs/adr/framework-shape.md`, T2 draft plan status notes; new `docs/t2-completion-report.md`.

1. Explain Payload→TryFrom→Data→User::create, ErrorBag and invalid session behavior, newtype ownership, opt-in old input/no secrets, request generation and deliberate unfinished stub blocker. Cite https://www.lpalmieri.com/posts/2020-12-11-zero-to-production-6-domain-modelling/ as existing design attribution; do not invent externally verified claims.
2. Root README command block lists generate request. Update T1 status to merged/done per actual narrower T1 finish plan; do not claim missing timing measurements were performed or relocate template auth. Update T2 scope/status/acceptance and sequencing to the actual narrowed shipped design and link validation guide; remove misleading combinator/model-generator expectations and clarify “merged” topic wording. Do not rewrite unrelated roadmap topics.
3. Correct ADR `TryFrom<Raw>` wording to Payload only; no framework redesign. Mark earlier T2 drafts as historical/superseded where they conflict (TODO stub, unanswered choices, old verification); keep context rather than deleting it.
4. Report baseline, changes, decisions, exact verification exit/results/log locations, coverage limitations, known non-goals and commit scope. Include that initial test attempts were blocked, not passed. Preserve Inertia comparison unchanged and outside commit.

**Outputs:** docs consistent with implementation and evidence. **Verification/completion:** docs commands cross-checked against actual CLI; every approved stream has results or a blocker. No “done” claim if required checks are blocked.

### T2-60 — Independent diff review, fix, and re-review gate

**Role:** read-only reviewer, then owning worker for fixes; orchestrator manages gate. **Inputs:** complete diff including in-scope untracked files, test evidence, approved documents. **Dependencies:** T2-50 and all required checks green.

1. Review type-enforcement, password handling, dummy verification, redirects/one-shot session props, generator dry-run/no-clobber/module safety, actual rendered output, and scope preservation. Compare final work against starting inventory; no secrets or unrelated generated artifacts.
2. Reviewer supplies severity-ranked findings with reproduction/verification; cannot self-approve worker changes. Fix all blocking findings, rerun affected tests plus final workspace checks, and re-review. Stop for a new approved direction if a fix requires changing a material locked decision.
3. Open final code diff via Plannotator. Use a discovered Plannotator tool if available; otherwise `plannotator review /home/mbv/work/gurthang --diff-type uncommitted --no-git-remote-check --tailscale --gate --json`. Explain the unrelated Inertia document is excluded from the commit even if shown. Run in background, read startup output, immediately provide the exact printed tailnet URL alone. Do not poll the review; parse returned decision. Approved notes are guidance, annotations require resolution/re-review, dismissed is not approved. Do not ask the user to review only tracked files while hiding new code.

**Outputs:** independent review findings/fixes and final explicit approval. **Completion:** no unresolved blocking findings; fresh verification results apply to reviewed tree; final gate approved. If user changes files after review, inspect/reverify/re-review affected content before commit.

### T2-70 — Scoped local commit and handoff to next milestone

**Role:** orchestrator only. **Inputs:** T2-60 approval, explicit user commit authorization recorded above. **Dependencies:** T2-60.

1. Revalidate HEAD/branch/worktree and inspect staged diff. Stage only reviewed T2 source/tests, required manifests/lockfile, validation docs/roadmap correction, and T2 planning/approval/spec/report documents. Explicitly exclude `docs/inertia-omega-vs-gurthang.md`, temporary logs/generated verification apps and any unrelated concurrent work. Ask if a newly discovered file's ownership is unclear.
2. Run `git diff --cached --check`, inspect `git diff --cached --stat` and full staged diff; verify no passwords/config secrets. Confirm staged tree matches verified/reviewed content. Never stage an unrelated existing index entry merely because it is already there.
3. Create a local Conventional Commit, suggested `feat(parse): finish typed auth inputs and request generation`; under 72-character imperative lowercase subject, body explains why. No amend/rebase/squash or tag/push. Record actual commit SHA and residual status; do not promise the entire worktree is clean since the unrelated comparison remains.
4. Final report: completed milestone, commit SHA/title, exact checks/results, explicit limitations, residual unrelated edits, and next task. The next task is to locate/read authoritative Andurel v2-rc.1 vision, compare it with this library, then plan the vision implementation and explore verify's type-driven/macro tradeoffs. Ask for the vision source if not found. No next-feature code is part of this completion.

**Output/completion:** local commit exists containing only approved T2 work; no publication; residual changes explained; next milestone clearly separated. If required tests or review remain blocked, do not commit—report the concrete blocker and ask the user rather than treating finish-up urgency as permission to bypass verification.

## 5. Verification commands and evidence

From `/home/mbv/work/gurthang`, once toolchain prerequisites are satisfied (use `--offline` additionally only when required dependencies are cached):

```sh
git diff --check
cargo fmt --all -- --check
cargo test -p gurthang-parse --locked
cargo test -p gurthang --test invalid --locked
cargo test -p gurthang-generate --locked
cargo test -p gurthang-cli --test cli --locked
cargo test -p gurthang-cli --test generated_project --locked
cargo test --workspace --locked
cargo test -p gurthang-cli --test generated_project --locked generated_app_passes_offline_tests -- --ignored --exact --nocapture
```

For a persistent manual fixture, choose a new, nonexistent directory under `/tmp/opencode/gurthang-t2-evidence/` (never overwrite a previous run), then from repository root:

```sh
GURTHANG_ROOT=/home/mbv/work/gurthang cargo run -p gurthang-cli --locked -- new t2-check --path /tmp/opencode/gurthang-t2-evidence/t2-check
```

From that generated app, run (Cargo may generate its own fresh lockfile):

```sh
cargo fmt --all -- --check
SQLX_OFFLINE=true cargo check --workspace
SQLX_OFFLINE=true cargo test --workspace
SQLX_OFFLINE=true cargo run --bin export_payloads
npm run typecheck
npm run build
```

If npm dependencies are absent, ask permission before `npm install` in that disposable app, then rerun both frontend checks. Do not read/copy a real .env; SQLX_OFFLINE is passed explicitly. If build/runtime configuration requires secrets or a database, stop and document the missing prerequisite instead of reading or creating them. Verify export output is actually under `resources/js/generated` used by the frontend; correct in-scope template path issues rather than suppressing type errors.

In a **separate** generated copy, exercise request dry-run and generation with the built CLI, verify no writes on dry-run, deliberate stub compile failure even with the domain name resolved, then implement a tiny sample domain conversion in the fixture only and show it compiles/rejects bad input. No sample fixture code is committed to the library unless it is an intentional test fixture reviewed in T2-60.

Record each command, working directory, toolchain, exit status, outcome (pass/fail/blocked/not run), and log path in the completion report. Use shell pipeline failure handling if teeing logs so tee's success cannot mask a test failure. Final commands must run after all fixes; cached or historical evidence is not a fresh pass. Maintain separate fresh generated-app and workspace evidence. Clippy can be supplemental, but do not introduce a new repository-wide lint cleanup requirement.

## 6. Known verification gaps and failure boundaries

- Planner ran `git diff --check` successfully, while rustfmt check failed on pending facade files. `cargo test -p gurthang-parse --locked --offline` stopped at required lockfile change. Inspection agent's `cargo test -p gurthang-parse` and `cargo test -p gurthang --lib` stopped at missing clang. These are not successful code tests. No workspace or generated-app suite was run successfully during planning.
- Lazy-pool controller tests prove early failure paths without a DB, not real DB registration, unique-constraint execution, successful login timing, or persistent Postgres sessions. Preserve existing tests; explicitly disclose remaining live-DB gaps. New live-DB execution needs permission and a disposable target.
- invalid writes multiple session keys, not one atomic transaction; do not promise rollback if a later insert fails. Old input is per-field opt-in and consumed by app pages, not automatically cleared generically by the framework.
- Email validation is intentionally limited. Password secrecy is an API/logging discipline, not secure-memory zeroization. Do not claim stronger guarantees.
- Generated generic model/scaffold code is still schema-driven. Typed User is the worked example; compiler enforcement for every arbitrary generated resource requires the app author to define real domain types/conversions. The compile-time request stub teaches this rather than solving it automatically.
- If factory sync would overwrite the hand-written User types/factory, demonstrate and report the existing limitation; do not redesign sync/model generation in this task. Stop if completing the approved User slice would require such a redesign.
- Material departures, new dependency-heavy validation, macros/verify, unavailable vision assumptions, permission escalation, or destructive environment repair require user direction. Otherwise resolve implementation details within this spec without requesting redundant high-level approval.

## 7. Readiness checklist for the orchestrator

- Foundation → T2-10; worked app → T2-20; agent workflow → T2-30/T2-40; docs/fresh-app verification/review → T2-40/T2-50/T2-60; authorized local commit → T2-70.
- Every item has an owner, inputs, dependencies, targets, actions, outputs and objective completion criteria. Shared writers and concurrency are explicit.
- Planning approval does not replace the final code review or tool-install/network/database permissions. The known missing-toolchain blocker is still unresolved at this handoff.
- Start at T2-00. No access to the originating chat is needed; ask only about changed state, missing prerequisites/permissions or material scope changes.
