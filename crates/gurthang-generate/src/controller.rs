use std::{io::Write, path::Path};

use gurthang_project::find_root;
use sqlx::PgPool;

use crate::{
    Error,
    naming::Resource,
    region, registration,
    schema::{self, Table},
    tmpl, wiring,
};

#[derive(Clone, Debug, Default)]
pub struct ControllerOptions {
    pub actions: Vec<String>,
    pub dry_run: bool,
}

impl ControllerOptions {
    pub fn actions(&self) -> Vec<String> {
        if self.actions.is_empty() {
            [
                "index", "show", "new", "create", "edit", "update", "destroy",
            ]
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
    write_controller(
        &root,
        &resource,
        &actions,
        table.as_ref(),
        options.dry_run,
        out,
    )?;
    write_routes(&root, &resource, &actions, options.dry_run, out)?;
    write_pages(&root, &resource, &actions, options.dry_run, out)?;
    if options.dry_run {
        writeln!(out, "Would write src/controllers/mod.rs")?;
        writeln!(out, "Would write src/routes/generated.rs")?;
        writeln!(out, "Would update src/lib.rs")?;
    } else {
        registration::rewrite(&root, false)?;
        wiring::apply(&root, &resource, &actions)?;
        writeln!(out, "Wrote src/controllers/mod.rs")?;
        writeln!(out, "Wrote src/routes/generated.rs")?;
        writeln!(out, "Updated src/lib.rs")?;
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
        &tmpl::controller(resource, actions, table)?,
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
        &tmpl::routes(resource, actions)?,
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
        if let Some(page) = tmpl::page_name(action) {
            let relative = format!("resources/js/Pages/{}/{}.tsx", resource.plural_pascal, page);
            write_new(
                root.join(&relative),
                &relative,
                &tmpl::page(resource, action)?,
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
