use std::{fs, path::Path};

use crate::{Error, tmpl};

pub fn rewrite(root: &Path, check: bool) -> Result<(), Error> {
    let controllers = tmpl::controllers_mod(&scan_controllers(&root.join("src/controllers"))?);
    let routes = tmpl::routes_generated(&scan_routes(&root.join("src/routes"))?);
    write_generated(&root.join("src/controllers/mod.rs"), &controllers, check)?;
    write_generated(&root.join("src/routes/generated.rs"), &routes, check)?;
    Ok(())
}

fn scan_controllers(directory: &Path) -> Result<Vec<tmpl::ControllerModule>, Error> {
    let mut modules = Vec::new();
    for name in rust_modules(directory, &["mod"])? {
        modules.push(tmpl::ControllerModule { name });
    }
    Ok(modules)
}

fn scan_routes(directory: &Path) -> Result<Vec<String>, Error> {
    rust_modules(directory, &["mod", "generated"])
}

pub(crate) fn rust_modules(directory: &Path, skip: &[&str]) -> Result<Vec<String>, Error> {
    let mut names = Vec::new();
    if !directory.is_dir() {
        return Ok(names);
    }
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|name| name.to_str()) else {
            continue;
        };
        if skip.contains(&name) {
            continue;
        }
        names.push(name.to_owned());
    }
    names.sort();
    Ok(names)
}

pub(crate) fn write_generated(path: &Path, contents: &str, check: bool) -> Result<(), Error> {
    if check {
        if path.exists() && fs::read_to_string(path)? != contents {
            return Err(Error::Message(format!("{} is out of date", path.display())));
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}
