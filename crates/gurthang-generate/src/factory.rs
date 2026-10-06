use std::{fs, io::Write, path::Path};

use gurthang_project::find_root;
use sqlx::PgPool;

use crate::{
    Error,
    naming::Resource,
    region,
    schema::{self, Table},
    tmpl,
};

pub fn write_factory(
    root: &Path,
    resource: &Resource,
    table: &Table,
    dry_run: bool,
    overwrite: bool,
    out: &mut impl Write,
) -> Result<(), Error> {
    let path = root.join(format!("models/src/factories/{}.rs", resource.snake));
    let contents = tmpl::factory(resource, table)?;
    if dry_run {
        writeln!(out, "Would write models/src/factories/{}.rs", resource.snake)?;
        return Ok(());
    }
    region::write_if_allowed(&path, &contents, false, overwrite)?;
    region::ensure_line(
        &root.join("models/src/factories/mod.rs"),
        &format!("pub mod {};", resource.snake),
    )?;
    region::ensure_line(&root.join("models/src/lib.rs"), "pub mod factories;")?;
    writeln!(out, "Wrote models/src/factories/{}.rs", resource.snake)?;
    Ok(())
}

pub fn sync_one(name: &str, check: bool, out: &mut impl Write) -> Result<(), Error> {
    let root = find_root()?;
    let resource = Resource::parse(name, None);
    let table = load_table(&root, &resource.table)?;
    let path = root.join(format!("models/src/factories/{}.rs", resource.snake));
    let contents = tmpl::factory(&resource, &table)?;
    if check {
        if path.exists() && fs::read_to_string(&path)? != contents {
            return Err(Error::Message(format!(
                "{} is out of date",
                path.display()
            )));
        }
        writeln!(out, "{} is current", path.display())?;
        return Ok(());
    }
    region::write_if_allowed(&path, &contents, false, true)?;
    writeln!(out, "Wrote models/src/factories/{}.rs", resource.snake)?;
    Ok(())
}

pub fn sync_all(check: bool, sync: bool, out: &mut impl Write) -> Result<(), Error> {
    if !check && !sync {
        return Err(Error::Message(
            "gurthang sync factories requires --check or --sync".into(),
        ));
    }
    let root = find_root()?;
    let models = root.join("models/src");
    if !models.is_dir() {
        return Err(Error::Message("models/src is missing".into()));
    }
    for entry in fs::read_dir(&models)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with(".rs") || name == "lib.rs" {
            continue;
        }
        let stem = name.trim_end_matches(".rs");
        if check {
            sync_one(stem, true, out)?;
        } else {
            sync_one(stem, false, out)?;
        }
    }
    Ok(())
}

fn load_table(root: &Path, table: &str) -> Result<Table, Error> {
    crate::env::runtime()?.block_on(async {
        let url = crate::env::database_url(root)?;
        let pool = PgPool::connect(&url).await?;
        schema::load_table(&pool, table).await
    })
}

