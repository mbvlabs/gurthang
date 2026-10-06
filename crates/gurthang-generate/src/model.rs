use std::{fs, io::Write};

use gurthang_project::find_root;
use sqlx::PgPool;

use crate::{
    Error, factory,
    naming::Resource,
    region,
    schema::{self, Column, Table},
};

#[derive(Clone, Debug, Default)]
pub struct ModelOptions {
    pub table: Option<String>,
    pub dry_run: bool,
}

pub fn generate(name: &str, options: ModelOptions, out: &mut impl Write) -> Result<(), Error> {
    write_model(name, options, false, out)
}

pub fn sync(name: &str, check: bool, out: &mut impl Write) -> Result<(), Error> {
    write_model(
        name,
        ModelOptions {
            dry_run: check,
            ..ModelOptions::default()
        },
        true,
        out,
    )
}

fn write_model(
    name: &str,
    options: ModelOptions,
    overwrite: bool,
    out: &mut impl Write,
) -> Result<(), Error> {
    let root = find_root()?;
    let resource = Resource::parse(name, options.table.as_deref());
    let table = env_table(&root, &resource.table)?;
    let path = root.join(format!("models/src/{}.rs", resource.snake));
    let custom = if path.exists() {
        region::extract_custom(&fs::read_to_string(&path)?)
    } else {
        String::new()
    };
    let contents = region::with_custom(&render_model(&resource, &table), &custom);
    if options.dry_run && overwrite {
        if path.exists() && fs::read_to_string(&path)? != contents {
            return Err(Error::Message(format!(
                "{} is out of date",
                path.display()
            )));
        }
        writeln!(out, "{} is current", path.display())?;
        return Ok(());
    }
    if options.dry_run {
        writeln!(out, "Would write models/src/{}.rs", resource.snake)?;
        factory::write_factory(&root, &resource, &table, true, overwrite, out)?;
        return Ok(());
    }
    region::write_if_allowed(&path, &contents, false, overwrite)?;
    region::ensure_line(
        &root.join("models/src/lib.rs"),
        &format!("pub mod {};", resource.snake),
    )?;
    factory::write_factory(&root, &resource, &table, false, overwrite, out)?;
    writeln!(out, "Wrote models/src/{}.rs", resource.snake)?;
    Ok(())
}

fn env_table(root: &std::path::Path, table: &str) -> Result<Table, Error> {
    crate::env::runtime()?.block_on(async {
        let url = crate::env::database_url(root)?;
        let pool = PgPool::connect(&url).await?;
        schema::load_table(&pool, table).await
    })
}

pub(crate) fn render_model(resource: &Resource, table: &Table) -> String {
    let struct_fields = table
        .columns
        .iter()
        .map(|column| format!("    pub {}: {},", column.rust_field, column.rust_type))
        .collect::<Vec<_>>()
        .join("\n");
    let data_fields = writable_columns(table)
        .iter()
        .map(|column| format!("    pub {}: {},", column.rust_field, column.rust_type))
        .collect::<Vec<_>>()
        .join("\n");
    let pk = table.columns.iter().find(|column| column.primary_key);
    let find = pk
        .map(|pk| find_method(table, pk))
        .unwrap_or_default();
    let update = pk
        .map(|pk| update_method(resource, table, pk))
        .unwrap_or_default();
    let delete = pk
        .map(|pk| delete_method(table, pk))
        .unwrap_or_default();

    region::wrap_generated(&format!(
        "#[derive(Clone, Debug)]\n\
         pub struct {} {{\n\
         {struct_fields}\n\
         }}\n\n\
         #[derive(Clone, Debug)]\n\
         pub struct Create{}Data {{\n\
         {data_fields}\n\
         }}\n\n\
         #[derive(Clone, Debug)]\n\
         pub struct Update{}Data {{\n\
         {data_fields}\n\
         }}\n\n\
         impl {} {{\n\
         {find}{}\
         {}\
         {update}{delete}\
         }}",
        resource.pascal,
        resource.pascal,
        resource.pascal,
        resource.pascal,
        list_method(table),
        create_method(resource, table),
    ))
}

fn writable_columns(table: &Table) -> Vec<&Column> {
    table
        .columns
        .iter()
        .filter(|column| !column.primary_key && !column.timestamp && !column.identity)
        .collect()
}

fn find_method(table: &Table, pk: &Column) -> String {
    format!(
        "    pub async fn find(pool: &sqlx::PgPool, id: {}) -> sqlx::Result<Option<Self>> {{\n\
         \        sqlx::query_as!(\n\
         \            Self,\n\
         \            \"SELECT * FROM {} WHERE {} = $1\",\n\
         \            id\n\
         \        )\n\
         \        .fetch_optional(pool)\n\
         \        .await\n\
         \    }}\n\n",
        pk.rust_type, table.name, pk.name
    )
}

fn list_method(table: &Table) -> String {
    format!(
        "    pub async fn list(pool: &sqlx::PgPool) -> sqlx::Result<Vec<Self>> {{\n\
         \        sqlx::query_as!(\n\
         \            Self,\n\
         \            \"SELECT * FROM {} ORDER BY 1\"\n\
         \        )\n\
         \        .fetch_all(pool)\n\
         \        .await\n\
         \    }}\n\n",
        table.name
    )
}

fn create_method(resource: &Resource, table: &Table) -> String {
    let writable = writable_columns(table);
    let pk = table.columns.iter().find(|column| column.primary_key);
    let mut columns = Vec::new();
    let mut values = Vec::new();
    let mut binds = Vec::new();
    let mut index = 1;
    let mut id_line = String::new();
    if let Some(pk) = pk
        && !pk.identity
        && pk.rust_type.contains("Uuid")
    {
        columns.push(pk.name.clone());
        values.push(format!("${index}"));
        binds.push("            id,".into());
        id_line = "        let id = uuid::Uuid::new_v4();\n".into();
        index += 1;
    }
    for column in &writable {
        columns.push(column.name.clone());
        values.push(format!("${index}"));
        binds.push(format!("            data.{},", column.rust_field));
        index += 1;
    }
    for column in table.columns.iter().filter(|column| column.timestamp) {
        columns.push(column.name.clone());
        values.push(format!("${index}"));
        binds.push("            now,".into());
        index += 1;
    }
    format!(
        "    pub async fn create(pool: &sqlx::PgPool, data: Create{}Data) -> sqlx::Result<Self> {{\n\
         {id_line}\
         \        let now = chrono::Utc::now();\n\
         \        sqlx::query_as!(\n\
         \            Self,\n\
         \            \"INSERT INTO {} ({}) VALUES ({}) RETURNING *\",\n\
         {}\n\
         \        )\n\
         \        .fetch_one(pool)\n\
         \        .await\n\
         \    }}\n\n",
        resource.pascal,
        table.name,
        columns.join(", "),
        values.join(", "),
        binds.join("\n"),
    )
}

fn update_method(resource: &Resource, table: &Table, pk: &Column) -> String {
    let writable = writable_columns(table);
    let mut sets = Vec::new();
    let mut binds = Vec::new();
    let mut index = 1;
    for column in &writable {
        sets.push(format!("{} = ${index}", column.name));
        binds.push(format!("            data.{},", column.rust_field));
        index += 1;
    }
    if table.columns.iter().any(|column| column.name == "updated_at") {
        sets.push(format!("updated_at = ${index}"));
        binds.push("            now,".into());
        index += 1;
    }
    binds.push("            id,".into());
    format!(
        "    pub async fn update(pool: &sqlx::PgPool, id: {}, data: Update{}Data) -> sqlx::Result<Self> {{\n\
         \        let now = chrono::Utc::now();\n\
         \        sqlx::query_as!(\n\
         \            Self,\n\
         \            \"UPDATE {} SET {} WHERE {} = ${index} RETURNING *\",\n\
         {}\n\
         \        )\n\
         \        .fetch_one(pool)\n\
         \        .await\n\
         \    }}\n\n",
        pk.rust_type,
        resource.pascal,
        table.name,
        sets.join(", "),
        pk.name,
        binds.join("\n"),
    )
}

fn delete_method(table: &Table, pk: &Column) -> String {
    format!(
        "    pub async fn delete(pool: &sqlx::PgPool, id: {}) -> sqlx::Result<u64> {{\n\
         \        Ok(sqlx::query!(\n\
         \            \"DELETE FROM {} WHERE {} = $1\",\n\
         \            id\n\
         \        )\n\
         \        .execute(pool)\n\
         \        .await?\n\
         \        .rows_affected())\n\
         \    }}\n",
        pk.rust_type, table.name, pk.name
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Column;

    fn widget_table() -> Table {
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
                Column {
                    name: "created_at".into(),
                    rust_type: "chrono::DateTime<chrono::Utc>".into(),
                    rust_field: "created_at".into(),
                    nullable: false,
                    primary_key: false,
                    identity: false,
                    timestamp: true,
                },
                Column {
                    name: "updated_at".into(),
                    rust_type: "chrono::DateTime<chrono::Utc>".into(),
                    rust_field: "updated_at".into(),
                    nullable: false,
                    primary_key: false,
                    identity: false,
                    timestamp: true,
                },
            ],
        }
    }

    #[test]
    fn generated_model_binds_uuid_and_timestamps() {
        let resource = Resource::parse("Widget", None);
        let source = render_model(&resource, &widget_table());
        assert!(source.contains("pub struct Widget"));
        assert!(source.contains("pub struct CreateWidgetData"));
        assert!(source.contains("let id = uuid::Uuid::new_v4();"));
        assert!(source.contains("INSERT INTO widgets (id, name, created_at, updated_at)"));
        assert!(source.contains("SELECT * FROM widgets WHERE id = $1"));
        assert!(source.contains("DELETE FROM widgets WHERE id = $1"));
        assert!(source.contains(region::GENERATED_START));
    }

    #[test]
    fn serial_primary_keys_are_omitted_from_insert() {
        let mut table = widget_table();
        table.columns[0].rust_type = "i32".into();
        table.columns[0].identity = true;
        let source = render_model(&Resource::parse("Widget", None), &table);
        assert!(source.contains("INSERT INTO widgets (name, created_at, updated_at)"));
        assert!(!source.contains("let id = uuid::Uuid::new_v4();"));
    }
}
