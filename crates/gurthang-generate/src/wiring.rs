use std::fs;
use std::path::Path;

use crate::{Error, naming::Resource, tmpl};

pub fn apply(root: &Path, resource: &Resource, actions: &[String]) -> Result<(), Error> {
    let path = root.join("src/app.rs");
    if !path.is_file() {
        return Err(Error::Message("src/app.rs is missing".into()));
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
    source = ensure_add_route(&source, &wiring.add_route)?;
    for export in &wiring.export_types {
        source = ensure_export(&source, export)?;
    }
    Ok(source)
}

fn ensure_add_route(source: &str, line: &str) -> Result<String, Error> {
    let trimmed = line.trim();
    if source.contains(trimmed) {
        return Ok(source.to_owned());
    }
    let needle = ".add_route(";
    let start = source.rfind(needle).ok_or_else(|| {
        Error::Message("src/app.rs is missing .add_route(...) in Hooks::routes".into())
    })?;
    let close = matching_paren(source, start + needle.len() - 1).ok_or_else(|| {
        Error::Message("could not find the end of the last add_route call".into())
    })?;
    let insert_at = close + 1;
    let indent = line_indent(source, start);
    let mut updated = String::new();
    updated.push_str(&source[..insert_at]);
    updated.push('\n');
    updated.push_str(&indent);
    updated.push_str(trimmed);
    updated.push_str(&source[insert_at..]);
    Ok(updated)
}

fn ensure_export(source: &str, ty: &str) -> Result<String, Error> {
    let call = format!("{ty}::export()");
    if source.contains(&call) {
        return Ok(source.to_owned());
    }
    let Some(start) = source.find("fn export_payloads(") else {
        return Err(Error::Message(
            "src/app.rs is missing export_payloads()".into(),
        ));
    };
    let rest = &source[start..];
    let Some(relative_ok) = rest.find("\n        Ok(())") else {
        return rest
            .find("\n    Ok(())")
            .map(|relative_ok| {
                insert_export(source, start + relative_ok + 1, ty)
            })
            .ok_or_else(|| Error::Message("could not find export_payloads return".into()));
    };
    Ok(insert_export(source, start + relative_ok + 1, ty))
}

fn insert_export(source: &str, index: usize, ty: &str) -> String {
    let mut updated = String::new();
    updated.push_str(&source[..index]);
    updated.push_str(&format!("        {ty}::export().map_err(|error| Error::Message(error.to_string()))?;\n"));
    updated.push_str(&source[index..]);
    updated
}

fn matching_paren(source: &str, open: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(open).copied() != Some(b'(') {
        return None;
    }
    let mut depth = 0;
    for (offset, byte) in bytes[open..].iter().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn line_indent(source: &str, index: usize) -> String {
    let start = source[..index].rfind('\n').map(|i| i + 1).unwrap_or(0);
    source[start..index]
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::naming::Resource;

    fn sample_app() -> &'static str {
        r#"
impl Hooks for App {
    fn routes(ctx: &Context) -> AppRoutes {
        AppRoutes::new()
            .add_route(controllers::welcome::routes(ctx))
            .add_route(controllers::auth::routes(ctx))
    }

    fn export_payloads() -> Result<()> {
        WelcomeProps::export().map_err(|error| Error::Message(error.to_string()))?;
        Ok(())
    }
}
"#
    }

    #[test]
    fn wires_generated_controller_into_app() {
        let resource = Resource::parse("Widget", None);
        let actions = ["index", "create"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let updated = wire(sample_app(), &tmpl::controller_wiring(&resource, &actions)).unwrap();
        assert!(updated.contains(".add_route(controllers::widgets::routes(ctx))"));
        assert!(updated.contains("crate::controllers::widgets::WidgetProps::export()"));
        assert!(updated.contains("crate::controllers::widgets::IndexProps::export()"));
        let routes_fn = updated
            .split("fn export_payloads")
            .next()
            .unwrap();
        let last_add = routes_fn.rfind(".add_route(").unwrap();
        assert!(
            routes_fn[last_add..].contains("controllers::widgets::routes(ctx)"),
            "generated add_route should be last: {}",
            &routes_fn[last_add..]
        );
    }
}
