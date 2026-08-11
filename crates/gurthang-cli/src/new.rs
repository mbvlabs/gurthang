use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use tempfile::Builder;

use crate::{
    cli::NewArgs,
    error::{Error, Result},
    project_name::ProjectName,
    renderer,
};

pub fn execute(args: NewArgs, out: &mut impl Write) -> Result<()> {
    let name = ProjectName::parse(&args.name)?;
    let destination = absolute_destination(args.path.unwrap_or_else(|| PathBuf::from(&args.name)))?;

    validate_destination(&destination)?;
    if args.dry_run {
        writeln!(
            out,
            "Would create {} at {}",
            name.project_name(),
            destination.display()
        )
        .map_err(|error| Error::io("could not write dry-run output", error))?;
        for path in renderer::manifest() {
            writeln!(out, "  {path}")
                .map_err(|error| Error::io("could not write dry-run manifest", error))?;
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
    renderer::render(temporary.path(), &name)?;

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
    )
    .and_then(|_| writeln!(out))
    .and_then(|_| writeln!(out, "Next:"))
    .and_then(|_| writeln!(out, "  cd {}", destination.display()))
    .and_then(|_| writeln!(out, "  cp .env.example .env"))
    .and_then(|_| writeln!(out, "  ./bin/install-tailwindcli"))
    .and_then(|_| writeln!(out, "  npm install"))
    .and_then(|_| writeln!(out, "  npm run css:build"))
    .and_then(|_| writeln!(out, "  sqlx migrate run"))
    .and_then(|_| writeln!(out, "  gurthang run"))
    .map_err(|error| Error::io("could not write success output", error))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn args(name: &str, path: PathBuf, dry_run: bool) -> NewArgs {
        NewArgs {
            name: name.into(),
            path: Some(path),
            dry_run,
        }
    }

    #[test]
    fn creates_a_project_and_substitutes_names() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("output");
        let mut output = Vec::new();
        execute(args("my-app", destination.clone(), false), &mut output).unwrap();

        let cargo = fs::read_to_string(destination.join("Cargo.toml")).unwrap();
        let main = fs::read_to_string(destination.join("src/main.rs")).unwrap();
        assert!(cargo.contains("name = \"my-app\""));
        assert!(main.contains("use my_app::"));
        assert!(!cargo.contains("__GURTHANG_"));
        assert!(destination.join(".gitignore").is_file());
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("  gurthang run\n"));
        assert!(!output.contains("  cargo run\n"));
    }

    #[test]
    fn permits_an_existing_empty_directory() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("output");
        fs::create_dir(&destination).unwrap();
        execute(args("demo", destination.clone(), false), &mut Vec::new()).unwrap();
        assert!(destination.join("Cargo.toml").is_file());
    }

    #[test]
    fn refuses_files_and_non_empty_directories() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("file");
        fs::write(&file, "mine").unwrap();
        assert!(execute(args("demo", file.clone(), false), &mut Vec::new()).is_err());
        assert_eq!(fs::read_to_string(file).unwrap(), "mine");

        let directory = temp.path().join("directory");
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("mine"), "mine").unwrap();
        assert!(execute(args("demo", directory.clone(), false), &mut Vec::new()).is_err());
        assert!(directory.join("mine").is_file());
    }

    #[test]
    fn dry_run_writes_nothing_and_prints_sorted_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("output");
        let mut output = Vec::new();
        execute(args("demo", destination.clone(), true), &mut output).unwrap();
        assert!(!destination.exists());

        let output = String::from_utf8(output).unwrap();
        let listed = output.lines().skip(1).map(str::trim).collect::<Vec<_>>();
        assert_eq!(listed, renderer::manifest());
    }

    #[test]
    fn a_setup_failure_does_not_leave_a_partial_destination() {
        let temp = tempfile::tempdir().unwrap();
        let parent_file = temp.path().join("not-a-directory");
        fs::write(&parent_file, "mine").unwrap();
        let destination = parent_file.join("output");

        assert!(execute(args("demo", destination.clone(), false), &mut Vec::new()).is_err());
        assert!(!destination.exists());
        assert_eq!(fs::read_to_string(parent_file).unwrap(), "mine");
    }
}
