use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use gurthang_project::find_root;

#[derive(Debug)]
pub enum Error {
    Message(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<gurthang_project::Error> for Error {
    fn from(error: gurthang_project::Error) -> Self {
        Self::Message(error.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Message(error.to_string())
    }
}

struct Tool {
    name: &'static str,
    hint: &'static str,
}

const TOOLS: &[Tool] = &[
    Tool {
        name: "cargo",
        hint: "install Rust from https://rustup.rs",
    },
    Tool {
        name: "clang",
        hint: "install your distribution's clang package",
    },
    Tool {
        name: "mold",
        hint: "install your distribution's mold package",
    },
    Tool {
        name: "psql",
        hint: "install PostgreSQL client tools",
    },
    Tool {
        name: "node",
        hint: "install Node.js 22 or newer",
    },
    Tool {
        name: "npm",
        hint: "install npm (bundled with Node.js)",
    },
];

pub fn check(out: &mut impl Write) -> Result<()> {
    let root = find_root()?;
    report(&root, out)
}

pub fn sync(out: &mut impl Write) -> Result<()> {
    let root = find_root()?;
    if version_of("cargo", &["--version"]).is_none() {
        return Err(Error::Message(
            "cargo is required to install tools; install Rust from https://rustup.rs".into(),
        ));
    }

    let desired = project_sqlx_version(&root);
    let installed = installed_sqlx_cli();
    let up_to_date = match (&installed, &desired) {
        (Some(installed), Some(desired)) => sqlx_cli_matches(installed, desired),
        (Some(_), None) => true,
        (None, _) => false,
    };
    if !up_to_date {
        let version = desired.as_deref().unwrap_or("latest");
        writeln!(
            out,
            "+ installing sqlx-cli {version} (compiled from source; this can take a few minutes)"
        )?;
        let mut command = Command::new("cargo");
        command.args(["install", "sqlx-cli", "--force"]);
        if let Some(desired) = &desired {
            command.args(["--version", desired]);
        }
        let status = command.status()?;
        if !status.success() {
            return Err(Error::Message(format!(
                "cargo install sqlx-cli failed ({status})"
            )));
        }
    }
    report(&root, out)
}

fn report(root: &Path, out: &mut impl Write) -> Result<()> {
    for tool in TOOLS {
        match version_of(tool.name, &["--version"]) {
            Some(version) => {
                let summary = if version.starts_with(tool.name) {
                    version
                } else {
                    format!("{} {}", tool.name, version)
                };
                writeln!(out, "+ {summary}")?;
            }
            None => writeln!(out, "! {} not found; {}", tool.name, tool.hint)?,
        }
    }
    match (installed_sqlx_cli(), project_sqlx_version(root)) {
        (None, _) => writeln!(out, "! sqlx-cli not found; run `gurthang tools sync`")?,
        (Some(installed), Some(desired)) if !sqlx_cli_matches(&installed, &desired) => writeln!(
            out,
            "! sqlx-cli {installed} does not match project sqlx {desired}; run `gurthang tools sync`"
        )?,
        (Some(installed), _) => writeln!(out, "+ sqlx-cli {installed}")?,
    }
    Ok(())
}

fn version_of(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    Some(line.to_owned())
}

fn installed_sqlx_cli() -> Option<String> {
    let line = version_of("cargo", &["sqlx", "--version"])?;
    line.split_whitespace()
        .find(|token| {
            token
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
                && token.contains('.')
        })
        .map(str::to_owned)
}

fn project_sqlx_version(root: &Path) -> Option<String> {
    if let Ok(lock) = fs::read_to_string(root.join("Cargo.lock"))
        && let Some(version) = sqlx_version_from_lock(&lock)
    {
        return Some(version);
    }
    for relative in ["Cargo.toml", "models/Cargo.toml"] {
        if let Ok(manifest) = fs::read_to_string(root.join(relative))
            && let Some(version) = sqlx_version_from_manifest(&manifest)
        {
            return Some(version);
        }
    }
    None
}

fn sqlx_version_from_lock(lock: &str) -> Option<String> {
    let lines: Vec<&str> = lock.lines().collect();
    lines
        .iter()
        .position(|line| line.trim() == "name = \"sqlx\"")
        .and_then(|index| lines.get(index + 1))
        .and_then(|line| line.trim().strip_prefix("version = \""))
        .and_then(|version| version.strip_suffix('"'))
        .map(str::to_owned)
}

fn sqlx_version_from_manifest(manifest: &str) -> Option<String> {
    for line in manifest.lines() {
        let Some(rest) = line.trim().strip_prefix("sqlx") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim();
        if let Some(version) = rest
            .strip_prefix('"')
            .and_then(|version| version.strip_suffix('"'))
            && !version.is_empty()
        {
            return Some(version.to_owned());
        }
        if let Some(table) = rest.strip_prefix('{') {
            for pair in table.split(',') {
                let Some(version) = pair.trim().strip_prefix("version") else {
                    continue;
                };
                let Some(version) = version.trim_start().strip_prefix('=') else {
                    continue;
                };
                let version = version.trim().trim_matches('"');
                if !version.is_empty() {
                    return Some(version.to_owned());
                }
            }
        }
    }
    None
}

fn sqlx_cli_matches(installed: &str, desired: &str) -> bool {
    installed == desired || installed.starts_with(&format!("{desired}."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_sqlx_version_from_lock() {
        let lock = "[[package]]\nname = \"sqlx\"\nversion = \"0.8.6\"\nsource = \"registry\"\n";
        assert_eq!(sqlx_version_from_lock(lock).as_deref(), Some("0.8.6"));
        assert_eq!(
            sqlx_version_from_lock("name = \"sqlx-core\"\nversion = \"0.8.6\"\n").as_deref(),
            None
        );
    }

    #[test]
    fn reads_sqlx_version_from_manifest() {
        assert_eq!(
            sqlx_version_from_manifest("sqlx = \"0.8\"").as_deref(),
            Some("0.8")
        );
        assert_eq!(
            sqlx_version_from_manifest(
                "sqlx = { version = \"0.8\", default-features = false, features = ["
            )
            .as_deref(),
            Some("0.8")
        );
        assert_eq!(
            sqlx_version_from_manifest("sqlx-core = \"0.8\"").as_deref(),
            None
        );
        assert_eq!(
            sqlx_version_from_manifest("[dependencies]\naxum = \"0.8\"").as_deref(),
            None
        );
        assert_eq!(
            sqlx_version_from_manifest("sqlx = { workspace = true }").as_deref(),
            None
        );
    }

    #[test]
    fn matches_installed_cli_against_project_requirement() {
        assert!(sqlx_cli_matches("0.8.6", "0.8.6"));
        assert!(sqlx_cli_matches("0.8.6", "0.8"));
        assert!(!sqlx_cli_matches("0.9.0", "0.8"));
        assert!(!sqlx_cli_matches("0.8.6", "0.8.6-rc.1"));
        assert!(!sqlx_cli_matches("0.8.6", "0.80"));
    }
}
