use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

pub fn find_root() -> Result<PathBuf> {
    let current = std::env::current_dir()
        .map_err(|error| Error::io("could not determine current directory", error))?;
    find_root_from(&current)
}

pub fn find_root_from(start: &Path) -> Result<PathBuf> {
    for directory in start.ancestors() {
        if directory.join("gurthang.toml").is_file() && directory.join("Cargo.toml").is_file() {
            return Ok(directory.to_path_buf());
        }
    }
    Err(Error::NotAProject)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_project_root_from_a_nested_directory() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("Cargo.toml"), "[package]").unwrap();
        std::fs::write(directory.path().join("gurthang.toml"), "schema_version = 1\n").unwrap();
        let nested = directory.path().join("src/controllers");
        std::fs::create_dir_all(&nested).unwrap();
        assert_eq!(find_root_from(&nested).unwrap(), directory.path());
    }

    #[test]
    fn rejects_directories_outside_an_application() {
        let directory = tempfile::tempdir().unwrap();
        assert!(matches!(
            find_root_from(directory.path()),
            Err(Error::NotAProject)
        ));
    }
}
