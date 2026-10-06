use std::{collections::BTreeMap, fs, io::Write, path::Path};

use gurthang_project::find_root;

use crate::{Error, region, registration, tmpl};

pub fn sync(check: bool, out: &mut impl Write) -> Result<(), Error> {
    let root = find_root()?;
    let directory = root.join("src/routes");
    if !directory.is_dir() {
        return Err(Error::Message("src/routes is missing".into()));
    }
    let mut catalog = Vec::new();
    collect_catalog(&directory, &mut catalog)?;
    let methods = collect_methods(&root)?;
    let mut routes = Vec::new();
    for route in catalog {
        let Some(method) = methods.get(&route.ident) else {
            continue;
        };
        routes.push(JsRoute {
            name: route.name,
            method: method.clone(),
            path: route.path,
        });
    }
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
            return Err(Error::Message(
                "resources/js/routes.ts is out of date".into(),
            ));
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
}

fn collect_methods(root: &Path) -> Result<BTreeMap<String, String>, Error> {
    let mut methods = BTreeMap::new();
    collect_mounts(&root.join("src/controllers"), &mut methods)?;
    Ok(methods)
}

fn collect_mounts(directory: &Path, methods: &mut BTreeMap<String, String>) -> Result<(), Error> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_mounts(&path, methods)?;
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        for (ident, verb) in parse_mount(&fs::read_to_string(&path)?) {
            methods.insert(ident, verb);
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ListedRoute {
    pub name: String,
    pub method: String,
    pub path: String,
}

pub fn listed_routes(root: &Path) -> Result<Vec<ListedRoute>, Error> {
    let directory = root.join("src/routes");
    if !directory.is_dir() {
        return Err(Error::Message("src/routes is missing".into()));
    }
    let mut catalog = Vec::new();
    collect_catalog(&directory, &mut catalog)?;
    let methods = collect_methods(root)?;
    let mut routes = Vec::new();
    for route in catalog {
        let Some(method) = methods.get(&route.ident) else {
            continue;
        };
        routes.push(ListedRoute {
            name: route.name,
            method: method.clone(),
            path: route.path,
        });
    }
    routes.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(routes)
}

pub fn print(out: &mut impl Write) -> Result<(), Error> {
    let root = find_root()?;
    let routes = listed_routes(&root)?;
    if routes.is_empty() {
        writeln!(out, "No mounted routes.")?;
        return Ok(());
    }
    let name_width = routes.iter().map(|route| route.name.len()).max().unwrap_or(8);
    for route in routes {
        writeln!(
            out,
            "{:<name_width$} {:<7} {}",
            route.name, route.method, route.path
        )?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct CatalogRoute {
    ident: String,
    name: String,
    path: String,
}

#[derive(Clone, Debug)]
struct JsRoute {
    name: String,
    method: String,
    path: String,
}

fn collect_catalog(directory: &Path, routes: &mut Vec<CatalogRoute>) -> Result<(), Error> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_catalog(&path, routes)?;
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        parse_catalog(&fs::read_to_string(&path)?, routes);
    }
    Ok(())
}

fn parse_catalog(source: &str, routes: &mut Vec<CatalogRoute>) {
    let mut ident = None;
    let mut name = None;
    let mut path = None;
    let mut in_route = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(found) = const_ident(trimmed) {
            ident = Some(found);
        }
        if trimmed.contains("Route {") {
            in_route = true;
            name = None;
            path = None;
            continue;
        }
        if !in_route {
            continue;
        }
        if let Some(value) = field_value(trimmed, "name") {
            name = Some(value);
        } else if let Some(value) = field_value(trimmed, "path") {
            path = Some(value);
        }
        if trimmed.starts_with("};") || trimmed == "}" {
            if let (Some(ident), Some(name), Some(path)) = (ident.take(), name.take(), path.take())
            {
                routes.push(CatalogRoute { ident, name, path });
            }
            in_route = false;
        }
    }
}

fn parse_mount(source: &str) -> BTreeMap<String, String> {
    let mut methods = BTreeMap::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let Some((ident, verb)) = mount_binding(trimmed) else {
            continue;
        };
        methods.insert(ident, verb.to_uppercase());
    }
    methods
}

fn mount_binding(line: &str) -> Option<(String, String)> {
    let (left, right) = line.split_once("=>")?;
    let ident = left.trim().rsplit("::").next()?.trim().to_owned();
    if ident.is_empty()
        || !ident
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let verb = right.trim().split('(').next()?.trim();
    if !matches!(verb, "get" | "post" | "put" | "patch" | "delete") {
        return None;
    }
    Some((ident, verb.to_owned()))
}

fn const_ident(line: &str) -> Option<String> {
    let rest = line.strip_prefix("pub const ")?;
    let ident = rest.split(':').next()?.trim();
    if ident.is_empty() {
        return None;
    }
    Some(ident.to_owned())
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
    fn parses_route_constants_and_mount_methods() {
        let source = r#"
pub const LOGIN: Route = Route {
    name: "login",
    path: "/login",
};
"#;
        let mut routes = Vec::new();
        parse_catalog(source, &mut routes);
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].ident, "LOGIN");
        assert_eq!(routes[0].name, "login");
        assert_eq!(routes[0].path, "/login");

        let methods = parse_mount(
            r#"
    mount!(auth, {
        crate::routes::auth::LOGIN => get(auth, Auth::new_login),
        crate::routes::auth::LOGIN_CREATE => post(auth, Auth::login),
    });
"#,
        );
        assert_eq!(methods.get("LOGIN").map(String::as_str), Some("GET"));
        assert_eq!(
            methods.get("LOGIN_CREATE").map(String::as_str),
            Some("POST")
        );
    }
}
