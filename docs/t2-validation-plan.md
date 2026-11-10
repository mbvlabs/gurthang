# T2 — type-driven domain types

Palmieri ([domain modelling](https://www.lpalmieri.com/posts/2020-12-11-zero-to-production-6-domain-modelling/)):
newtype, private field, `parse` only constructor. No `Email(pub String)`,
no `inner_mut`.

**Naming.** Browser → `*Payload`. Model write DTO → `*Data`.

```text
UserRegistrationPayload   // strings from the form
UserCreateData            // Email + NewPassword (parsed)
UserLoginPayload          // strings from the form
```

Do **not** revisit `generate model` / `sync model` this PR. Template
`User` is the worked example. Generated `CreateWidgetData` stays as it
is. T1 `--timings` is out; mark T1 done in `docs/roadmap.md` (docs only).

**CLI for agents (locked):** `generate model` stays schema SQL. Teaching
is clap help, stdout next-step, `generate request`, generated `AGENTS.md`,
and the app README grow-the-app block. Details:
[`docs/t2-cli-plan.md`](t2-cli-plan.md).

## Domain (models crate, User only)

```rust
pub struct Email(String);        // parse only; AsRef<str>
pub struct NewPassword(String);  // required + min 12; Debug redacts
pub struct PasswordAttempt(String); // login: non-empty, no min-12
pub struct UserCreateData {
    pub email: Email,
    pub password: NewPassword,
}
```

`create` takes `&Email` + `password_hash: String`. Row `User.email` stays
`String` (`query_as!`). Unique email stays a DB error. Factory strings
go through `parse`. Delete `validate()` / `CreateUserData`.

## HTTP

```rust
pub struct UserRegistrationPayload { pub email: String, pub password: String }

impl TryFrom<UserRegistrationPayload> for UserCreateData {
    type Error = ErrorBag; // field-keyed; collect all
}

let payload = UserRegistrationPayload::from_body(request).await?;
match UserCreateData::try_from(payload) {
    Ok(data) => backend.register(data).await,
    Err(errors) => invalid(&session, auth::REGISTER, errors.old(), errors).await,
}
```

No extractor that 303s. `gurthang::invalid` = Inertia `errors` + `old.*`.
Login: `UserLoginPayload` → `Email` + `PasswordAttempt`; auth failure
stays the generic credentials message.

## `gurthang-parse`

`ErrorBag` + field mapping. Invariants live on `Email::parse`. Facade
re-exports. `models` does not depend on it.

## Generator

`gurthang generate request UserRegistration [--dry-run]` writes
`src/http/requests/user_registration.rs` (`UserRegistrationPayload` +
`TryFrom` stub). No `sync request`. README CLI line.

## Tests / docs

`Email::parse` / `NewPassword::parse` unit tests. Register: bad email /
short password → 303 + `errors` + `old.email`. `create` does not accept
`String`. `docs/validation.md` cites the article.

Out: `generate model` rewrite, `validator`/`garde`, sqlx `Decode` on the
row, auto-redirect extractor, T3 verify/reset, T1 `--timings`.

---

## Call sites: methods vs functions

Today (associated functions on the struct):

```rust
User::find(&pool, id).await?
User::create(&mut tx, email, hash).await?
let items = Widget::list(&self.database).await?;
Widget::update(&self.database, id, data).await?;
```

Free functions in the model module (`models/src/user.rs`):

```rust
user::find(&pool, id).await?
user::create(&mut tx, email, hash).await?
let items = widget::list(&self.database).await?;
widget::update(&self.database, id, data).await?;
```

Same arguments, same `query!`. The row type `User` stays a struct of
columns either way. Switching to functions **is** a models change
(template `user.rs` + generate templates).

:::question
SQL: methods on the struct, or free functions?

T2 can keep `User::create` (no models-style revisit). Free functions
would retouch `user.rs` and, later, `generate model`.

- [ ] Keep associated functions (`User::create`, `Widget::list`) this PR
- [ ] Switch template User to free functions this PR (`user::create`)
- [ ] Switch User + generate model/controller templates to free functions

Recommended: Keep associated functions this PR
:::
