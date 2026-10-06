use sqlx::PgPool;

use crate::Error;

#[derive(Clone, Debug)]
pub struct Column {
    pub name: String,
    pub rust_type: String,
    pub rust_field: String,
    pub nullable: bool,
    pub primary_key: bool,
    pub identity: bool,
    pub timestamp: bool,
}

#[derive(Clone, Debug)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
}

pub async fn load_table(pool: &PgPool, table: &str) -> Result<Table, Error> {
    let rows: Vec<(String, String, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT column_name, data_type, udt_name, is_nullable, column_default, \
                COALESCE(is_identity, 'NO') \
         FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = $1 \
         ORDER BY ordinal_position",
    )
    .bind(table)
    .fetch_all(pool)
    .await?;
    if rows.is_empty() {
        return Err(Error::Message(format!(
            "table {table} was not found; run gurthang db migrate up first"
        )));
    }
    let pks: Vec<String> = sqlx::query_scalar(
        "SELECT kcu.column_name \
         FROM information_schema.table_constraints tc \
         JOIN information_schema.key_column_usage kcu \
           ON tc.constraint_name = kcu.constraint_name \
          AND tc.table_schema = kcu.table_schema \
         WHERE tc.table_schema = 'public' AND tc.table_name = $1 \
           AND tc.constraint_type = 'PRIMARY KEY'",
    )
    .bind(table)
    .fetch_all(pool)
    .await?;

    let columns = rows
        .into_iter()
        .map(|(name, data_type, udt_name, nullable, default, identity)| {
            let primary_key = pks.iter().any(|pk| pk == &name);
            let identity = identity == "YES"
                || default
                    .as_deref()
                    .is_some_and(|value| value.starts_with("nextval("));
            let rust_type = rust_type(&data_type, &udt_name, nullable == "YES");
            Column {
                rust_field: rust_field(&name),
                rust_type,
                timestamp: matches!(name.as_str(), "created_at" | "updated_at"),
                nullable: nullable == "YES",
                primary_key,
                identity,
                name,
            }
        })
        .collect();
    Ok(Table {
        name: table.to_owned(),
        columns,
    })
}

fn rust_field(column: &str) -> String {
    if column == "type" {
        "r#type".into()
    } else {
        column.to_owned()
    }
}

fn rust_type(data_type: &str, udt_name: &str, nullable: bool) -> String {
    let inner = match (data_type, udt_name) {
        (_, "uuid") | ("uuid", _) => "uuid::Uuid",
        (_, "timestamptz") | ("timestamp with time zone", _) => "chrono::DateTime<chrono::Utc>",
        (_, "timestamp") | ("timestamp without time zone", _) => "chrono::NaiveDateTime",
        (_, "int4") | ("integer", _) => "i32",
        (_, "int8") | ("bigint", _) => "i64",
        (_, "int2") | ("smallint", _) => "i16",
        (_, "bool") | ("boolean", _) => "bool",
        (_, "jsonb") | ("json", _) => "serde_json::Value",
        _ => "String",
    };
    if nullable {
        format!("Option<{inner}>")
    } else {
        inner.to_owned()
    }
}
