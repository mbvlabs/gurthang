use std::{fs, io::Write};

use gurthang_project::find_root;
use heck::{ToPascalCase, ToSnakeCase};

use crate::{Error, GenerateOptions};

pub fn generate(name: &str, options: GenerateOptions, out: &mut impl Write) -> Result<(), Error> {
    let root = find_root()?;
    let tasks = root.join("src/tasks/mod.rs");
    let app = root.join("src/app.rs");
    if !tasks.exists() {
        return Err(Error::Message("src/tasks/mod.rs is missing".into()));
    }
    if !app.exists() {
        return Err(Error::Message("src/app.rs is missing".into()));
    }
    let pascal = name.to_pascal_case();
    let snake = name.to_snake_case();
    let module = root.join(format!("src/tasks/{snake}.rs"));
    if module.exists() {
        return Err(Error::Message(format!(
            "src/tasks/{snake}.rs already exists"
        )));
    }
    if options.dry_run {
        writeln!(out, "Would write src/tasks/{snake}.rs")?;
        writeln!(out, "Would register tasks::{snake} in src/app.rs")?;
        return Ok(());
    }
    fs::create_dir_all(root.join("src/tasks"))?;
    fs::write(&module, task_source(&pascal, &snake))?;
    let mut source = fs::read_to_string(&tasks)?;
    source = ensure_mod(&source, &snake);
    fs::write(&tasks, source)?;
    let current = fs::read_to_string(&app)?;
    fs::write(&app, wire_register_tasks(&current, &snake, &pascal)?)?;
    writeln!(out, "Wrote src/tasks/{snake}.rs")?;
    writeln!(out, "Registered tasks::{snake} in src/app.rs")?;
    Ok(())
}

fn ensure_mod(source: &str, snake: &str) -> String {
    let line = format!("pub mod {snake};");
    if source.contains(&line) {
        return source.to_owned();
    }
    if source.trim().is_empty() {
        return format!("{line}\n");
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

fn task_source(pascal: &str, snake: &str) -> String {
    format!(
        r#"use std::collections::BTreeMap;

use gurthang::prelude::*;
use gurthang::Result;

pub struct {pascal};

#[async_trait]
impl Task for {pascal} {{
    fn info(&self) -> TaskInfo {{
        TaskInfo {{
            name: "{snake}".into(),
            detail: "Generated task".into(),
        }}
    }}

    async fn run(&self, ctx: &Context, vars: &BTreeMap<String, String>) -> Result<()> {{
        let _ = (ctx, vars);
        Ok(())
    }}
}}
"#
    )
}

fn wire_register_tasks(source: &str, snake: &str, pascal: &str) -> Result<String, Error> {
    let call = format!("tasks.register(crate::tasks::{snake}::{pascal});");
    if source.contains(&call) {
        return Ok(source.to_owned());
    }
    let source = source.replace(
        "fn register_tasks(_tasks: &mut Tasks)",
        "fn register_tasks(tasks: &mut Tasks)",
    );
    if let Some(idx) = source.find("fn register_tasks(tasks: &mut Tasks) {}") {
        let end = idx + "fn register_tasks(tasks: &mut Tasks) {}".len();
        return Ok(format!(
            "{}fn register_tasks(tasks: &mut Tasks) {{\n        {call}\n    }}{}",
            &source[..idx],
            &source[end..]
        ));
    }
    if let Some(last) = source.rfind("tasks.register(") {
        let line_end = source[last..]
            .find('\n')
            .map(|offset| last + offset + 1)
            .unwrap_or(source.len());
        let indent = source[source[..last].rfind('\n').map(|i| i + 1).unwrap_or(0)..last]
            .chars()
            .take_while(|ch| ch.is_whitespace())
            .collect::<String>();
        let mut updated = String::new();
        updated.push_str(&source[..line_end]);
        updated.push_str(&indent);
        updated.push_str(&call);
        updated.push('\n');
        updated.push_str(&source[line_end..]);
        return Ok(updated);
    }
    Err(Error::Message(
        "src/app.rs is missing register_tasks".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_empty_register_tasks() {
        let source = "    fn register_tasks(_tasks: &mut Tasks) {}\n";
        let updated = wire_register_tasks(source, "foo", "Foo").unwrap();
        assert!(updated.contains("fn register_tasks(tasks: &mut Tasks)"));
        assert!(updated.contains("tasks.register(crate::tasks::foo::Foo);"));
        assert!(!updated.contains("(_tasks:"));
    }

    #[test]
    fn appends_a_second_task() {
        let source = r#"    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::foo::Foo);
    }
"#;
        let updated = wire_register_tasks(source, "bar", "Bar").unwrap();
        assert!(updated.contains("tasks.register(tasks::foo::Foo);"));
        assert!(updated.contains("tasks.register(crate::tasks::bar::Bar);"));
        assert!(updated.find("bar").unwrap() > updated.find("foo").unwrap());
    }
}
