use std::{fs, path::Path};

use gurthang_cli::{cli::NewArgs, new, renderer};

#[test]
fn generated_project_has_the_embedded_manifest_and_no_placeholders() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("generated");
    new::execute(
        NewArgs {
            name: "sample-app".into(),
            path: Some(destination.clone()),
            dry_run: false,
        },
        &mut Vec::new(),
    )
    .unwrap();

    let mut actual = Vec::new();
    collect_files(&destination, &destination, &mut actual);
    actual.sort();
    assert_eq!(actual, renderer::manifest());

    for path in &actual {
        let bytes = fs::read(destination.join(path)).unwrap();
        assert!(
            !bytes
                .windows(b"__GURTHANG_".len())
                .any(|value| value == b"__GURTHANG_"),
            "placeholder remains in {path}"
        );
    }

    let cargo = fs::read_to_string(destination.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("name = \"sample-app\""));
    assert!(cargo.contains("name = \"sample_app\""));
    let package = fs::read_to_string(destination.join("package.json")).unwrap();
    assert!(package.contains("\"name\": \"sample-app\""));
    let readme = fs::read_to_string(destination.join("README.md")).unwrap();
    for command in ["npm install", "sqlx migrate run", "cargo run"] {
        assert!(readme.contains(command), "README is missing {command}");
    }
    for required in [
        "migrations/0001_create_users.sql",
        "migrations/0002_create_sessions.sql",
        "src/app.rs",
        "src/controllers/auth.rs",
        "src/controllers/dashboard.rs",
        "src/models/user.rs",
        "src/services/auth.rs",
        "src/web/csrf.rs",
        "src/web/datastar.rs",
        "src/web/inertia/response.rs",
        "resources/js/Pages/Auth/Login.tsx",
        "resources/js/Pages/Auth/Register.tsx",
        "templates/fragments/counter.html",
        "tests/auth.rs",
        "tests/web.rs",
        "templates/pages/home.html",
    ] {
        assert!(destination.join(required).is_file(), "missing {required}");
    }

    let dependencies = fs::read_to_string(destination.join("Cargo.toml")).unwrap();
    for dependency in [
        "argon2",
        "axum-login",
        "datastar",
        "tower-sessions-sqlx-store",
    ] {
        assert!(dependencies.contains(dependency), "missing {dependency}");
    }

    let session_migration =
        fs::read_to_string(destination.join("migrations/0002_create_sessions.sql")).unwrap();
    for column in ["id TEXT", "data BYTEA", "expiry_date TIMESTAMPTZ"] {
        assert!(session_migration.contains(column), "missing {column}");
    }
}

fn collect_files(root: &Path, directory: &Path, output: &mut Vec<String>) {
    for entry in fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            collect_files(root, &entry.path(), output);
        } else {
            output.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
}
