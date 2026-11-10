# inertia-omega vs Gurthang: Inertia v3 Adapter Comparison

A feature-by-feature comparison of **inertia-omega** (the official Rust
server-side Inertia.js v3 implementation from
[inertiajs/inertia-omega](https://github.com/inertiajs/inertia-omega)) against
Gurthang's own adapter in `crates/gurthang-inertia`.

Both target the same protocol and the same client (`@inertiajs/react` v3), so
this is the closest apples-to-apples comparison in the `docs/` set. Omega is the
**framework-agnostic core + Axum adapter** ported from `inertia-laravel`; Gurthang
is the **Axum-coupled, single-app** slice described in `plan.md`.

> Legend: **Full** = first-class equivalent, **Partial** = exists but narrower or
> different shape, **Absent** = no equivalent today, **Diverges** = solved a
> different way on purpose, **Gurthang+** = Gurthang is stronger.

---

## 0. Executive summary

The two agree on the shape of a render: a handler returns a page component and
props, the adapter turns that into an Inertia JSON page for XHR visits and a
root-view HTML document for first visits, and a version mismatch triggers a full
reload.

They diverge on almost everything else:

1. **Scope.** Omega is a standalone, published crate (~4,000 LOC, 24 modules)
   that ports the *whole* Laravel adapter: the props behavior system, flash,
   error bags, history flags, BigInt encoding, and the full protocol
   `before`/`after` lifecycle. Gurthang's adapter is ~600 LOC across 6 files and
   implements only the subset its generated app needs.
2. **Coupling.** Omega has a framework-agnostic core on the `http` crate plus an
   optional `axum` feature that is a thin translation layer. Gurthang's adapter
   *is* Axum: `InertiaRenderer` returns `axum::Response` and takes an
   `InertiaRequest` built from `Method`/`Uri`/`HeaderMap`.
3. **Props.** Omega's props are typed, lazy, and behavior-carrying (`defer`,
   `merge`, `once`, `scroll`, `always`, `deep_merge`, `try_lazy`), with a
   resolver that runs sibling callbacks concurrently and emits per-prop metadata.
   Gurthang's props are a plain `Serialize` value merged into shared data, with
   only top-level partial-reload include/exclude filtering.

Gurthang's deliberate trade is a small, single-app adapter with an **embedded,
stdio-based SSR worker** (no `reqwest`, no HTTP SSR server) and **Rust-sourced,
`ts-rs`-exported page contracts** that Omega does not attempt. Omega's trade is
protocol completeness and reusability.

---

## 1. Positioning and layout

| | inertia-omega | Gurthang (`gurthang-inertia`) |
|---|---|---|
| Origin | Official port of `inertia-laravel` | Hand-rolled for the generated app |
| Distribution | Published crate, `[ALPHA]` | Internal crate, one consumer |
| Core | Framework-agnostic (`http` crate) + adapter | Axum-only |
| Adapter seam | `axum` feature flag | none (Axum is hard-wired) |
| Files / LOC | 24 files / ~4,081 | 6 files / ~600 |
| Dependencies | `http`, `serde`, `indexmap`, `futures-util`, optional `reqwest`/`axum`/`tower-*`/`validator`/`garde` | `axum`, `serde`, `serde_json`, `thiserror`, `tokio`, `tracing` |
| Config type | `Config` (version, root view, shares, SSR, flags) | none; constructor args `(head_tags, title, version)` |
| Testing helper | `AssertablePage` (Laravel `assertInertia`) | renderer unit tests only |

Omega's module map (`config.rs`, `inertia.rs`, `request.rs`, `props/`,
`response.rs`, `page.rs`, `protocol.rs`, `session/`, `ssr.rs`, `view.rs`,
`testing.rs`, `axum/`) is documented in its README as the reference adapter shape.
Gurthang's map (`error.rs`, `page.rs`, `renderer.rs`, `request.rs`, `ssr.rs`,
`root.html`) is the subset the generated app exercises.

---

## 2. The render entry point

| Capability | Omega | Gurthang | Status |
|---|---|---|---|
| Render a page | `inertia.render("Home", props)` | `renderer.render(&req, "Home", props, shared)` | Partial |
| Deferred render resolution | handler returns `Response`, layer resolves props after | props resolved inside the handler's `render().await` | Diverges |
| Per-request handle | `Inertia` extractor (`FromRequestParts`) | none; `InertiaRequest` value built by hand | Absent |
| `Config` object | `Config::new().version(..).share(..)` | constructor args + `with_ssr` | Partial |
| Status/header preservation | yes (kept around the render) | n/a (renderer builds the response) | Partial |
| `with` / `with_view_data` / `with_header` | yes | no | Absent |
| `without_ssr` per render | yes | mode chosen by `render` vs `render_ssr` | Partial |

Gurthang's `InertiaPage` trait (`COMPONENT` + `RENDER_MODE`) was designed to be
the single source of truth, but the generated controllers bypass it: they call
`render`/`render_ssr` with a `&'static str` component and a separate `Serialize`
props struct. Omega takes the component as a plain `impl Into<String>` and leans
on the `props!` macro instead of a trait. Neither forces the contract.

---

## 3. The props system

This is the largest gap.

| Capability | Omega | Gurthang | Status |
|---|---|---|---|
| Plain serializable props | yes | yes | Full |
| `lazy` callback | yes | no | Absent |
| `try_lazy` + `rescue` | yes | no | Absent |
| `optional` (partial-only) | yes | no (filtered only by header) | Absent |
| `defer` / `.group` | yes | no | Absent |
| `once` / `.once_as` / `.until` / `.fresh` | yes | no | Absent |
| `always` | yes | no | Absent |
| `merge` / `append` / `prepend` / `.append_at` / `.prepend_at` | yes | no | Absent |
| `deep_merge` | yes | no | Absent |
| `match_on` | yes | no | Absent |
| `scroll` / `scroll_with` + `Paginator` | yes | no | Absent |
| Dot-notation nesting (`"auth.user.id"`) | yes | no | Absent |
| Concurrent sibling callbacks | yes (`join_all`) | n/a (no callbacks) | Absent |
| Per-prop metadata on the page | yes | no | Absent |
| Prop errors fail/`rescue` the response | yes | n/a | Absent |

Gurthang's `render_with` (`crates/gurthang-inertia/src/renderer.rs:75`) simply
serializes shared and page props to JSON objects, merges them (page wins), then
applies `filter_partial` (`renderer.rs:177`): top-level `retain` against
`x-inertia-partial-data` / `x-inertia-partial-except`. There is no callback
deferral, no merge semantics, and no `deferredProps`/`mergeProps`/`onceProps`
metadata, so the client's deferred/merge/once/infinite-scroll features are inert.
This matches `plan.md` §2, which explicitly excludes those behaviors from the PoC.

---

## 4. The page object

| Field | Omega | Gurthang |
|---|---|---|
| `component` | yes | yes |
| `props` | `Map<String, Value>` | `Value` (object) |
| `url` | yes (+ `resolve_url_using`) | yes |
| `version` | yes | yes |
| `sharedProps` | yes | no |
| `mergeProps` / `prependProps` / `deepMergeProps` | yes | no |
| `matchPropsOn` | yes | no |
| `deferredProps` / `rescuedProps` | yes | no |
| `scrollProps` / `onceProps` | yes | no |
| `preserveBigIntegers` | yes | no |
| `clearHistory` / `encryptHistory` | yes | no |
| `flash` | yes (protocol field) | no (hand-rolled as a prop) |
| `preserveFragment` | yes | no |

Gurthang's `Page` (`crates/gurthang-inertia/src/page.rs:15`) has exactly the four
required fields. Omega's `Page` (`src/page.rs`) flattens a `Metadata` struct and
carries the optional flags, serializing in protocol order and omitting empties.

---

## 5. Protocol rules (versioning, redirects, Vary)

| Capability | Omega | Gurthang | Status |
|---|---|---|---|
| Version mismatch → `409` + `X-Inertia-Location` | yes (full URL, echoes `X-Inertia-Version`) | yes (path+query only, no version echo) | Partial |
| Version check scope | `protocol::before`, before the handler | inline in `render_with`, only for `is_get` | Partial |
| `Vary: X-Inertia` | on **every** response | only on JSON / `409` responses | Partial |
| `303` after `PUT`/`PATCH`/`DELETE` | automatic (`protocol::after`) | manual `mutation_redirect` helper | Partial |
| Empty `200` → redirect back | yes | no | Absent |
| Redirect-with-fragment → `409` + `X-Inertia-Redirect` | yes | no | Absent |
| Prefetch-aware redirect handling | yes | no | Absent |
| `X-Inertia-Location` for external visits | yes (`inertia.location`) | `external_location` helper | Full |
| Named protocol headers | `header.rs` constants | string literals | Partial |

Omega splits the rules into `protocol::before`/`protocol::after` and lets the
adapter call them around the handler; Gurthang folds the version check into the
renderer and offers two free functions (`external_location`, `mutation_redirect`)
for redirects. The net effect for the generated app is similar, but Gurthang's
HTML responses never advertise `Vary: X-Inertia` and it does not implement the
empty-`200` and fragment edge cases.

---

## 6. Shared data, flash, and validation errors

| Capability | Omega | Gurthang | Status |
|---|---|---|---|
| Global shared props | `Config::share(closure)` | none | Absent |
| Per-request shared props | `inertia.share(k, v)` (middleware-friendly) | `shared` arg passed to `render` | Partial |
| Flash data (protocol `flash`) | `inertia.flash` / `Response::flash` | hand-rolled session `flash.success` prop | Diverges |
| Validation errors | `ValidationErrors` + error bags | hand-rolled `errors` map | Partial |
| `back_with_errors` / `with_errors_in` | yes | no | Absent |
| `validator` / `garde` conversions | yes (features) | no | Absent |
| `with_all_errors` | yes | no | Absent |
| Error bag scoping (`x-inertia-error-bag`) | yes | request field parsed, unused in rendering | Partial |

Gurthang's renderer unconditionally injects an empty `errors` object
(`renderer.rs:100`) so pages always have the key, and the generated app builds
`SharedProps { auth, errors, flash }` by pulling `errors`, `old.email`, and
`flash.success` out of the session in `SharedProps::from_auth`
(`crates/gurthang-new/templates/src/controllers/shared.rs`). Flash therefore
travels as a **prop**, not the protocol `flash` field, so it is kept in browser
history. Omega keeps flash out of history by design.

---

## 7. Server-side rendering

| Capability | Omega | Gurthang | Status |
|---|---|---|---|
| Transport | HTTP to `127.0.0.1:13714/render` | stdio subprocess, custom `gurthang:ssr:` protocol | Diverges |
| Dev-time SSR | Vite plugin `/__inertia_ssr` via hot file | spawns `node dist-ssr/ssr.mjs` | Diverges |
| Gateway abstraction | `Gateway` trait + `DynGateway` | concrete `InertiaSsr` only | Partial |
| Bundle delivery | path only | `include_bytes!` embedded in the binary, or dev path | Gurthang+ |
| Concurrency | HTTP client, connection-pooled, concurrent | single worker behind a `Mutex` (serialized) | Omega |
| Timeout / fallback to CSR | yes (5s default, `except` paths, `enabled`) | yes (per-render timeout, restart on failure) | Full |
| Runtime version gate | none | Node ≥ 22 check | Gurthang+ |
| Health check | `is_healthy()` | process-ready handshake | Full |
| Standard SSR contract | yes (any `@inertiajs` SSR server) | bespoke protocol (only the generated `ssr.tsx`) | Omega |

Omega's `HttpGateway` speaks the *standard* Inertia SSR contract, so it works
with the stock `@inertiajs` SSR server and the Vite dev plugin. Gurthang's
`InertiaSsr` (`crates/gurthang-inertia/src/ssr.rs`) spawns one long-lived Node
process and frames requests with a `gurthang:ssr:` line protocol
(`ssr.rs:19`), guarded by a single `Mutex` (`ssr.rs:89`) that serializes every
render. That is simpler to deploy (no extra HTTP service, bundle embedded) but
non-standard and a throughput bottleneck.

---

## 8. Sessions, root view, and JSON safety

| Capability | Omega | Gurthang | Status |
|---|---|---|---|
| Session abstraction | `Session` trait (`get`/`put`/`pull`) + `ArraySession` + `tower-sessions` impl | none (uses `axum-login` / `tower_sessions::Session` directly) | Absent |
| Session keys module | `session::key` constants | string literals | Partial |
| `commit()` lifecycle | yes (adapter-driven) | no | Absent |
| Root view | `RootView` trait + `View { page, head, body, data, ssr }` + default | `root.html` with `__TITLE__`/`__SSR_HEAD__`/`__ASSET_TAGS__`/`__BODY__` | Partial |
| Root element id | `Config::root_id` | fixed `app` in `root.html` | Partial |
| HTML-safe JSON | escapes `<`, `>`, `&`, `/`, U+2028/2029 | escapes `/` only | Partial |
| BigInt encoding | `encode_big_integers` (`$bigint` markers) | no | Absent |

Gurthang escapes `/` → `\/` (which defeats `</script>`), and its test asserts no
HTML entities are introduced (`renderer.rs:292`). Omega escapes the full
HTML-significant set (`src/json.rs`), which also covers `<!--` and attribute
contexts. Omega's `Session` trait makes the core testable without Axum; Gurthang
has no session seam, so its renderer is only testable through `renderer.rs`'s
unit tests.

---

## 9. Testing

| Capability | Omega | Gurthang | Status |
|---|---|---|---|
| `AssertablePage` from body | yes (JSON or HTML) | no | Absent |
| `component` / `url` / `version` assertions | yes | no | Absent |
| `has` / `missing` / `equals` / `has_count` (dot paths) | yes | no | Absent |
| `deferred` / `flash` assertions | yes | no | Absent |
| Framework-free core tests | yes (`Request` + `ArraySession` + `into_page`) | no | Absent |

---

## 10. Where Gurthang is ahead or intentionally different

- **Typed page contracts.** `ts-rs` exports `DashboardProps.ts`, `SharedProps.ts`,
  etc., from the Rust structs, and `routes.ts` is generated by `sync routes`.
  Omega has no TypeScript codegen; props are untyped JSON.
- **Embedded SSR bundle.** `include_bytes!(concat!(env!("OUT_DIR"),
  "/inertia-ssr.mjs"))` yields a single self-contained binary. Omega assumes a
  separately-run SSR server.
- **Runtime version gate.** Gurthang refuses Node < 22 up front; Omega does not.
- **No HTTP dependency for SSR.** Gurthang's stdio worker needs no `reqwest`, no
  SSR port, and no extra process to supervise at deploy time.
- **Single-file root view.** `root.html` is easy to read and edit; Omega's
  `RootView` closure is more flexible but more code.
- **`InertiaPage` trait + `RENDER_MODE`.** A good idea (per-page SSR opt-in) that
  Omega lacks, even if the generated controllers currently bypass it.

---

## 11. Gaps and recommended follow-ups

If Gurthang wants parity with the official adapter, the highest-value items, in
order:

1. **Props behaviors** (T12 in `docs/roadmap.md`): `defer`, `merge`, `once`,
   `scroll`, `always`, `deep_merge`, `lazy`/`try_lazy`, dot-notation, and the
   matching `deferredProps`/`mergeProps`/`onceProps`/`scrollProps` metadata. This
   is the single biggest functional gap and unblocks infinite scroll and
   merge-heavy lists.
2. **Protocol lifecycle**: split the version/redirect rules into
   `before`/`after`, add `Vary: X-Inertia` to every response, and implement the
   empty-`200` and fragment-`409` edge cases.
3. **Flash + error bags as protocol fields** rather than hand-rolled props, so
   flash stays out of browser history, plus `with_all_errors` and
   `validator`/`garde` conversions.
4. **A `Session` seam** so the adapter is testable without Axum and can commit
   flash/errors/history flags centrally.
5. **`AssertablePage`** and framework-free core tests.
6. **Full HTML-safe JSON escaping** (`<`, `>`, `&`, U+2028/2029) and BigInt
   encoding.

Optional, lower priority: `Config::share`, `root_id`, `resolve_url_using`,
`encrypt_history`/`clear_history`, and history encryption.

---

## 12. Verdict

Gurthang's adapter is a correct, minimal implementation of the Inertia v3
protocol's *core* (render, JSON vs HTML, version reload, partial reload,
redirect helpers) and is honest about being a PoC slice. Omega is the complete,
reusable, Laravel-faithful port and is the right reference for closing the gap.

The two most consequential differences are **(a)** the missing props behavior
system, which makes the client's deferred/merge/once/scroll features unusable
today, and **(b)** the bespoke stdio SSR transport versus Omega's standard HTTP
gateway. Everything else is either a small protocol detail or a deliberate
Gurthang simplification. The typed-contract and embedded-bundle choices are
Gurthang strengths worth keeping.

---

## 13. What to adopt from Omega

Omega is a faithful port of `inertia-laravel`, so "adopt from Omega" mostly
means "finish the protocol Gurthang deliberately sliced down for the PoC". The
ADR rule applies: new capabilities land as primitives in `gurthang-inertia` and
get composed through the `gurthang` facade. Do **not** copy Omega's
framework-agnostic core or its adapter abstraction; Gurthang is Axum-only by
non-negotiable.

### Adopt as-is (correctness and core protocol)

| # | Item | Why | Where |
|---|---|---|---|
| A1 | `protocol::before` / `protocol::after` split | Fixes real bugs: `Vary: X-Inertia` on **every** response, `303` after mutation, empty-`200` → redirect back, fragment redirect → `409` + `X-Inertia-Redirect`, prefetch-aware. Cheap. | new `protocol.rs`, called from the Axum layer / `render` |
| A2 | Props behaviors: `defer`/`group`, `merge`/`append`/`prepend`/`append_at`/`prepend_at`, `deep_merge`, `match_on`, `once`/`once_as`/`until`/`fresh`, `always`, `lazy`/`try_lazy`+`rescue`, `optional`, `scroll`/`scroll_with`, dot-notation | The core Inertia v3 value. Unblocks T12 (pagination/infinite scroll) and merge-heavy lists. | `props/` module in `gurthang-inertia` |
| A3 | Page `Metadata` (`deferredProps`, `mergeProps`, `prependProps`, `deepMergeProps`, `matchPropsOn`, `onceProps`, `scrollProps`, `rescuedProps`) | Required for A2; the client reads it. | `page.rs` |
| A4 | Flash + error bags as **protocol** fields (not props) | Keeps flash out of browser history; aligns with roadmap T2 ("map validation failures to the Inertia `errors` bag + `old.*` flash"). | `page.rs`, `Inertia` handle, `response.rs` |
| A5 | `ValidationErrors` type + `validator` / `garde` `From` impls, `with_all_errors`, error-bag scoping | Roadmap T2. Agent cannot ship a half-wired errors shape. | `gurthang-inertia` (feature-gated) |
| A6 | HTML-safe JSON escaping (`<`, `>`, `&`, `/`, U+2028/2029) | Security correctness; Gurthang escapes only `/` today. | `json.rs` |
| A7 | Named protocol-header constants | Readability; replaces string literals. | `header.rs` |
| A8 | `Session` trait (`get`/`put`/`pull`) + `ArraySession` | Testability without Axum; one place to commit flash/errors/history. Implement for `tower_sessions::Session` (Gurthang already uses it). | `session/` |
| A9 | `AssertablePage` test helper | Agent-first testing story; complements T17 boot harness. | `testing.rs` |

### Adopt, adapted to Gurthang's shape

| # | Item | Adaptation |
|---|---|---|
| B1 | Per-request `Inertia` handle as an Axum extractor | Adopt the handle (carries request + session + pending share/flash/errors). This lets **middleware** share the current user via `inertia.share(...)` instead of each controller rebuilding `SharedProps::from_auth` (today's `templates/src/controllers/shared.rs`). Biggest ergonomic win. |
| B2 | `Config` object | Replace `InertiaRenderer::new(head_tags, title, version)` with an `InertiaConfig` (version, shared props, root id, flags). Keep `with_ssr`. |
| B3 | `Gateway` trait for SSR | Adopt the seam (enables testing + alternatives). Keep the **embedded stdio worker as the default** (a Gurthang strength), and add an optional `HttpGateway` later so dev-time SSR can use the stock `@inertiajs` Vite plugin. |
| B4 | `RootView` as a `View { page, head, body, data, ssr }` struct | Keep `root.html` + `AssetResolver`, but feed it a `View` so root rendering is data-driven rather than `__PLACEHOLDER__` string replaces. |
| B5 | `commit()` lifecycle | Adapter-driven session write, replacing scattered `session.insert("flash.success", ...)` calls. |

### Deliberately skip

- **Framework-agnostic core + multi-framework adapter** (`http`-crate core, `DynSession`, `DynGateway` indirection). Gurthang is Axum-only; the abstraction is pure cost here.
- **`resolve_url_using`, `version_with`, `preserve_big_integers`, history encryption** (`clear_history`/`encrypt_history`). Low value for the generated app; revisit only if a page needs them.
- **Omega's "not ported yet" list**: DevTools, `ProvidesInertiaProperties`, component-name transformers, `ensure_pages_exist`, Precognition.
- **HTTP SSR transport by default.** Keep stdio/embedded; HTTP is opt-in (B3).

### Suggested sequencing

1. **A1, A6, A7** — small, independent, fixes correctness. Can land first.
2. **A3 + A2** — the props system; the largest piece. Start with `defer`/`lazy`/`merge`/`always`, then `once`/`scroll`; wire T12's `Paginator` to `scroll`.
3. **B1 + B2 + A8 + B5** — the `Inertia` handle, `Config`, `Session` seam, and `commit()`; removes the hand-rolled `SharedProps` plumbing.
4. **A4 + A5** — flash/error bags as protocol fields plus `validator`/`garde`; ties into T2.
5. **A9 + B4** — `AssertablePage` and the `View` root contract.
6. **B3** — `Gateway` trait now; `HttpGateway` only if dev-time SSR via the Vite plugin is wanted.

Keep the two Gurthang strengths untouched: **`ts-rs`-exported typed page
contracts** and the **embedded, single-binary SSR bundle**.
