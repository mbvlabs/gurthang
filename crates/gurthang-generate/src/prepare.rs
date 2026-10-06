use std::{fs, io::Write, process::Command};

use gurthang_project::find_root;

use crate::{Error, env};

pub fn run(check: bool, out: &mut impl Write) -> Result<(), Error> {
    let root = find_root()?;
    env::load_env(&root);
    ensure_sqlx_offline(&root, out)?;
    let mut command = Command::new("cargo");
    command.arg("sqlx").arg("prepare");
    if check {
        command.arg("--check");
    }
    command.arg("--workspace");
    command.current_dir(&root);
    command.env_remove("SQLX_OFFLINE");
    writeln!(
        out,
        "+ cargo sqlx prepare{} --workspace",
        if check { " --check" } else { "" }
    )?;
    let status = command.status()?;
    if !status.success() {
        return Err(Error::Message(format!(
            "cargo sqlx prepare failed ({status}); is DATABASE_URL reachable and sqlx-cli installed?"
        )));
    }
    if check {
        writeln!(out, "SQLx offline data is current")?;
    } else {
        writeln!(out, "Updated SQLx offline query data")?;
    }
    Ok(())
}

pub fn ensure_sqlx_offline(root: &std::path::Path, out: &mut impl Write) -> Result<(), Error> {
    let path = root.join(".env");
    if !path.is_file() {
        return Ok(());
    }
    let text = fs::read_to_string(&path)?;
    if text.lines().any(|line| line.starts_with("SQLX_OFFLINE=")) {
        return Ok(());
    }
    let mut updated = text;
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str("SQLX_OFFLINE=true\n");
    fs::write(&path, updated)?;
    writeln!(out, "Added SQLX_OFFLINE=true to .env")?;
    Ok(())
}
