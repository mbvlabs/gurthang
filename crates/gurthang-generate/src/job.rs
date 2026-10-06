use std::{fs, io::Write, path::Path};

use gurthang_project::find_root;
use heck::{ToPascalCase, ToSnakeCase};

use crate::{Error, GenerateOptions, registration, tmpl};

pub fn generate(name: &str, options: GenerateOptions, out: &mut impl Write) -> Result<(), Error> {
    let root = find_root()?;
    let workers = root.join("src/workers/mod.rs");
    if !workers.exists() {
        return Err(Error::Message("src/workers/mod.rs is missing".into()));
    }
    let app = root.join("src/app.rs");
    if !app.exists() {
        return Err(Error::Message("src/app.rs is missing".into()));
    }
    let pascal = name.to_pascal_case();
    let snake = name.to_snake_case();
    let worker_file = root.join(format!("src/workers/{snake}.rs"));
    if worker_file.exists() {
        return Err(Error::Message(format!(
            "src/workers/{snake}.rs already exists"
        )));
    }
    if options.dry_run {
        writeln!(out, "Would write src/workers/{snake}.rs")?;
        writeln!(out, "Would rewrite src/workers/generated.rs")?;
        writeln!(out, "Would register workers::{snake} in src/app.rs")?;
        return Ok(());
    }
    fs::create_dir_all(root.join("src/workers"))?;
    fs::write(&worker_file, worker_source(&pascal, &snake))?;
    let mut source = fs::read_to_string(&workers)?;
    source = ensure_worker_mod(&source, &snake);
    fs::write(&workers, source)?;
    rewrite_generated(&root, false)?;
    register_in_app(&app, &snake)?;
    writeln!(out, "Wrote src/workers/{snake}.rs")?;
    writeln!(out, "Rewrote src/workers/generated.rs")?;
    writeln!(out, "Registered workers::{snake} in src/app.rs")?;
    Ok(())
}

fn rewrite_generated(root: &Path, check: bool) -> Result<(), Error> {
    let directory = root.join("src/workers");
    let jobs = scan_jobs(&directory)?;
    let contents = tmpl::workers_generated(&jobs)?;
    registration::write_generated(&directory.join("generated.rs"), &contents, check)
}

fn scan_jobs(directory: &Path) -> Result<Vec<tmpl::JobModule>, Error> {
    Ok(
        registration::rust_modules(directory, &["mod", "generated"])?
            .into_iter()
            .map(|snake| tmpl::JobModule {
                pascal: snake.to_pascal_case(),
                snake,
            })
            .collect(),
    )
}

fn ensure_worker_mod(source: &str, snake: &str) -> String {
    let line = format!("pub mod {snake};");
    if source.contains(&line) {
        return source.to_owned();
    }
    if let Some(last) = source.rfind("pub mod ") {
        let line_end = source[last..]
            .find('\n')
            .map(|offset| last + offset + 1)
            .unwrap_or(source.len());
        let mut updated = String::new();
        updated.push_str(&source[..line_end]);
        updated.push_str(&line);
        updated.push('\n');
        updated.push_str(&source[line_end..]);
        updated
    } else {
        format!("{line}\n{source}")
    }
}

fn worker_source(pascal: &str, snake: &str) -> String {
    format!(
        r#"use gurthang::jobs::{{PerformJob, Result}};
use gurthang::Context;
use serde::{{Deserialize, Serialize}};
use sqlx::PgPool;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct {pascal};

impl PerformJob for {pascal} {{
    fn name(&self) -> &'static str {{
        "{snake}"
    }}

    async fn perform(self, database: &PgPool) -> Result<()> {{
        let _ = database;
        Ok(())
    }}
}}

pub fn register(_ctx: &Context) {{}}
"#
    )
}

fn register_in_app(path: &std::path::Path, snake: &str) -> Result<(), Error> {
    let source = fs::read_to_string(path)?;
    let call = format!("workers::{snake}::register(ctx);");
    if source.contains(&call) {
        return Ok(());
    }
    let updated = append_register(&source, &call)?;
    fs::write(path, updated)?;
    Ok(())
}

fn append_register(source: &str, call: &str) -> Result<String, Error> {
    let marker = "::register(ctx);";
    if let Some(last) = source.rfind(marker) {
        let line_end = source[last..]
            .find('\n')
            .map(|offset| last + offset + 1)
            .unwrap_or(source.len());
        let indent = line_indent(
            source,
            source[..last].rfind('\n').map(|i| i + 1).unwrap_or(0),
        );
        let mut updated = String::new();
        updated.push_str(&source[..line_end]);
        updated.push_str(&indent);
        updated.push_str(call);
        if !call.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(&source[line_end..]);
        return Ok(updated);
    }
    let Some(start) = source.find("async fn connect_workers(") else {
        return Err(Error::Message(
            "src/app.rs is missing connect_workers".into(),
        ));
    };
    let rest = &source[start..];
    let Some(brace) = rest.find('{') else {
        return Err(Error::Message("could not find connect_workers body".into()));
    };
    let insert_at = start + brace + 1;
    let mut updated = String::new();
    updated.push_str(&source[..insert_at]);
    updated.push('\n');
    updated.push_str("        ");
    updated.push_str(call);
    updated.push('\n');
    updated.push_str(&source[insert_at..]);
    Ok(updated)
}

fn line_indent(source: &str, line_start: usize) -> String {
    source[line_start..]
        .chars()
        .take_while(|ch| ch.is_whitespace() && *ch != '\n')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_register_after_the_last_call() {
        let source = r#"
    async fn connect_workers(ctx: &Context) -> Result<()> {
        workers::purge_expired_sessions::register(ctx);
        Ok(())
    }
"#;
        let updated = append_register(source, "workers::send_welcome::register(ctx);").unwrap();
        assert!(updated.contains("workers::purge_expired_sessions::register(ctx);"));
        assert!(updated.contains("workers::send_welcome::register(ctx);"));
        assert!(
            updated.find("send_welcome").unwrap() > updated.find("purge_expired_sessions").unwrap()
        );
    }

    #[test]
    fn inserts_worker_mod_before_the_include() {
        let source = "pub mod purge_expired_sessions;\n\ninclude!(\"generated.rs\");\n";
        let updated = ensure_worker_mod(source, "send_welcome");
        assert!(updated.contains("pub mod send_welcome;"));
        assert!(
            updated.find("pub mod send_welcome;").unwrap()
                < updated.find("include!(\"generated.rs\")").unwrap()
        );
    }

    #[test]
    fn rewrites_job_enum_from_worker_files() {
        let directory = tempfile::tempdir().unwrap();
        let workers = directory.path().join("src/workers");
        fs::create_dir_all(&workers).unwrap();
        fs::write(
            workers.join("mod.rs"),
            "pub mod purge_expired_sessions;\ninclude!(\"generated.rs\");\n",
        )
        .unwrap();
        fs::write(workers.join("purge_expired_sessions.rs"), "").unwrap();
        fs::write(workers.join("send_welcome.rs"), "").unwrap();
        rewrite_generated(directory.path(), false).unwrap();
        let source = fs::read_to_string(workers.join("generated.rs")).unwrap();
        assert!(source.contains("PurgeExpiredSessions,"));
        assert!(source.contains("SendWelcome,"));
        assert!(source.contains("Mailer(Email)"));
        assert!(source.contains("send_welcome::SendWelcome"));
        assert!(!source.contains("gurthang:generated"));
    }
}
