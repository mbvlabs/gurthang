use std::{fs, path::Path};

use gurthang_new::{self, manifest};

#[test]
fn generated_project_has_the_embedded_manifest_and_no_placeholders() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("generated");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    gurthang_new::execute(
        "sample-app",
        Some(destination.clone()),
        false,
        &source,
        &mut Vec::new(),
    )
    .unwrap();

    let mut actual = Vec::new();
    collect_files(&destination, &destination, &mut actual);
    actual.sort();
    assert_eq!(actual, manifest());

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
    let source = source.canonicalize().unwrap();
    assert!(!cargo.contains(&format!("path = \"{}/crates", source.display())));
    assert!(cargo.contains("name = \"sample-app\""));
    assert!(cargo.contains("name = \"sample_app\""));
    assert!(cargo.contains("gurthang-inertia"));
    assert!(cargo.contains("gurthang-jobs"));
    assert!(cargo.contains("gurthang-http"));
    assert!(!cargo.contains("tower-sessions-sqlx-store"));
    assert!(cargo.contains("[workspace]"));
    assert!(cargo.contains("default-run = \"sample-app\""));
    assert!(cargo.contains("sample_app_models"));
    assert!(!cargo.contains("tera"));
    assert!(!cargo.contains("datastar"));
    assert!(!cargo.contains("src/domain"));

    let models_cargo = fs::read_to_string(destination.join("models/Cargo.toml")).unwrap();
    assert!(models_cargo.contains("name = \"sample_app_models\""));
    assert!(models_cargo.contains("macros"));

    let package = fs::read_to_string(destination.join("package.json")).unwrap();
    assert!(package.contains("\"name\": \"sample-app\""));
    assert!(package.contains("\"dev\": \"vite\""));
    assert!(!package.contains("css:build"));
    assert!(!package.contains("tailwindcli"));

    let readme = fs::read_to_string(destination.join("README.md")).unwrap();
    for command in [
        "npm install",
        "gurthang db create",
        "gurthang db migrate up",
        "gurthang run",
    ] {
        assert!(readme.contains(command), "README is missing {command}");
    }
    assert!(!readme.contains("./bin/install-tailwindcli"));
    assert!(!readme.contains("sqlx migrate run"));

    for required in [
        "gurthang.toml",
        ".env.example",
        ".cargo/config.toml",
        "models/.sqlx/.gitkeep",
        "models/src/user.rs",
        "models/src/sessions.rs",
        "models/src/factories/user.rs",
        "migrations/0001_create_users.sql",
        "migrations/0002_create_sessions.sql",
        "migrations/0003_create_background_jobs.sql",
        "src/models.rs",
        "src/bin/seed.rs",
        "src/bin/export_payloads.rs",
        "src/controllers/auth.rs",
        "src/controllers/dashboard.rs",
        "src/controllers/welcome.rs",
        "src/jobs/mod.rs",
        "src/services/auth.rs",
        "src/routes/mod.rs",
        "src/routes/generated.rs",
        "src/routes/welcome.rs",
        "src/routes/auth.rs",
        "src/routes/dashboard.rs",
        "src/controllers/shared.rs",
        "src/web/assets.rs",
        "css/base.css",
        "assets/.gitkeep",
        "resources/js/Pages/Welcome.tsx",
        "resources/js/Pages/Auth/Login.tsx",
        "resources/js/ssr.tsx",
        "resources/js/routes.ts",
        "vite.ssr.config.ts",
        "build.rs",
    ] {
        assert!(destination.join(required).is_file(), "missing {required}");
    }

    for forbidden in [
        "tests/auth.rs",
        "src/models/user.rs",
        "templates/pages/home.html",
        "src/web/datastar.rs",
        "src/web/tera.rs",
        "src/controllers/pages.rs",
        "src/views/mod.rs",
        "src/views/inertia/welcome.rs",
        "src/routes.rs",
        "bin/install-tailwindcli",
        "src/domain/mod.rs",
        "crates/gurthang-inertia/Cargo.toml",
    ] {
        assert!(
            !destination.join(forbidden).exists(),
            "should not ship {forbidden}"
        );
    }

    let mold = fs::read_to_string(destination.join(".cargo/config.toml")).unwrap();
    assert!(mold.contains("fuse-ld=mold"));

    let env = fs::read_to_string(destination.join(".env.example")).unwrap();
    assert!(env.contains("SQLX_OFFLINE=true"));

    let user = fs::read_to_string(destination.join("models/src/user.rs")).unwrap();
    assert!(user.contains("sqlx::query_as!"));
    assert!(user.contains("SELECT id, email, password_hash, created_at, updated_at FROM users"));
    assert!(!user.contains("created_at: _"));
    assert!(user.contains("gurthang:custom:start"));
    assert!(!user.contains("impl AuthUser for User"));

    let controllers = fs::read_to_string(destination.join("src/controllers/dashboard.rs")).unwrap();
    assert!(!controllers.contains("sqlx::query"));
    assert!(controllers.contains("pub struct Dashboard"));
    assert!(controllers.contains("render_ssr("));
    assert!(!controllers.contains("AppState"));

    let auth = fs::read_to_string(destination.join("src/services/auth.rs")).unwrap();
    assert!(auth.contains("impl axum_login::AuthUser for AuthUser"));

    let jobs = fs::read_to_string(destination.join("src/jobs/mod.rs")).unwrap();
    assert!(!jobs.contains("sqlx::query!"));
    assert!(jobs.contains("purge_expired"));
    assert!(jobs.contains("impl PerformJob for Job"));

    let sessions = fs::read_to_string(destination.join("models/src/sessions.rs")).unwrap();
    assert!(sessions.contains("sqlx::query!"));

    let welcome = fs::read_to_string(destination.join("src/controllers/welcome.rs")).unwrap();
    assert!(welcome.contains("pub struct Welcome"));
    assert!(welcome.contains(".render("));
    assert!(welcome.contains("\"Welcome\""));
    assert!(!welcome.contains("impl InertiaPage"));
    assert!(!welcome.contains("views::inertia"));

    let lib = fs::read_to_string(destination.join("src/lib.rs")).unwrap();
    assert!(lib.contains("gurthang_http::mount!"));
    assert!(lib.contains("get(app.welcome, Welcome::show)"));
    assert!(lib.contains("pub fn export_payloads("));
    assert!(lib.contains("pub struct App"));

    let routes_mod = fs::read_to_string(destination.join("src/routes/mod.rs")).unwrap();
    assert!(routes_mod.contains("mod generated"));
    assert!(!routes_mod.contains("gurthang:generated"));
    assert!(!routes_mod.contains("fn router"));

    let welcome_route = fs::read_to_string(destination.join("src/routes/welcome.rs")).unwrap();
    assert!(welcome_route.contains("pub const WELCOME"));
    assert!(!welcome_route.contains("fn mount"));
    assert!(!welcome_route.contains("AppState"));
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
