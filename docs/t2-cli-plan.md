# T2 CLI — type-driven design when agents create models

Agents do not read `docs/validation.md` first. They run `gurthang generate
model Widget` and copy what it writes. Today that is
`CreateWidgetData { name: String }`. The compiler never stops them.

The CLI is the teaching surface. T2 already locked User as the worked
example, `*Payload` / `*Data` names, `gurthang-parse`, and
`User::create` (associated functions). This note is how that convention
shows up **in commands**, not only in `models/src/user.rs`.

## What an agent actually sees

| Surface | Today | Needs |
| --- | --- | --- |
| `gurthang generate model --help` | “Write a model from the live database schema.” | Point at `generate request` and `AGENTS.md` (schema is not the parse boundary) |
| Files written | `models/src/generated/widget.rs` + wrapper; `CreateWidgetData` is schema types | Unchanged. HTTP parse files come from `generate request` |
| stdout | `Wrote models/src/widget.rs` | Next command + “do not pass `String` into `::create`” |
| `generate request` | absent | `UserRegistrationPayload` → `UserCreateData` |
| Generated `AGENTS.md` | none | House style an agent reads on `gurthang new` |
| App README “Growing the app” | `generate model` then `scaffold` | Same sequence as help: migrate → model → request → scaffold |
| `gurthang manifest` (T18) | later | Command graph; not this PR |

`sync model` must not wipe hand-written newtypes. Generated SQL stays in
`models/src/generated/`; parse types live in the wrapper
`models/src/widget.rs` (same split User will use).

**Decision:** teach, don’t rewrite `generate model`. Schema `*Data` stays
column types (`String`). The compiler lesson for new resources is
`generate request` + the files it writes, not a generated newtype per
column. User (Email / NewPassword) stays the hand-written example.

## High-level CLI

```text
gurthang db migrate up
gurthang generate model Widget
# row Widget { name: String }           ← query_as!
# CreateWidgetData { name: String }     ← schema, unchanged this PR
# Widget::create(pool, data)

gurthang generate request WidgetCreate
# src/http/requests/widget_create.rs
# WidgetCreatePayload { name: String }  ← browser
# TryFrom → domain *Data (agent fills parse; stub will not compile until then)
```

stdout after `generate model`:

```text
Wrote models/src/widget.rs
Wrote models/src/generated/widget.rs
HTTP parse boundary: gurthang generate request WidgetCreate
Do not pass form strings into Widget::create — parse a *Payload first.
See AGENTS.md (Models).
```

`generate scaffold` should mention the request step (not silently
bind form strings into `create`).

## AGENTS.md (from `gurthang new`)

Short. Agents grep this.

```markdown
# Models

- Browser input: `*Payload` (`src/http/requests/`).
- Writes: `*Data` after `parse` (`Email`, `WidgetName`, …).
- `query_as!` row fields stay `String` / sqlx types.
- Do not pass form `String`s into `::create`. Parse a `*Payload` first
  (`gurthang generate request`).
- Hand-written newtypes (`Email`, `NewPassword`) live next to the model
  that owns them. `generate model` does not invent those.
```

## T2 vs generate model

Locked T2 said **do not rewrite** `generate model` this PR. That leaves
agents with a String bag on every new resource. Pick one:

:::question
When `gurthang generate model Widget` runs, what teaches type-driven design?

A is docs/stdout they skip. B without `generate request` still has
`String` at HTTP. C is the compiler + the command name.

- [x] Teach only: keep schema `*Data` as String; help, stdout, `generate request`, AGENTS.md
- [ ] `generate model` emits empty-reject newtypes on `WidgetCreateData`; no AGENTS.md this PR
- [ ] Both: newtypes on `*CreateData` + `generate request` for `*Payload` + AGENTS.md + stdout

Recommended: Both: newtypes on `*CreateData` + `generate request` for `*Payload` + AGENTS.md + stdout
:::

Out: T18 `gurthang check` / `--json` / `manifest`, sqlx `Decode` on the
row, rewriting every generated model to free functions.
