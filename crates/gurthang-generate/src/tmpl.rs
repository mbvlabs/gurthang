use askama::Template;

use crate::{
    Error,
    naming::Resource,
    schema::{Column, Table},
};

#[derive(Clone, Debug)]
struct NamedType {
    name: String,
    ty: String,
}

#[derive(Clone, Debug)]
struct FactoryField {
    name: String,
    ty: String,
    default: String,
}

#[derive(Clone, Debug)]
struct RouteConst {
    ident: String,
    name: String,
    verb: String,
    path: String,
    handler: String,
}

#[derive(Clone, Debug)]
struct ViewPage {
    name: String,
    kind: String,
    component: String,
}

#[derive(Clone, Debug)]
struct JsRoute {
    key: String,
    name: String,
    method: String,
    path: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ControllerModule {
    pub name: String,
}

#[derive(Clone, Debug)]
pub(crate) struct JobModule {
    pub pascal: String,
    pub snake: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ControllerWiring {
    pub add_route: String,
    pub export_types: Vec<String>,
}

#[derive(Template)]
#[template(path = "model.rs", escape = "none")]
struct ModelTemplate {
    pascal: String,
    table: String,
    struct_fields: Vec<NamedType>,
    data_fields: Vec<NamedType>,
    has_pk: bool,
    pk_name: String,
    pk_type: String,
    generate_uuid: bool,
    insert_columns: String,
    insert_placeholders: String,
    insert_binds: Vec<String>,
    update_sets: String,
    update_binds: Vec<String>,
    update_pk_index: u32,
}

#[derive(Template)]
#[template(path = "factory.rs", escape = "none")]
struct FactoryTemplate {
    snake: String,
    pascal: String,
    fields: Vec<FactoryField>,
}

#[derive(Template)]
#[template(path = "model_wrapper.rs", escape = "none")]
struct ModelWrapperTemplate {
    snake: String,
}

#[derive(Template)]
#[template(path = "workers_generated.rs", escape = "none")]
struct WorkersGeneratedTemplate {
    jobs: Vec<JobModule>,
}

#[derive(Template)]
#[template(path = "controller.rs", escape = "none")]
struct ControllerTemplate {
    snake: String,
    pascal: String,
    plural: String,
    struct_name: String,
    pk_type: String,
    form_empty: bool,
    form_fields: Vec<NamedType>,
    actions: Vec<String>,
    item_fields: Vec<NamedType>,
    pages: Vec<ViewPage>,
    has_item_props: bool,
    mount_routes: Vec<RouteConst>,
    index_ident: String,
    show_ident: String,
    index_component: String,
    show_component: String,
    new_component: String,
    edit_component: String,
}

#[derive(Template)]
#[template(path = "routes.rs", escape = "none")]
struct RoutesTemplate {
    routes: Vec<RouteConst>,
}

#[derive(Template)]
#[template(path = "page.tsx", escape = "none")]
struct PageTemplate {
    page: String,
    title: String,
}

#[derive(Template)]
#[template(path = "js_routes.ts", escape = "none")]
struct JsRoutesTemplate {
    routes: Vec<JsRoute>,
}

#[derive(Template)]
#[template(path = "migration.sql", escape = "none")]
struct MigrationTemplate {
    slug: String,
}

fn render(template: impl Template) -> Result<String, Error> {
    Ok(template.render()?)
}

pub(crate) fn model(resource: &Resource, table: &Table) -> Result<String, Error> {
    let writable = writable_columns(table);
    let pk = table.columns.iter().find(|column| column.primary_key);
    let generate_uuid = pk.is_some_and(|pk| !pk.identity && pk.rust_type.contains("Uuid"));

    let mut insert_columns = Vec::new();
    let mut insert_placeholders = Vec::new();
    let mut insert_binds = Vec::new();
    let mut index = 1_u32;
    if generate_uuid {
        if let Some(pk) = pk {
            insert_columns.push(pk.name.clone());
            insert_placeholders.push(format!("${index}"));
            insert_binds.push("id".into());
            index += 1;
        }
    }
    for column in &writable {
        insert_columns.push(column.name.clone());
        insert_placeholders.push(format!("${index}"));
        insert_binds.push(format!("data.{}", column.rust_field));
        index += 1;
    }
    for column in table.columns.iter().filter(|column| column.timestamp) {
        insert_columns.push(column.name.clone());
        insert_placeholders.push(format!("${index}"));
        insert_binds.push("now".into());
        index += 1;
    }

    let mut update_sets = Vec::new();
    let mut update_binds = Vec::new();
    let mut update_index = 1_u32;
    for column in &writable {
        update_sets.push(format!("{} = ${update_index}", column.name));
        update_binds.push(format!("data.{}", column.rust_field));
        update_index += 1;
    }
    if table
        .columns
        .iter()
        .any(|column| column.name == "updated_at")
    {
        update_sets.push(format!("updated_at = ${update_index}"));
        update_binds.push("now".into());
        update_index += 1;
    }
    update_binds.push("id".into());

    render(ModelTemplate {
        pascal: resource.pascal.clone(),
        table: table.name.clone(),
        struct_fields: named_types(table.columns.iter()),
        data_fields: named_types(writable.into_iter()),
        has_pk: pk.is_some(),
        pk_name: pk.map(|column| column.name.clone()).unwrap_or_default(),
        pk_type: pk
            .map(|column| column.rust_type.clone())
            .unwrap_or_default(),
        generate_uuid,
        insert_columns: insert_columns.join(", "),
        insert_placeholders: insert_placeholders.join(", "),
        insert_binds,
        update_sets: update_sets.join(", "),
        update_binds,
        update_pk_index: update_index,
    })
}

pub(crate) fn model_wrapper(resource: &Resource) -> Result<String, Error> {
    render(ModelWrapperTemplate {
        snake: resource.snake.clone(),
    })
}

pub(crate) fn workers_generated(jobs: &[JobModule]) -> Result<String, Error> {
    render(WorkersGeneratedTemplate {
        jobs: jobs.to_vec(),
    })
}

pub(crate) fn factory(resource: &Resource, table: &Table) -> Result<String, Error> {
    let fields = writable_columns(table)
        .into_iter()
        .map(|column| FactoryField {
            name: column.rust_field.clone(),
            ty: column.rust_type.clone(),
            default: default_value(column),
        })
        .collect();
    render(FactoryTemplate {
        snake: resource.snake.clone(),
        pascal: resource.pascal.clone(),
        fields,
    })
}

pub(crate) fn controller(
    resource: &Resource,
    actions: &[String],
    table: Option<&Table>,
) -> Result<String, Error> {
    let form_fields = writable(table);
    let pages = actions
        .iter()
        .filter_map(|action| view_page(resource, action))
        .collect::<Vec<_>>();
    let item_fields = table
        .map(|table| named_types(table.columns.iter()))
        .unwrap_or_else(|| {
            vec![NamedType {
                name: "id".into(),
                ty: "uuid::Uuid".into(),
            }]
        });
    let has_item_props = pages
        .iter()
        .any(|page| page.kind == "index" || page.kind == "item");
    render(ControllerTemplate {
        snake: resource.snake.clone(),
        pascal: resource.pascal.clone(),
        plural: resource.plural_snake.clone(),
        struct_name: resource.plural_pascal.clone(),
        pk_type: table
            .and_then(primary_key)
            .map(|column| column.rust_type.clone())
            .unwrap_or_else(|| "uuid::Uuid".into()),
        form_empty: form_fields.is_empty(),
        form_fields,
        actions: actions.to_vec(),
        item_fields,
        pages,
        has_item_props,
        mount_routes: actions
            .iter()
            .filter_map(|action| route_const(resource, action))
            .collect(),
        index_ident: route_ident(resource, "index"),
        show_ident: route_ident(resource, "show"),
        index_component: page_component(resource, "index"),
        show_component: page_component(resource, "show"),
        new_component: page_component(resource, "new"),
        edit_component: page_component(resource, "edit"),
    })
}

pub(crate) fn controller_wiring(resource: &Resource, actions: &[String]) -> ControllerWiring {
    let mut export_types = Vec::new();
    if actions
        .iter()
        .any(|action| matches!(action.as_str(), "index" | "show" | "edit"))
    {
        export_types.push(format!(
            "crate::controllers::{}::{}Props",
            resource.plural_snake, resource.pascal
        ));
    }
    for action in actions {
        if let Some(name) = props_name(action) {
            export_types.push(format!(
                "crate::controllers::{}::{name}",
                resource.plural_snake
            ));
        }
    }
    ControllerWiring {
        add_route: format!(
            ".add_route(controllers::{}::routes(ctx))",
            resource.plural_snake
        ),
        export_types,
    }
}

pub(crate) fn routes(resource: &Resource, actions: &[String]) -> Result<String, Error> {
    render(RoutesTemplate {
        routes: actions
            .iter()
            .filter_map(|action| route_const(resource, action))
            .collect(),
    })
}

pub(crate) fn controllers_mod(modules: &[ControllerModule]) -> String {
    let mut source = String::new();
    for module in modules {
        source.push_str(&format!("pub mod {};\n", module.name));
    }
    source
}

pub(crate) fn routes_generated(modules: &[String]) -> String {
    let mut source = String::new();
    for module in modules {
        source.push_str(&format!("pub mod {module};\n"));
    }
    source
}

pub(crate) fn page(resource: &Resource, action: &str) -> Result<String, Error> {
    let page = page_name(action).unwrap_or("Index");
    render(PageTemplate {
        page: page.to_owned(),
        title: format!("{} {page}", resource.plural_pascal),
    })
}

pub(crate) fn js_routes(routes: &[(String, String, String)]) -> Result<String, Error> {
    render(JsRoutesTemplate {
        routes: routes
            .iter()
            .map(|(name, method, path)| JsRoute {
                key: js_key(name),
                name: name.clone(),
                method: method.clone(),
                path: path.clone(),
            })
            .collect(),
    })
}

pub(crate) fn migration(slug: &str) -> Result<String, Error> {
    render(MigrationTemplate {
        slug: slug.to_owned(),
    })
}

pub(crate) fn page_name(action: &str) -> Option<&'static str> {
    match action {
        "index" => Some("Index"),
        "show" => Some("Show"),
        "new" => Some("New"),
        "edit" => Some("Edit"),
        _ => None,
    }
}

pub(crate) fn props_name(action: &str) -> Option<&'static str> {
    match action {
        "index" => Some("IndexProps"),
        "show" => Some("ShowProps"),
        "new" => Some("NewProps"),
        "edit" => Some("EditProps"),
        _ => None,
    }
}

fn writable_columns(table: &Table) -> Vec<&Column> {
    table
        .columns
        .iter()
        .filter(|column| !column.primary_key && !column.timestamp && !column.identity)
        .collect()
}

fn writable(table: Option<&Table>) -> Vec<NamedType> {
    table
        .map(|table| named_types(writable_columns(table).into_iter()))
        .unwrap_or_default()
}

fn named_types<'a>(columns: impl Iterator<Item = &'a Column>) -> Vec<NamedType> {
    columns
        .map(|column| NamedType {
            name: column.rust_field.clone(),
            ty: column.rust_type.clone(),
        })
        .collect()
}

fn primary_key(table: &Table) -> Option<&Column> {
    table.columns.iter().find(|column| column.primary_key)
}

fn default_value(column: &Column) -> String {
    let inner = column
        .rust_type
        .strip_prefix("Option<")
        .and_then(|value| value.strip_suffix('>'))
        .unwrap_or(&column.rust_type);
    let value = match inner {
        "String" => "\"example\".into()".into(),
        "bool" => "false".into(),
        "i16" | "i32" | "i64" => "0".into(),
        ty if ty.contains("Uuid") => "uuid::Uuid::new_v4()".into(),
        ty if ty.contains("DateTime") => "chrono::Utc::now()".into(),
        _ => "Default::default()".into(),
    };
    if column.nullable {
        format!("Some({value})")
    } else {
        value
    }
}

fn route_ident(resource: &Resource, action: &str) -> String {
    format!(
        "{}_{}",
        resource.plural_snake.to_uppercase(),
        action.to_uppercase()
    )
}

fn route_const(resource: &Resource, action: &str) -> Option<RouteConst> {
    let (name, method, path) = route_parts(resource, action)?;
    Some(RouteConst {
        ident: route_ident(resource, action),
        name,
        verb: method.to_lowercase(),
        path,
        handler: action.to_owned(),
    })
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

fn page_component(resource: &Resource, action: &str) -> String {
    view_page(resource, action)
        .map(|page| page.component)
        .unwrap_or_default()
}

fn view_page(resource: &Resource, action: &str) -> Option<ViewPage> {
    let name = props_name(action)?;
    let page = page_name(action)?;
    let kind = match action {
        "index" => "index",
        "show" | "edit" => "item",
        "new" => "empty",
        _ => return None,
    };
    Some(ViewPage {
        name: name.to_owned(),
        kind: kind.to_owned(),
        component: format!("{}/{page}", resource.plural_pascal),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Column;

    fn widget() -> (Resource, Table) {
        (
            Resource::parse("Widget", None),
            Table {
                name: "widgets".into(),
                columns: vec![
                    Column {
                        name: "id".into(),
                        rust_type: "uuid::Uuid".into(),
                        rust_field: "id".into(),
                        nullable: false,
                        primary_key: true,
                        identity: false,
                        timestamp: false,
                    },
                    Column {
                        name: "name".into(),
                        rust_type: "String".into(),
                        rust_field: "name".into(),
                        nullable: false,
                        primary_key: false,
                        identity: false,
                        timestamp: false,
                    },
                ],
            },
        )
    }

    #[test]
    fn renders_scaffold_file_types() {
        let (resource, table) = widget();
        let actions = ["index", "show", "new", "create"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let factory_src = factory(&resource, &table).unwrap();
        let controller_src = controller(&resource, &actions, Some(&table)).unwrap();
        let page_src = page(&resource, "index").unwrap();
        let routes_src =
            js_routes(&[("widgets.index".into(), "GET".into(), "/widgets".into())]).unwrap();
        assert!(factory_src.contains("pub struct WidgetFactory"));
        assert!(!factory_src.contains("gurthang:generated"));
        assert!(!factory_src.contains("gurthang:custom"));
        let jobs = workers_generated(&[
            JobModule {
                pascal: "PurgeExpiredSessions".into(),
                snake: "purge_expired_sessions".into(),
            },
            JobModule {
                pascal: "SendWelcome".into(),
                snake: "send_welcome".into(),
            },
        ])
        .unwrap();
        assert!(jobs.contains("PurgeExpiredSessions,"));
        assert!(jobs.contains("SendWelcome,"));
        assert!(jobs.contains("Mailer(Email)"));
        assert!(jobs.contains("Self::SendWelcome => \"send_welcome\""));
        assert!(jobs.contains("send_welcome::SendWelcome"));
        assert!(!jobs.contains("gurthang:generated"));
        assert!(controller_src.contains("pub struct Widgets"));
        assert!(controller_src.contains("pub async fn index"));
        assert!(controller_src.contains(".render("));
        assert!(controller_src.contains("\"Widgets/Index\""));
        assert!(!controller_src.contains("impl InertiaPage"));
        assert!(controller_src.contains("pub fn routes(ctx: &Context)"));
        assert!(controller_src.contains("mount!("));
        assert!(!controller_src.contains("AppState"));
        assert!(!controller_src.contains("views::inertia"));
        let rust_routes = routes(&resource, &actions).unwrap();
        assert!(rust_routes.contains("path: \"/widgets/{id}\""));
        assert!(rust_routes.contains("name: \"widgets.show\""));
        let wiring = controller_wiring(&resource, &actions);
        assert_eq!(
            wiring.add_route,
            ".add_route(controllers::widgets::routes(ctx))"
        );
        assert!(page_src.contains("export default function Index"));
        assert!(routes_src.contains("widgets.index"));
        assert!(routes_src.contains("{${key}}"));
    }

    #[test]
    fn nullable_factory_defaults_are_wrapped() {
        let column = Column {
            name: "title".into(),
            rust_type: "Option<String>".into(),
            rust_field: "title".into(),
            nullable: true,
            primary_key: false,
            identity: false,
            timestamp: false,
        };
        assert_eq!(default_value(&column), "Some(\"example\".into())");
    }
}

fn js_key(name: &str) -> String {
    if name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        name.to_owned()
    } else {
        format!("'{name}'")
    }
}
