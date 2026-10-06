use std::fs;
use std::path::Path;

use crate::{Error, naming::Resource, tmpl};

pub fn apply(root: &Path, resource: &Resource, actions: &[String]) -> Result<(), Error> {
    let path = root.join("src/lib.rs");
    if !path.is_file() {
        return Err(Error::Message("src/lib.rs is missing".into()));
    }
    let current = fs::read_to_string(&path)?;
    let updated = wire(&current, &tmpl::controller_wiring(resource, actions))?;
    if updated != current {
        fs::write(&path, updated)?;
    }
    Ok(())
}

fn wire(source: &str, wiring: &tmpl::ControllerWiring) -> Result<String, Error> {
    let mut source = source.to_owned();
    source = ensure_import(
        &source,
        &format!(
            "use crate::controllers::{}::{};",
            wiring.module, wiring.struct_name
        ),
    );
    source = ensure_app_field(
        &source,
        &format!("    pub {}: {},\n", wiring.field, wiring.struct_name),
    )?;
    source = ensure_app_init(
        &source,
        &format!(
            "            {}: {} {{\n                database: database.clone(),\n                inertia: inertia.clone(),\n            }},\n",
            wiring.field, wiring.struct_name
        ),
    )?;
    for line in &wiring.mount_lines {
        source = ensure_mount_line(&source, line)?;
    }
    for export in &wiring.export_types {
        source = ensure_export(&source, export)?;
    }
    Ok(source)
}

fn ensure_import(source: &str, line: &str) -> String {
    if source.contains(line) {
        return source.to_owned();
    }
    insert_after_line(source, "pub mod web;", &format!("\n{line}"))
        .unwrap_or_else(|| format!("{line}\n{source}"))
}

fn ensure_app_field(source: &str, field: &str) -> Result<String, Error> {
    if source.contains(field.trim()) {
        return Ok(source.to_owned());
    }
    insert_before(source, "    pub database:", field)
        .ok_or_else(|| Error::Message("could not find App.database to insert controller".into()))
}

fn ensure_app_init(source: &str, init: &str) -> Result<String, Error> {
    if source.contains(init.trim()) {
        return Ok(source.to_owned());
    }
    insert_before(source, "            jobs:", init).ok_or_else(|| {
        Error::Message("could not find App::new jobs field to insert controller".into())
    })
}

fn ensure_mount_line(source: &str, line: &str) -> Result<String, Error> {
    if source.contains(line.trim()) {
        return Ok(source.to_owned());
    }
    let Some(start) = source.find("gurthang_http::mount!(") else {
        return Err(Error::Message(
            "src/lib.rs is missing gurthang_http::mount!".into(),
        ));
    };
    let rest = &source[start..];
    let Some(relative_end) = rest.find("\n    });") else {
        return Err(Error::Message("could not find the end of mount!".into()));
    };
    let index = start + relative_end + 1;
    let mut updated = String::new();
    updated.push_str(&source[..index]);
    updated.push_str(line);
    if !line.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&source[index..]);
    Ok(updated)
}

fn ensure_export(source: &str, ty: &str) -> Result<String, Error> {
    let call = format!("    {ty}::export()?;");
    if source.contains(&call) {
        return Ok(source.to_owned());
    }
    let Some(start) = source.find("pub fn export_payloads(") else {
        return Err(Error::Message(
            "src/lib.rs is missing export_payloads()".into(),
        ));
    };
    let rest = &source[start..];
    let Some(relative_ok) = rest.find("\n    Ok(())") else {
        return Err(Error::Message(
            "could not find export_payloads return".into(),
        ));
    };
    let index = start + relative_ok + 1;
    let mut updated = String::new();
    updated.push_str(&source[..index]);
    updated.push_str(&call);
    updated.push('\n');
    updated.push_str(&source[index..]);
    Ok(updated)
}

fn insert_before(source: &str, marker: &str, insertion: &str) -> Option<String> {
    let index = source.find(marker)?;
    let mut updated = String::new();
    updated.push_str(&source[..index]);
    updated.push_str(insertion);
    updated.push_str(&source[index..]);
    Some(updated)
}

fn insert_after_line(source: &str, marker: &str, insertion: &str) -> Option<String> {
    let start = source.find(marker)?;
    let rest = &source[start..];
    let end = rest.find('\n')?;
    let index = start + end + 1;
    let mut updated = String::new();
    updated.push_str(&source[..index]);
    updated.push_str(insertion);
    updated.push_str(&source[index..]);
    Some(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::naming::Resource;

    fn sample_lib() -> &'static str {
        r#"
use crate::{
    controllers::{auth::Auth, dashboard::Dashboard, welcome::Welcome},
};

pub struct App {
    pub welcome: Welcome,
    pub auth: Auth,
    pub dashboard: Dashboard,
    pub database: PgPool,
    pub jobs: JobQueue,
}

impl App {
    pub fn new(database: PgPool, inertia: InertiaRenderer) -> Self {
        Self {
            welcome: Welcome { inertia: inertia.clone() },
            jobs: JobQueue::new(database.clone()),
            database,
        }
    }
}

fn router(app: App) -> Router {
    let router = gurthang_http::mount!(app, {
        routes::welcome::WELCOME => get(app.welcome, Welcome::show),
    });
    router.with_state(app)
}

pub fn export_payloads() -> Result<(), Box<dyn std::error::Error>> {
    WelcomeProps::export()?;
    Ok(())
}
"#
    }

    #[test]
    fn wires_generated_controller_into_lib() {
        let resource = Resource::parse("Widget", None);
        let actions = ["index", "create"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let updated = wire(sample_lib(), &tmpl::controller_wiring(&resource, &actions)).unwrap();
        assert!(updated.contains("use crate::controllers::widgets::Widgets;"));
        assert!(updated.contains("pub widgets: Widgets,"));
        assert!(updated.contains("widgets: Widgets {"));
        assert!(
            updated.contains("routes::widgets::WIDGETS_INDEX => get(app.widgets, Widgets::index),")
        );
        assert!(updated.contains("crate::controllers::widgets::WidgetProps::export()?;"));
        assert!(updated.contains("crate::controllers::widgets::IndexProps::export()?;"));
    }
}
