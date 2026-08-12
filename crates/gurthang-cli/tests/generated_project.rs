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
    for command in [
        "./bin/install-tailwindcli",
        "npm install",
        "npm run css:build",
        "sqlx migrate run",
        "gurthang run",
    ] {
        assert!(readme.contains(command), "README is missing {command}");
    }
    for required in [
        "migrations/0001_create_users.sql",
        "migrations/0002_create_sessions.sql",
        "migrations/0003_create_background_jobs.sql",
        "src/app.rs",
        "src/controllers/auth.rs",
        "src/controllers/dashboard.rs",
        "src/models/user.rs",
        "src/jobs/mod.rs",
        "src/jobs/queue.rs",
        "src/jobs/worker.rs",
        "src/services/auth.rs",
        "src/web/csrf.rs",
        "src/web/datastar.rs",
        "src/web/development.rs",
        "src/web/inertia/response.rs",
        "src/web/inertia/ssr.rs",
        "bin/install-tailwindcli",
        "css/base.css",
        "assets/css/.gitkeep",
        "resources/js/Pages/Auth/Login.tsx",
        "resources/js/Pages/Auth/Register.tsx",
        "resources/js/ssr.tsx",
        "vite.ssr.config.ts",
        "build.rs",
        "templates/fragments/counter.html",
        "tests/auth.rs",
        "tests/jobs.rs",
        "tests/web.rs",
        "templates/pages/home.html",
    ] {
        assert!(destination.join(required).is_file(), "missing {required}");
    }

    let inertia_entry = fs::read_to_string(destination.join("resources/js/app.tsx")).unwrap();
    assert!(inertia_entry.contains("../../css/base.css"));
    let tera_layout = fs::read_to_string(destination.join("templates/layouts/base.html")).unwrap();
    assert!(tera_layout.contains("/assets/css/style.css"));
    let routes = fs::read_to_string(destination.join("src/routes.rs")).unwrap();
    assert!(routes.contains("ServeDir::new(\"assets\")"));
    let package = fs::read_to_string(destination.join("package.json")).unwrap();
    assert!(package.contains("./bin/tailwindcli -i ./css/base.css"));
    assert!(package.contains("\"dev\": \"vite\""));
    assert!(package.contains("\"build:ssr\""));
    assert!(package.contains("\"release\""));

    let ssr = fs::read_to_string(destination.join("src/web/inertia/ssr.rs")).unwrap();
    assert!(ssr.contains("kill_on_drop(true)"));
    let config = fs::read_to_string(destination.join("src/config.rs")).unwrap();
    assert!(config.contains("INERTIA_SSR_RUNTIME"));
    assert!(!config.contains("INERTIA_SSR_ENABLED"));
    let page_contract = fs::read_to_string(destination.join("src/views/inertia/mod.rs")).unwrap();
    assert!(page_contract.contains("const RENDER_MODE: InertiaRenderMode"));
    assert!(page_contract.contains("InertiaRenderMode::Client"));
    let dashboard = fs::read_to_string(destination.join("src/views/inertia/dashboard.rs")).unwrap();
    assert!(dashboard.contains("InertiaRenderMode::Ssr"));
    let auth_pages = fs::read_to_string(destination.join("src/views/inertia/auth.rs")).unwrap();
    assert!(auth_pages.contains("InertiaRenderMode::Client"));
    let app = fs::read_to_string(destination.join("resources/js/app.tsx")).unwrap();
    assert!(app.contains("hydrateRoot"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = fs::metadata(destination.join("bin/install-tailwindcli"))
            .unwrap()
            .permissions()
            .mode();
        assert_ne!(mode & 0o111, 0, "Tailwind installer is not executable");
    }

    let dependencies = fs::read_to_string(destination.join("Cargo.toml")).unwrap();
    assert!(dependencies.contains(r#"features = ["v4", "v7", "serde"]"#));
    for dependency in [
        "argon2",
        "axum-login",
        "datastar",
        "include_dir",
        "mime_guess",
        "notify",
        "tower-livereload",
        "tower-sessions-sqlx-store",
    ] {
        assert!(dependencies.contains(dependency), "missing {dependency}");
    }

    let session_migration =
        fs::read_to_string(destination.join("migrations/0002_create_sessions.sql")).unwrap();
    for column in ["id TEXT", "data BYTEA", "expiry_date TIMESTAMPTZ"] {
        assert!(session_migration.contains(column), "missing {column}");
    }

    let jobs = fs::read_to_string(destination.join("src/jobs/worker.rs")).unwrap();
    assert!(jobs.contains("FOR UPDATE SKIP LOCKED"));
    assert!(jobs.contains("PgListener"));
    let job_migration =
        fs::read_to_string(destination.join("migrations/0003_create_background_jobs.sql")).unwrap();
    assert!(job_migration.contains("payload JSONB"));
    assert!(job_migration.contains("id UUID PRIMARY KEY"));
    assert!(job_migration.contains("locked_by UUID"));
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
