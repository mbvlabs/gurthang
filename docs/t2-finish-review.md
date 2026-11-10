# Finish T2, then move on

## Decision requested

Finish **type-driven form parsing and its agent-facing workflow** as one bounded milestone. Do not start another roadmap topic or redesign model generation. Approval authorizes preparation of a separate orchestrator execution spec—not commits, publication, deployment, or source changes by this planner.

## Where we actually are

| Evidence | What it means |
|---|---|
| Current checkout is `master`, at the middleware PR #2 merge | Middleware is the preceding milestone, not the unfinished feature in this working tree. |
| Pending changes add a parsing leaf crate, facade exports, and an invalid-input redirect helper | T2 has a foundation, not a completed end-to-end slice. |
| T2 design notes describe typed User inputs, auth migration, request generation, and teaching surfaces | These newer notes narrow the older roadmap; they should guide completion. |
| Formatting check fails; the initial locked/offline parsing test could not proceed without a lockfile update | There is no fresh passing workspace verification. An inspection agent's unlocked test attempts added the local parsing crate to the lockfile; both builds then stopped because `clang` is missing. The generated lockfile delta is retained and must be revalidated. |
| An untracked Inertia-adapter comparison is also present | Preserve it separately; adapter feature expansion is not part of finishing T2. |

The roadmap's T1 status and broad T2 wording are historical, not reliable completion evidence. Its “merged” T2 heading refers to combining topics, not proof the implementation shipped. Existing review questions are not treated as approved decisions simply because a recommendation was written down.

## Outcome and boundaries

Browser strings become `*Payload`; explicit parsing produces domain `*Data` containing private, invariant-bearing types. Registration and login demonstrate the boundary. Failures retain the existing Inertia redirect/session error contract, collect all field errors, and keep only explicitly selected non-secret old input.

Keep associated model functions and database-shaped row fields. Keep the existing SQL and offline metadata valid. Registration passwords require at least 12 characters; login attempts only require a non-empty password, so existing shorter passwords remain usable. Authentication failure remains generic and session rotation, password hashing, CSRF, and route protection must not regress.

**Not included:** rewriting model/sync generators, a rule DSL or validation dependency, automatic redirecting extractors, new auth features, middleware redesign, timing benchmarks, queue work, aggregate checks/JSON manifests, or Inertia protocol expansion.

## Proposed approach

```text
Stabilize parsing + redirect contract
          ├── Typed User + generated auth flow ──┐
          └── Request generator + CLI guidance ─┤
                                               ↓
                                Fresh-app verification + docs
                                               ↓
                                     Review → fix → re-review
```

| Work stream | Responsibility / completion outcome |
|---|---|
| 1. Foundation | Review and finish the existing error bag, public exports, and session redirect helper; prove field collection, 303 behavior, and password non-retention. |
| 2. Worked application | Give User private Email/password types; migrate HTTP payloads, auth service, fixtures, and login error display together. Test malformed input before database access and preserve successful auth behavior. |
| 3. Agent workflow | Add non-destructive request generation with dry-run, module registration, and tests. Teach the boundary through CLI help/output, generated agent guidance, scaffold hints, and README. Do not silently claim schema-generated models enforce domain invariants. |
| 4. Close-out | Verify a freshly generated app as well as the workspace; publish the validation guide and reconcile only relevant roadmap/ADR drift. Record exact results and limitations; present the completed diff for review. |

Streams 2 and 3 can proceed in parallel once the foundation contract and decisions below are settled. Shared generation/manifest/test surfaces need coordinated ownership. Close-out waits for both.

## Remaining choices

:::question
How strongly should an unfinished generated request block compilation?

The request generator cannot infer a resource's business rules. The stub must make unfinished parsing visible rather than quietly accept browser strings. A compile-time blocker is safer; the earlier draft instead proposed an unresolved domain type plus a runtime TODO.

- [ ] Unresolved domain type plus TODO — once the type exists, unfinished parsing can compile and panic
- [ ] Unresolved domain type plus compile-time blocker — parsing must be implemented before the app builds

Recommended: Unresolved domain type plus compile-time blocker
:::

:::question
Should malformed login input show field-specific errors?

This concerns malformed email or an empty password before account lookup. Unknown accounts and wrong passwords still receive the same generic credentials message.

- [ ] Field-specific errors for malformed input
- [ ] Generic credentials message for all login failures

Recommended: Field-specific errors for malformed input
:::

:::question
How should invalid User factory input behave?

Factories are test fixtures. Their strings must pass the same constructors as real inputs; a bad fixture should fail clearly.

- [ ] Keep the convenient factory return type; panic clearly on invalid fixture data
- [ ] Return a result and update callers to handle fixture parsing failures

Recommended: Keep the convenient factory return type; panic clearly on invalid fixture data
:::

:::question
Where should reusable JSON/form body reading live?

The auth controllers already support both body formats. Keeping this helper in the generated application avoids adding a framework API solely for this migration.

- [ ] Generated application's shared controller utilities
- [ ] Public framework facade API

Recommended: Generated application's shared controller utilities
:::

## Acceptance and verification

- Registration and login use parsed domain types; raw strings cannot call the typed User write boundary.
- Bad registration input produces one 303 redirect, both field errors, the submitted email, and no password in old input. Login follows the selected error policy; duplicate email keeps the established error contract.
- Error-bag unit/doctests, domain constructor/redacted-debug tests, compile-fail coverage, redirect integration tests, generator safety tests, and generated-app structural tests are present.
- Request generation handles supported naming, registers modules without duplicate declarations, refuses overwrite, and makes no writes on dry-run. The unfinished stub behaves exactly as selected above.
- Fresh workspace tests and formatting pass; an actual newly generated app passes offline Rust tests and frontend typechecking/build after required dependency setup. Record environmental blockers explicitly instead of calling blocked checks passes.
- Existing middleware, session, CSRF, auth, Inertia, and generator regression suites remain intact. No live database mutation without separate permission.
- Documentation describes the shipped behavior and known limitation: general model/scaffold generation still uses schema types and needs application-authored domain parsing.
- Completed diff receives review; fixes are re-tested and re-reviewed before declaring this milestone ready. Commits and publishing remain separately authorized.

## Risks and handling

**Generated applications are separate builds.** Workspace success alone does not prove templates compile; test the real generated output.

**Toolchain setup may be missing.** Establish the linker/compiler and offline SQLx prerequisites first. Do not install tools, commit local shims, or weaken tests to hide setup failures.

**Session behavior can regress subtly.** Verify the redirect followed by shared-prop consumption, not only session insertion; retain existing one-shot error/old-input behavior.

**Old plans conflict.** The approved decisions in this review supersede conflicting T2 draft recommendations. Preserve historical notes but make the authoritative outcome discoverable.

## After approval

The planner will write a separate, durable execution spec with concrete work items, ownership, dependencies, commands, review gates, and permissions. That spec—not this high-level review—is the orchestrator's entry point. No next roadmap topic is selected here.
