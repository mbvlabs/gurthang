use std::{fs, io::Write, path::Path};

use gurthang_project::find_root;
use sqlx::PgPool;

use crate::{
    Error, factory,
    naming::Resource,
    region, registration,
    schema::{self, Table},
    tmpl,
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
    let generated_path = root.join(format!("models/src/generated/{}.rs", resource.snake));
    let wrapper_path = root.join(format!("models/src/{}.rs", resource.snake));
    if overwrite {
        if !generated_path.exists() {
            return Err(Error::Message(format!(
                "models/src/generated/{}.rs is missing; {} is not a generated model",
                resource.snake, resource.pascal
            )));
        }
    } else if wrapper_path.exists() {
        return Err(Error::Message(format!(
            "models/src/{}.rs already exists",
            resource.snake
        )));
    } else if generated_path.exists() {
        return Err(Error::Message(format!(
            "models/src/generated/{}.rs already exists",
            resource.snake
        )));
    }
    let table = env_table(&root, &resource.table)?;
    let contents = tmpl::model(&resource, &table)?;
    if options.dry_run && overwrite {
        if fs::read_to_string(&generated_path)? != contents {
            return Err(Error::Message(format!(
                "{} is out of date",
                generated_path.display()
            )));
        }
        writeln!(out, "{} is current", generated_path.display())?;
        return Ok(());
    }
    if options.dry_run {
        writeln!(
            out,
            "Would write models/src/generated/{}.rs",
            resource.snake
        )?;
        writeln!(out, "Would write models/src/{}.rs", resource.snake)?;
        factory::write_factory(&root, &resource, &table, true, overwrite, out)?;
        return Ok(());
    }
    region::write_if_allowed(&generated_path, &contents, false, overwrite)?;
    if !wrapper_path.exists() {
        region::write_if_allowed(
            &wrapper_path,
            &tmpl::model_wrapper(&resource)?,
            false,
            false,
        )?;
        writeln!(out, "Wrote models/src/{}.rs", resource.snake)?;
    }
    region::ensure_line(&root.join("models/src/lib.rs"), "pub mod generated;")?;
    region::ensure_line(
        &root.join("models/src/lib.rs"),
        &format!("pub mod {};", resource.snake),
    )?;
    rewrite_generated_mod(&root, false)?;
    factory::write_factory(&root, &resource, &table, false, overwrite, out)?;
    writeln!(out, "Wrote models/src/generated/{}.rs", resource.snake)?;
    Ok(())
}

fn rewrite_generated_mod(root: &Path, check: bool) -> Result<(), Error> {
    let directory = root.join("models/src/generated");
    let modules = registration::rust_modules(&directory, &["mod"])?
        .into_iter()
        .map(|name| tmpl::ControllerModule { name })
        .collect::<Vec<_>>();
    registration::write_generated(
        &directory.join("mod.rs"),
        &tmpl::controllers_mod(&modules),
        check,
    )
}

fn env_table(root: &Path, table: &str) -> Result<Table, Error> {
    crate::env::runtime()?.block_on(async {
        let url = crate::env::database_url(root)?;
        let pool = PgPool::connect(&url).await?;
        schema::load_table(&pool, table).await
    })
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
        let source = tmpl::model(&resource, &widget_table()).unwrap();
        assert!(source.contains("pub struct Widget"));
        assert!(source.contains("pub struct CreateWidgetData"));
        assert!(source.contains("let id = uuid::Uuid::new_v4();"));
        assert!(source.contains("INSERT INTO widgets (id, name, created_at, updated_at)"));
        assert!(source.contains("SELECT * FROM widgets WHERE id = $1"));
        assert!(source.contains("DELETE FROM widgets WHERE id = $1"));
        assert!(source.contains("data.name,"));
        assert!(!source.contains("gurthang:generated"));
        assert!(!source.contains("gurthang:custom"));
        let wrapper = tmpl::model_wrapper(&resource).unwrap();
        assert_eq!(wrapper.trim(), "pub use crate::generated::widget::*;");
    }

    #[test]
    fn serial_primary_keys_are_omitted_from_insert() {
        let mut table = widget_table();
        table.columns[0].rust_type = "i32".into();
        table.columns[0].identity = true;
        let source = tmpl::model(&Resource::parse("Widget", None), &table).unwrap();
        assert!(source.contains("INSERT INTO widgets (name, created_at, updated_at)"));
        assert!(!source.contains("let id = uuid::Uuid::new_v4();"));
    }
}
