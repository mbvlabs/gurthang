pub fn write_if_allowed(
    path: &std::path::Path,
    contents: &str,
    dry_run: bool,
    overwrite: bool,
) -> Result<bool, crate::Error> {
    if dry_run {
        return Ok(false);
    }
    if path.exists() && !overwrite {
        return Err(crate::Error::Message(format!(
            "{} already exists",
            path.display()
        )));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)?;
    Ok(true)
}

pub fn ensure_line(path: &std::path::Path, line: &str) -> Result<(), crate::Error> {
    let mut source = if path.exists() {
        std::fs::read_to_string(path)?
    } else {
        String::new()
    };
    if source.contains(line) {
        return Ok(());
    }
    if !source.is_empty() && !source.ends_with('\n') {
        source.push('\n');
    }
    source.push_str(line);
    source.push('\n');
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, source)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_a_missing_line() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("lib.rs");
        std::fs::write(&path, "pub mod user;\n").unwrap();
        ensure_line(&path, "pub mod generated;").unwrap();
        let source = std::fs::read_to_string(&path).unwrap();
        assert!(source.contains("pub mod user;"));
        assert!(source.contains("pub mod generated;"));
        ensure_line(&path, "pub mod generated;").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path)
                .unwrap()
                .matches("pub mod generated;")
                .count(),
            1
        );
    }
}
