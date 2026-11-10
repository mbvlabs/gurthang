# Finish and commit T2

The finish-up scope was approved in [the finish review](t2-finish-review.md). The user's follow-up directs us to finish the current work and commit it, then move on to the Andurel v2-rc.1 vision and exploration of a type-driven `gurthang verify` command, potentially using macros.

To avoid another questionnaire, this proposes one explicit package. **Approve this document to accept the four defaults below and the finish-and-commit scope**, or annotate anything that should change. The planner writes the execution handoff; the orchestrator implements and verifies it.

| Choice | Proposed decision | Why |
|---|---|---|
| Unfinished request stub | Unresolved domain type plus a compile-time blocker until parsing is implemented | An unfinished parser must not compile merely because its target type exists. |
| Malformed login input | Show email/password field errors; keep unknown-account/wrong-password failures generic | Separates input mistakes from authentication without disclosing account existence. |
| User factory | Keep the direct data return type; panic with a clear fixture-specific message when parsing fails | Test fixtures remain convenient but cannot bypass domain constructors. |
| JSON/form reader | Keep it in the generated application's shared controller utilities | Preserves both body formats without introducing a new framework API. |

## Commit boundary

The user's follow-up authorizes a local commit of the completed current work after verification and final diff review. Preserve unrelated work; do not silently include the Inertia comparison. Do not push, tag, publish, deploy, rewrite history, or create/switch branches without separate permission. Do not commit failing or environmentally blocked work as though verified: stop and report the blocker.

The subsequent request to finish, commit, and push the **spec** separately authorizes publication of this planning package to the existing origin/master. It does not waive implementation verification or extend push permission to unfinished implementation.

## Next milestone, not this commit

After T2 is closed, investigate the authoritative **Andurel v2-rc.1 vision** for this library and explore **`gurthang verify`**, leveraging type-driven design and evaluating macros rather than assuming they are needed. Obtain the source vision if it cannot be located. No implementation of that next milestone is authorized by this finish-up plan; its scope and design need their own investigation and review.

Everything else in the approved finish-up scope stays unchanged. Once this package is approved, the planner will write the separate orchestrator execution spec without another high-level review.
