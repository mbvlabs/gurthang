use std::{fs, path::Path};

use include_dir::{Dir, DirEntry, include_dir};

use crate::{
    error::{Error, Result},
    project_name::ProjectName,
};

pub const PROJECT_TOKEN: &str = "__GURTHANG_PROJECT_NAME__";
pub const CRATE_TOKEN: &str = "__GURTHANG_CRATE_NAME__";
pub const PACKAGE_TOKEN: &str = "__GURTHANG_PACKAGE_NAME__";

static TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/project");

pub fn manifest() -> Vec<String> {
    let mut paths = Vec::new();
    visit_files(&TEMPLATE, &mut |path, _| paths.push(output_name(path)));
    paths.sort_unstable();
    paths
}

pub fn render(destination: &Path, name: &ProjectName) -> Result<()> {
    let mut result = Ok(());
    visit_files(&TEMPLATE, &mut |path, contents| {
        if result.is_err() {
            return;
        }
        result = render_file(destination, path, contents, name);
    });
    result
}

fn visit_files<'a>(directory: &'a Dir<'a>, visitor: &mut impl FnMut(&Path, &'a [u8])) {
    for entry in directory.entries() {
        match entry {
            DirEntry::Dir(child) => visit_files(child, visitor),
            DirEntry::File(file) => visitor(file.path(), file.contents()),
        }
    }
}

fn render_file(
    destination: &Path,
    source_path: &Path,
    contents: &[u8],
    name: &ProjectName,
) -> Result<()> {
    let relative = output_name(source_path);
    let output = destination.join(&relative);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| Error::io(format!("could not create {}", parent.display()), error))?;
    }

    let bytes = if is_text_template(source_path) {
        render_text(source_path, contents, name)?.into_bytes()
    } else {
        contents.to_vec()
    };
    fs::write(&output, bytes)
        .map_err(|error| Error::io(format!("could not write {}", output.display()), error))?;
    make_executable(&relative, &output)?;
    Ok(())
}

#[cfg(unix)]
fn make_executable(relative: &str, output: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    if relative == "bin/install-tailwindcli" {
        let mut permissions = fs::metadata(output)
            .map_err(|error| Error::io(format!("could not inspect {}", output.display()), error))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(output, permissions).map_err(|error| {
            Error::io(
                format!("could not make {} executable", output.display()),
                error,
            )
        })?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_relative: &str, _output: &Path) -> Result<()> {
    Ok(())
}

fn output_name(source_path: &Path) -> String {
    let path = source_path.to_string_lossy();
    path.strip_suffix(".gurthang")
        .map(str::to_owned)
        .unwrap_or_else(|| path.into_owned())
}

fn is_text_template(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if matches!(name, ".env.example" | ".gitignore.gurthang") {
        return true;
    }
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some(
            "gurthang"
                | "css"
                | "html"
                | "js"
                | "json"
                | "md"
                | "rs"
                | "sql"
                | "ts"
                | "tsx"
                | "toml"
                | "yml"
                | "yaml"
        )
    )
}

fn render_text(path: &Path, contents: &[u8], name: &ProjectName) -> Result<String> {
    let text = std::str::from_utf8(contents).map_err(|error| {
        Error::io(
            format!("template {} is not UTF-8", path.display()),
            std::io::Error::new(std::io::ErrorKind::InvalidData, error),
        )
    })?;
    let rendered = text
        .replace(PROJECT_TOKEN, name.project_name())
        .replace(CRATE_TOKEN, name.crate_name())
        .replace(PACKAGE_TOKEN, name.package_name());

    if [PROJECT_TOKEN, CRATE_TOKEN, PACKAGE_TOKEN]
        .iter()
        .any(|token| rendered.contains(token))
    {
        return Err(Error::UnknownPlaceholder {
            path: path.display().to_string(),
        });
    }
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_is_sorted() {
        let manifest = manifest();
        let mut expected = manifest.clone();
        expected.sort_unstable();
        assert_eq!(manifest, expected);
    }

    #[test]
    fn non_text_template_bytes_are_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let name = ProjectName::parse("demo").unwrap();
        let bytes = [0, 159, 146, 150, 255];
        render_file(temp.path(), Path::new("fixture.bin"), &bytes, &name).unwrap();
        assert_eq!(fs::read(temp.path().join("fixture.bin")).unwrap(), bytes);
    }
}
