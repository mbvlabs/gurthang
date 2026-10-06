use std::{fs, io::Write};

use gurthang_project::find_root;
use heck::{ToPascalCase, ToSnakeCase};

use crate::{Error, GenerateOptions};

pub fn generate(name: &str, options: GenerateOptions, out: &mut impl Write) -> Result<(), Error> {
    let root = find_root()?;
    let mailers = root.join("src/mailers/mod.rs");
    if !mailers.exists() {
        return Err(Error::Message("src/mailers/mod.rs is missing".into()));
    }
    let pascal = name.to_pascal_case();
    let snake = name.to_snake_case();
    let module = root.join(format!("src/mailers/{snake}.rs"));
    if module.exists() {
        return Err(Error::Message(format!(
            "src/mailers/{snake}.rs already exists"
        )));
    }
    if options.dry_run {
        writeln!(out, "Would write src/mailers/{snake}.rs")?;
        writeln!(out, "Would write src/mailers/{snake}/html.html")?;
        writeln!(out, "Would write src/mailers/{snake}/text.txt")?;
        return Ok(());
    }
    fs::create_dir_all(root.join(format!("src/mailers/{snake}")))?;
    fs::write(&module, mailer_source(&pascal, &snake))?;
    fs::write(
        root.join(format!("src/mailers/{snake}/html.html")),
        format!("<p>Hello from {pascal}, {{{{ name }}}}.</p>\n"),
    )?;
    fs::write(
        root.join(format!("src/mailers/{snake}/text.txt")),
        format!("Hello from {pascal}, {{{{ name }}}}.\n"),
    )?;
    let mut source = fs::read_to_string(&mailers)?;
    source = ensure_mod(&source, &snake);
    fs::write(&mailers, source)?;
    writeln!(out, "Wrote src/mailers/{snake}.rs")?;
    writeln!(out, "Wrote src/mailers/{snake}/html.html")?;
    writeln!(out, "Wrote src/mailers/{snake}/text.txt")?;
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

fn mailer_source(pascal: &str, snake: &str) -> String {
    format!(
        r#"use askama::Template;
use gurthang::prelude::*;
use gurthang::Result;

pub struct {pascal};

impl Mailer for {pascal} {{}}

#[derive(Template)]
#[template(path = "{snake}/html.html")]
struct Html<'a> {{
    name: &'a str,
}}

#[derive(Template)]
#[template(path = "{snake}/text.txt")]
struct Text<'a> {{
    name: &'a str,
}}

impl {pascal} {{
    pub async fn send(ctx: &Context, to: &str, name: &str) -> Result<()> {{
        let html = Html {{ name }}
            .render()
            .map_err(|error| Error::Message(error.to_string()))?;
        let text = Text {{ name }}
            .render()
            .map_err(|error| Error::Message(error.to_string()))?;
        Self::mail(
            ctx,
            &Email {{
                to: to.to_owned(),
                subject: "{pascal}".into(),
                html,
                text,
                ..Default::default()
            }},
        )
        .await
    }}
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepends_mailer_mod() {
        let updated = ensure_mod("pub mod auth;\n", "notice");
        assert!(updated.contains("pub mod auth;"));
        assert!(updated.contains("pub mod notice;"));
    }
}
