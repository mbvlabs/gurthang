use std::{fs, io::Write, process::Command};

use gurthang_project::find_root;

use crate::Error;

pub fn sync(check: bool, out: &mut impl Write) -> Result<(), Error> {
    let root = find_root()?;
    let generated = root.join("resources/js/generated");
    let snapshot = if check && generated.is_dir() {
        Some(snapshot_dir(&generated)?)
    } else {
        None
    };
    let status = Command::new("cargo")
        .args(["run", "--bin", "export_payloads"])
        .current_dir(&root)
        .status()?;
    if !status.success() {
        return Err(Error::Message(format!(
            "export_payloads failed ({status})"
        )));
    }
    if let Some(before) = snapshot {
        let after = snapshot_dir(&generated)?;
        if before != after {
            restore_dir(&generated, &before)?;
            return Err(Error::Message(
                "resources/js/generated is out of date".into(),
            ));
        }
        writeln!(out, "resources/js/generated is current")?;
        return Ok(());
    }
    writeln!(out, "Wrote resources/js/generated")?;
    Ok(())
}

fn snapshot_dir(directory: &std::path::Path) -> Result<Vec<(String, String)>, Error> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.path().is_file() {
            files.push((
                entry.file_name().to_string_lossy().into_owned(),
                fs::read_to_string(entry.path())?,
            ));
        }
    }
    files.sort();
    Ok(files)
}

fn restore_dir(directory: &std::path::Path, files: &[(String, String)]) -> Result<(), Error> {
    if directory.is_dir() {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path.is_file() {
                fs::remove_file(path)?;
            }
        }
    } else {
        fs::create_dir_all(directory)?;
    }
    for (name, contents) in files {
        fs::write(directory.join(name), contents)?;
    }
    Ok(())
}
