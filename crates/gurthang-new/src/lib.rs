mod error;
mod renderer;

pub use error::{Error, Result};
pub use renderer::manifest;

use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

use gurthang_project::ProjectName;
use tempfile::Builder;

pub fn execute(
    name: &str,
    path: Option<PathBuf>,
    dry_run: bool,
    source_root: &Path,
    out: &mut impl Write,
) -> Result<()> {
    let name = ProjectName::parse(name)?;
    let destination = absolute_destination(path.unwrap_or_else(|| PathBuf::from(name.project_name())))?;

    validate_destination(&destination)?;
    if dry_run {
        writeln!(
            out,
            "Would create {} at {}",
            name.project_name(),
            destination.display()
        )?;
        for path in renderer::manifest() {
            writeln!(out, "  {path}")?;
        }
        return Ok(());
    }

    let parent = destination
        .parent()
        .ok_or_else(|| Error::DestinationExists("destination has no parent directory".into()))?;
    fs::create_dir_all(parent)
        .map_err(|error| Error::io(format!("could not create {}", parent.display()), error))?;

    let temporary = Builder::new()
        .prefix(".gurthang-")
        .tempdir_in(parent)
        .map_err(|error| Error::io("could not create temporary project directory", error))?;
    let source = source_reference(&destination, source_root);
    renderer::render(temporary.path(), &name, &source)?;

    let destination_was_empty = destination.is_dir();
    if destination_was_empty {
        fs::remove_dir(&destination).map_err(|error| {
            Error::io(
                format!(
                    "could not prepare empty destination {}",
                    destination.display()
                ),
                error,
            )
        })?;
    }

    let temporary_path = temporary.keep();
    if let Err(error) = fs::rename(&temporary_path, &destination) {
        let _ = fs::remove_dir_all(&temporary_path);
        if destination_was_empty {
            let _ = fs::create_dir(&destination);
        }
        return Err(Error::io(
            format!("could not move project into {}", destination.display()),
            error,
        ));
    }

    writeln!(
        out,
        "Created {} at {}",
        name.project_name(),
        destination.display()
    )?;
    writeln!(out)?;
    writeln!(out, "Next:")?;
    writeln!(out, "  cd {}", destination.display())?;
    writeln!(out, "  cp .env.example .env")?;
    writeln!(out, "  npm install")?;
    writeln!(out, "  gurthang db create")?;
    writeln!(out, "  gurthang db migrate up")?;
    writeln!(out, "  gurthang run")?;
    Ok(())
}

fn absolute_destination(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }
    std::env::current_dir()
        .map(|current| current.join(path))
        .map_err(|error| Error::io("could not determine current directory", error))
}

/// Reference to the Gurthang checkout written into the generated Cargo.toml.
/// Prefers a path relative to the new project so the app keeps working when
/// the checkout and the project move together; falls back to an absolute path
/// when no shared root exists.
fn source_reference(destination: &Path, source_root: &Path) -> String {
    let source = source_root
        .canonicalize()
        .unwrap_or_else(|_| absolute(source_root));
    let destination = destination
        .canonicalize()
        .unwrap_or_else(|_| absolute(destination));
    relative_path(&destination, &source)
        .unwrap_or(source)
        .display()
        .to_string()
}

fn absolute(path: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn relative_path(base: &Path, target: &Path) -> Option<PathBuf> {
    let base: Vec<_> = base.components().collect();
    let target: Vec<_> = target.components().collect();
    if base.first() != target.first() {
        return None;
    }
    let common = base.iter().zip(&target).take_while(|(a, b)| a == b).count();
    let mut relative = PathBuf::new();
    for _ in common..base.len() {
        relative.push("..");
    }
    for component in &target[common..] {
        relative.push(component.as_os_str());
    }
    if relative.as_os_str().is_empty() {
        return None;
    }
    Some(relative)
}

fn validate_destination(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_file() {
        return Err(Error::DestinationExists(format!(
            "destination {} is an existing file",
            path.display()
        )));
    }
    let mut entries = fs::read_dir(path)
        .map_err(|error| Error::io(format!("could not inspect {}", path.display()), error))?;
    if entries
        .next()
        .transpose()
        .map_err(|error| Error::io(format!("could not inspect {}", path.display()), error))?
        .is_some()
    {
        return Err(Error::DestinationExists(format!(
            "destination {} is not empty",
            path.display()
        )));
    }
    Ok(())
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::io("write failed", error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_project_and_substitutes_names() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("output");
        let mut output = Vec::new();
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        execute(
            "my-app",
            Some(destination.clone()),
            false,
            &source,
            &mut output,
        )
        .unwrap();

        let cargo = fs::read_to_string(destination.join("Cargo.toml")).unwrap();
        let main = fs::read_to_string(destination.join("src/main.rs")).unwrap();
        assert!(cargo.contains("name = \"my-app\""));
        assert!(main.contains("my_app::run"));
        assert!(!cargo.contains("__GURTHANG_"));
        assert!(destination.join(".gitignore").is_file());
        assert!(destination.join("gurthang.toml").is_file());
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("  gurthang run\n"));
        assert!(!output.contains("  cargo run\n"));
    }

    #[test]
    fn dependency_paths_are_relative_to_the_project() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("output");
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        execute(
            "my-app",
            Some(destination.clone()),
            false,
            &source,
            &mut Vec::new(),
        )
        .unwrap();

        let cargo = fs::read_to_string(destination.join("Cargo.toml")).unwrap();
        let source = source.canonicalize().unwrap();
        assert!(!cargo.contains(&format!("path = \"{}/crates", source.display())));
        assert!(cargo.contains("gurthang-http = { path = \"../"));
    }

    #[test]
    fn dry_run_writes_nothing_and_prints_sorted_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("output");
        let mut output = Vec::new();
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        execute(
            "demo",
            Some(destination.clone()),
            true,
            &source,
            &mut output,
        )
        .unwrap();
        assert!(!destination.exists());
        let output = String::from_utf8(output).unwrap();
        let listed = output.lines().skip(1).map(str::trim).collect::<Vec<_>>();
        assert_eq!(listed, renderer::manifest());
    }
}
