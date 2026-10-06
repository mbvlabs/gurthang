use std::{fs, io::Write, path::Path};

use gurthang_project::find_root;

use crate::{Error, region, registration, tmpl};

pub fn sync(check: bool, out: &mut impl Write) -> Result<(), Error> {
    let root = find_root()?;
    let directory = root.join("src/routes");
    if !directory.is_dir() {
        return Err(Error::Message("src/routes is missing".into()));
    }
    let mut routes = Vec::new();
    collect_routes(&directory, &mut routes)?;
    routes.sort_by(|left, right| left.name.cmp(&right.name));
    let contents = tmpl::js_routes(
        &routes
            .iter()
            .map(|route| (route.name.clone(), route.method.clone(), route.path.clone()))
            .collect::<Vec<_>>(),
    )?;
    let path = root.join("resources/js/routes.ts");
    if check {
        if path.exists() && fs::read_to_string(&path)? != contents {
            return Err(Error::Message("resources/js/routes.ts is out of date".into()));
        }
        registration::rewrite(&root, true)?;
        writeln!(out, "resources/js/routes.ts is current")?;
        return Ok(());
    }
    region::write_if_allowed(&path, &contents, false, true)?;
    registration::rewrite(&root, false)?;
    writeln!(out, "Wrote resources/js/routes.ts")?;
    writeln!(out, "Wrote src/controllers/mod.rs")?;
    writeln!(out, "Wrote src/routes/generated.rs")?;
    Ok(())


#[derive(Clone, Debug)]
struct JsRoute {
    name: String,
    method: String,
    path: String,
}

fn collect_routes(directory: &Path, routes: &mut Vec<JsRoute>) -> Result<(), Error> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_routes(&path, routes)?;
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        parse_routes(&fs::read_to_string(&path)?, routes);
    }
    Ok(())
}

fn parse_routes(source: &str, routes: &mut Vec<JsRoute>) {
    let mut name = None;
    let mut method = None;
    let mut path = None;
    let mut in_route = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.contains("Route {") {
            in_route = true;
            name = None;
            method = None;
            path = None;
            continue;
        }
        if !in_route {
            continue;
        }
        if let Some(value) = field_value(trimmed, "name") {
            name = Some(value);
        } else if let Some(value) = field_value(trimmed, "method") {
            method = Some(value);
        } else if let Some(value) = field_value(trimmed, "path") {
            path = Some(value);
        }
        if trimmed.starts_with("};") || trimmed == "}" {
            if let (Some(name), Some(method), Some(path)) = (name.take(), method.take(), path.take())
            {
                routes.push(JsRoute { name, method, path });
            }
            in_route = false;
        }
    }
}

fn field_value(line: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}:");
    let rest = line.trim().strip_prefix(&prefix)?.trim();
    let rest = rest.trim_end_matches(',');
    let rest = rest.strip_prefix('"')?.strip_suffix('"')?;
    Some(rest.to_owned())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_route_constants() {
        let source = r#"
pub const LOGIN: Route = Route {
    name: "login",
    method: "GET",
    path: "/login",
};
"#;
        let mut routes = Vec::new();
        parse_routes(source, &mut routes);
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].name, "login");
        assert_eq!(routes[0].path, "/login");
    }
}
