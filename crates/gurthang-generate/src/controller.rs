use std::{fs, io::Write, path::Path};

use gurthang_project::find_root;
use sqlx::PgPool;

use crate::{
    Error,
    naming::Resource,
    region,
    schema::{self, Column, Table},
};

#[derive(Clone, Debug, Default)]
pub struct ControllerOptions {
    pub actions: Vec<String>,
    pub dry_run: bool,
}

impl ControllerOptions {
    pub fn actions(&self) -> Vec<String> {
        if self.actions.is_empty() {
            ["index", "show", "new", "create", "edit", "update", "destroy"]
                .into_iter()
                .map(str::to_owned)
                .collect()
        } else {
            self.actions.clone()
        }
    }
}

pub fn generate(name: &str, options: ControllerOptions, out: &mut impl Write) -> Result<(), Error> {
    let root = find_root()?;
    let resource = Resource::parse(name, None);
    let actions = options.actions();
    let table = load_table_optional(&root, &resource.table);
    write_controller(&root, &resource, &actions, table.as_ref(), options.dry_run, out)?;
    write_routes(&root, &resource, &actions, options.dry_run, out)?;
    write_views(&root, &resource, &actions, table.as_ref(), options.dry_run, out)?;
    write_pages(&root, &resource, &actions, options.dry_run, out)?;
    if !options.dry_run {
        register(&root, &resource, &actions)?;
        writeln!(out, "Next: gurthang sync routes && gurthang sync payloads")?;
    }
    Ok(())
}

fn load_table_optional(root: &Path, table: &str) -> Option<Table> {
    crate::env::runtime().ok()?.block_on(async {
        let url = crate::env::database_url(root).ok()?;
        let pool = PgPool::connect(&url).await.ok()?;
        schema::load_table(&pool, table).await.ok()
    })
}

fn write_controller(
    root: &Path,
    resource: &Resource,
    actions: &[String],
    table: Option<&Table>,
    dry_run: bool,
    out: &mut impl Write,
) -> Result<(), Error> {
    let relative = format!("src/controllers/{}.rs", resource.plural_snake);
    write_new(
        root.join(&relative),
        &relative,
        &render_controller(resource, actions, table),
        dry_run,
        out,
    )
}

fn write_routes(
    root: &Path,
    resource: &Resource,
    actions: &[String],
    dry_run: bool,
    out: &mut impl Write,
) -> Result<(), Error> {
    let relative = format!("src/routes/{}.rs", resource.plural_snake);
    write_new(
        root.join(&relative),
        &relative,
        &render_routes(resource, actions),
        dry_run,
        out,
    )
}

fn write_views(
    root: &Path,
    resource: &Resource,
    actions: &[String],
    table: Option<&Table>,
    dry_run: bool,
    out: &mut impl Write,
) -> Result<(), Error> {
    let relative = format!("src/views/inertia/{}.rs", resource.plural_snake);
    write_new(
        root.join(&relative),
        &relative,
        &render_views(resource, actions, table),
        dry_run,
        out,
    )
}

fn write_pages(
    root: &Path,
    resource: &Resource,
    actions: &[String],
    dry_run: bool,
    out: &mut impl Write,
) -> Result<(), Error> {
    for action in actions {
        if let Some(page) = page_name(action) {
            let relative = format!(
                "resources/js/Pages/{}/{}.tsx",
                resource.plural_pascal, page
            );
            write_new(
                root.join(&relative),
                &relative,
                &render_page(resource, action),
                dry_run,
                out,
            )?;
        }
    }
    Ok(())
}

fn write_new(
    path: std::path::PathBuf,
    relative: &str,
    contents: &str,
    dry_run: bool,
    out: &mut impl Write,
) -> Result<(), Error> {
    if dry_run {
        writeln!(out, "Would write {relative}")?;
        return Ok(());
    }
    region::write_if_allowed(&path, contents, false, false)?;
    writeln!(out, "Wrote {relative}")?;
    Ok(())
}

fn register(root: &Path, resource: &Resource, actions: &[String]) -> Result<(), Error> {
    region::ensure_line(
        &root.join("src/controllers/mod.rs"),
        &format!("pub mod {};", resource.plural_snake),
    )?;
    patch_marked_file(
        &root.join("src/routes/mod.rs"),
        "// gurthang:generated:modules:start",
        "// gurthang:generated:modules:end",
        &format!("pub mod {};", resource.plural_snake),
    )?;
    patch_marked_file(
        &root.join("src/routes/mod.rs"),
        "// gurthang:generated:mounts:start",
        "// gurthang:generated:mounts:end",
        &format!("    let router = {}::mount(router);", resource.plural_snake),
    )?;
    patch_marked_file(
        &root.join("src/views/inertia/mod.rs"),
        "// gurthang:generated:modules:start",
        "// gurthang:generated:modules:end",
        &format!("pub mod {};", resource.plural_snake),
    )?;
    for action in actions {
        if let Some(props) = props_name(action) {
            patch_marked_file(
                &root.join("src/views/inertia/mod.rs"),
                "// gurthang:generated:exports:start",
                "// gurthang:generated:exports:end",
                &format!("    {}::{props}::export()?;", resource.plural_snake),
            )?;
        }
    }
    Ok(())
}

fn patch_marked_file(path: &Path, start: &str, end: &str, line: &str) -> Result<(), Error> {
    if !path.exists() {
        return Err(Error::Message(format!(
            "{} is missing; cannot register generated code",
            path.display()
        )));
    }
    let source = fs::read_to_string(path)?;
    let updated = region::ensure_line_in_region(&source, start, end, line)?;
    if updated != source {
        fs::write(path, updated)?;
    }
    Ok(())
}

fn render_controller(resource: &Resource, actions: &[String], table: Option<&Table>) -> String {
    let pk_type = table
        .and_then(primary_key)
        .map(|column| column.rust_type.as_str())
        .unwrap_or("uuid::Uuid");
    let view_imports = actions
        .iter()
        .filter_map(|action| props_name(action))
        .collect::<Vec<_>>()
        .join(", ");
    let methods = actions
        .iter()
        .map(|action| controller_action(resource, action, pk_type, table))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"use axum::{{
    extract::{{Path, State}},
    http::{{HeaderMap, Method, Uri}},
    response::Response,
    Json,
}};
use serde::Deserialize;

use crate::{{
    app::AppState,
    error::{{AppError, Result}},
    models::{snake}::{{Create{pascal}Data, Update{pascal}Data, {pascal}}},
    services::auth::AuthSession,
    views::inertia::{{
        {plural}::{{{view_imports}}},
        shared::SharedProps,
    }},
}};
use gurthang_inertia::{{InertiaRequest, mutation_redirect}};

#[derive(Deserialize)]
pub struct {pascal}Form {{
{form_fields}
}}

impl From<{pascal}Form> for Create{pascal}Data {{
    fn from(form: {pascal}Form) -> Self {{
        Self {{
{form_assign}
        }}
    }}
}}

impl From<{pascal}Form> for Update{pascal}Data {{
    fn from(form: {pascal}Form) -> Self {{
        Self {{
{form_assign}
        }}
    }}
}}

{methods}"#,
        snake = resource.snake,
        pascal = resource.pascal,
        plural = resource.plural_snake,
        form_fields = form_fields(table),
        form_assign = form_assign(table),
    )
}

fn form_fields(table: Option<&Table>) -> String {
    let fields = writable(table);
    if fields.is_empty() {
        return "    pub _unused: Option<String>,".into();
    }
    fields
        .iter()
        .map(|column| format!("    pub {}: {},", column.rust_field, column.rust_type))
        .collect::<Vec<_>>()
        .join("\n")
}

fn form_assign(table: Option<&Table>) -> String {
    let fields = writable(table);
    if fields.is_empty() {
        return String::new();
    }
    fields
        .iter()
        .map(|column| format!("            {}: form.{},", column.rust_field, column.rust_field))
        .collect::<Vec<_>>()
        .join("\n")
}

fn writable(table: Option<&Table>) -> Vec<Column> {
    table
        .map(|table| {
            table
                .columns
                .iter()
                .filter(|column| !column.primary_key && !column.timestamp && !column.identity)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn primary_key(table: &Table) -> Option<&Column> {
    table.columns.iter().find(|column| column.primary_key)
}

fn controller_action(
    resource: &Resource,
    action: &str,
    pk_type: &str,
    _table: Option<&Table>,
) -> String {
    let pascal = &resource.pascal;
    let snake = &resource.snake;
    let plural = &resource.plural_snake;
    let path = &resource.path;
    match action {
        "index" => format!(
            r#"pub async fn index(
    State(state): State<AppState>,
    auth: AuthSession,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {{
    let items = {pascal}::list(&state.database).await?;
    let shared = SharedProps::from_auth(&auth).await?;
    state
        .inertia
        .render(
            &InertiaRequest::from_parts(&method, &uri, &headers),
            IndexProps {{
                {plural}: items.into_iter().map(Into::into).collect(),
            }},
            shared,
        )
        .await
        .map_err(Into::into)
}}
"#
        ),
        "show" => format!(
            r#"pub async fn show(
    State(state): State<AppState>,
    auth: AuthSession,
    Path(id): Path<{pk_type}>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {{
    let Some({snake}) = {pascal}::find(&state.database, id).await? else {{
        return Err(AppError::NotFound);
    }};
    let shared = SharedProps::from_auth(&auth).await?;
    state
        .inertia
        .render(
            &InertiaRequest::from_parts(&method, &uri, &headers),
            ShowProps {{
                {snake}: {snake}.into(),
            }},
            shared,
        )
        .await
        .map_err(Into::into)
}}
"#
        ),
        "new" => format!(
            r#"pub async fn new(
    State(state): State<AppState>,
    auth: AuthSession,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {{
    let shared = SharedProps::from_auth(&auth).await?;
    state
        .inertia
        .render(
            &InertiaRequest::from_parts(&method, &uri, &headers),
            NewProps {{}},
            shared,
        )
        .await
        .map_err(Into::into)
}}
"#
        ),
        "create" => format!(
            r#"pub async fn create(
    State(state): State<AppState>,
    Json(form): Json<{pascal}Form>,
) -> Result<Response> {{
    {pascal}::create(&state.database, form.into()).await?;
    mutation_redirect("{path}").map_err(Into::into)
}}
"#
        ),
        "edit" => format!(
            r#"pub async fn edit(
    State(state): State<AppState>,
    auth: AuthSession,
    Path(id): Path<{pk_type}>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {{
    let Some({snake}) = {pascal}::find(&state.database, id).await? else {{
        return Err(AppError::NotFound);
    }};
    let shared = SharedProps::from_auth(&auth).await?;
    state
        .inertia
        .render(
            &InertiaRequest::from_parts(&method, &uri, &headers),
            EditProps {{
                {snake}: {snake}.into(),
            }},
            shared,
        )
        .await
        .map_err(Into::into)
}}
"#
        ),
        "update" => format!(
            r#"pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<{pk_type}>,
    Json(form): Json<{pascal}Form>,
) -> Result<Response> {{
    {pascal}::update(&state.database, id, form.into()).await?;
    mutation_redirect(&format!("{path}/{{id}}"))
        .map_err(Into::into)
}}
"#
        ),
        "destroy" => format!(
            r#"pub async fn destroy(
    State(state): State<AppState>,
    Path(id): Path<{pk_type}>,
) -> Result<Response> {{
    {pascal}::delete(&state.database, id).await?;
    mutation_redirect("{path}").map_err(Into::into)
}}
"#
        ),
        other => format!("// unsupported action {other}\n"),
    }
}

fn render_routes(resource: &Resource, actions: &[String]) -> String {
    let consts = actions
        .iter()
        .filter_map(|action| route_const(resource, action))
        .collect::<Vec<_>>()
        .join("\n");
    let mounts = actions
        .iter()
        .filter_map(|action| route_mount(resource, action))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"use axum::{{Router, routing::{{{uses}}}}};
use gurthang_http::Route;

use crate::{{
    app::AppState,
    controllers::{module},
}};

{consts}

pub fn mount(router: Router<AppState>) -> Router<AppState> {{
    router
{mounts}
}}
"#,
        uses = route_uses(actions),
        module = resource.plural_snake,
    )
}

fn route_uses(actions: &[String]) -> String {
    let mut methods = Vec::new();
    for action in actions {
        let method = match action.as_str() {
            "index" | "show" | "new" | "edit" => "get",
            "create" => "post",
            "update" => "put",
            "destroy" => "delete",
            _ => continue,
        };
        if !methods.contains(&method) {
            methods.push(method);
        }
    }
    methods.join(", ")
}

fn route_const(resource: &Resource, action: &str) -> Option<String> {
    let (name, method, path) = route_parts(resource, action)?;
    let ident = format!(
        "{}_{}",
        resource.plural_snake.to_uppercase(),
        action.to_uppercase()
    );
    Some(format!(
        r#"pub const {ident}: Route = Route {{
    name: "{name}",
    method: "{method}",
    path: "{path}",
}};"#
    ))
}

fn route_mount(resource: &Resource, action: &str) -> Option<String> {
    let ident = format!(
        "{}_{}",
        resource.plural_snake.to_uppercase(),
        action.to_uppercase()
    );
    let method = match action {
        "index" | "show" | "new" | "edit" => "get",
        "create" => "post",
        "update" => "put",
        "destroy" => "delete",
        _ => return None,
    };
    Some(format!(
        "        .route({ident}.path, {method}({module}::{handler}))",
        module = resource.plural_snake,
        handler = action,
    ))
}

fn route_parts(resource: &Resource, action: &str) -> Option<(String, &'static str, String)> {
    let name = format!("{}.{}", resource.plural_snake, action);
    match action {
        "index" => Some((name, "GET", resource.path.clone())),
        "new" => Some((name, "GET", format!("{}/new", resource.path))),
        "create" => Some((name, "POST", resource.path.clone())),
        "show" => Some((name, "GET", format!("{}/{{id}}", resource.path))),
        "edit" => Some((name, "GET", format!("{}/{{id}}/edit", resource.path))),
        "update" => Some((name, "PUT", format!("{}/{{id}}", resource.path))),
        "destroy" => Some((name, "DELETE", format!("{}/{{id}}", resource.path))),
        _ => None,
    }
}

fn render_views(resource: &Resource, actions: &[String], table: Option<&Table>) -> String {
    let item_fields = item_fields(table);
    let pages = actions
        .iter()
        .filter_map(|action| view_props(resource, action))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"use serde::Serialize;
use ts_rs::TS;

use gurthang_inertia::{{InertiaPage, InertiaRenderMode}};
use crate::models::{snake}::{pascal};

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct {pascal}Props {{
{item_fields}
}}

impl From<{pascal}> for {pascal}Props {{
    fn from(item: {pascal}) -> Self {{
        Self {{
{from_fields}
        }}
    }}
}}

{pages}"#,
        snake = resource.snake,
        pascal = resource.pascal,
        from_fields = from_fields(table),
    )
}

fn item_fields(table: Option<&Table>) -> String {
    table
        .map(|table| {
            table
                .columns
                .iter()
                .map(|column| format!("    pub {}: {},", column.rust_field, column.rust_type))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "    pub id: uuid::Uuid,".into())
}

fn from_fields(table: Option<&Table>) -> String {
    table
        .map(|table| {
            table
                .columns
                .iter()
                .map(|column| format!("            {}: item.{},", column.rust_field, column.rust_field))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "            id: item.id,".into())
}

fn view_props(resource: &Resource, action: &str) -> Option<String> {
    let component = format!("{}/{}", resource.plural_pascal, page_name(action)?);
    let name = props_name(action)?;
    let fields = match action {
        "index" => format!(
            "    pub {}: Vec<{}Props>,",
            resource.plural_snake, resource.pascal
        ),
        "show" | "edit" => format!("    pub {}: {}Props,", resource.snake, resource.pascal),
        "new" => String::new(),
        _ => return None,
    };
    Some(format!(
        r#"#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../resources/js/generated/")]
pub struct {name} {{
{fields}
}}

impl InertiaPage for {name} {{
    const COMPONENT: &'static str = "{component}";
    const RENDER_MODE: InertiaRenderMode = InertiaRenderMode::Client;
}}
"#
    ))
}

fn props_name(action: &str) -> Option<&'static str> {
    match action {
        "index" => Some("IndexProps"),
        "show" => Some("ShowProps"),
        "new" => Some("NewProps"),
        "edit" => Some("EditProps"),
        _ => None,
    }
}

fn page_name(action: &str) -> Option<&'static str> {
    match action {
        "index" => Some("Index"),
        "show" => Some("Show"),
        "new" => Some("New"),
        "edit" => Some("Edit"),
        _ => None,
    }
}

fn render_page(resource: &Resource, action: &str) -> String {
    let page = page_name(action).unwrap_or("Index");
    let title = format!("{} {page}", resource.plural_pascal);
    format!(
        r#"import {{ Head }} from '@inertiajs/react'

export default function {page}() {{
  return (
    <main className="min-h-screen bg-slate-950 px-6 py-20 text-slate-100">
      <Head title="{title}" />
      <section className="mx-auto max-w-3xl rounded-3xl border border-slate-800 bg-slate-900 p-10">
        <h1 className="text-4xl font-bold">{title}</h1>
      </section>
    </main>
  )
}}
"#
    )
}
