use std::{fs, path::Path};

use askama::Template;
use gurthang_project::ProjectName;

use crate::error::{Error, Result};

struct Ctx<'a> {
    project_name: &'a str,
    crate_name: &'a str,
    package_name: &'a str,
    source: &'a str,
}

macro_rules! scaffold {
    ($($ident:ident => $path:literal),+ $(,)?) => {
        $(
            #[derive(Template)]
            #[template(path = $path, escape = "none")]
            struct $ident<'a> {
                project_name: &'a str,
                crate_name: &'a str,
                package_name: &'a str,
                source: &'a str,
            }
        )+

        pub fn manifest() -> Vec<String> {
            let mut paths = vec![$(output_name($path)),+];
            paths.sort_unstable();
            paths
        }

        pub fn render(destination: &Path, name: &ProjectName, source: &str) -> Result<()> {
            let ctx = Ctx {
                project_name: name.project_name(),
                crate_name: name.crate_name(),
                package_name: name.package_name(),
                source,
            };
            $(
                write_file(
                    destination,
                    &output_name($path),
                    $ident {
                        project_name: ctx.project_name,
                        crate_name: ctx.crate_name,
                        package_name: ctx.package_name,
                        source: ctx.source,
                    }
                    .render()?
                    .as_bytes(),
                )?;
            )+
            Ok(())
        }
    };
}

scaffold! {
    CargoConfig => ".cargo/config.toml",
    EnvExample => ".env.example",
    Gitignore => ".gitignore.gurthang",
    CargoToml => "Cargo.toml.gurthang",
    Readme => "README.md.gurthang",
    AssetsGitkeep => "assets/.gitkeep",
    AssetsKeep => "assets/keep.txt",
    AssetsCssGitkeep => "assets/css/.gitkeep",
    AskamaToml => "askama.toml",
    BuildRs => "build.rs",
    BaseCss => "css/base.css",
    GurthangToml => "gurthang.toml",
    MigrationUsers => "migrations/0001_create_users.sql",
    MigrationSessions => "migrations/0002_create_sessions.sql",
    MigrationJobs => "migrations/0003_create_background_jobs.sql",
    ModelsSqlxGitkeep => "models/.sqlx/.gitkeep",
    Sqlx0533 => "models/.sqlx/query-0533c1513f3d1e8a19da3ae5605d90bbdf2afdfbcb4e7d585ed33423079515f7.json",
    Sqlx2709 => "models/.sqlx/query-2709830df67c99ff91ace028ec2560f0104f0b1abe0279acc29d625740ba986e.json",
    Sqlx406f => "models/.sqlx/query-406f1d8cf0b11bc075d0a25833d226b36db77da0a259c87555ed1aca2af8bbf2.json",
    Sqlxb6b8 => "models/.sqlx/query-b6b8bd028b70bec73f8aaf60f2cbf7d15458ed1832d233bfe3ee9a97880bf98a.json",
    Sqlxd9a4 => "models/.sqlx/query-d9a4838ba7ae06f214ea564e1ffefc4c6e44a9e83201711d81a5909212014e47.json",
    ModelsCargoToml => "models/Cargo.toml.gurthang",
    FactoriesMod => "models/src/factories/mod.rs",
    FactoriesUser => "models/src/factories/user.rs",
    ModelsLib => "models/src/lib.rs",
    ModelsSessions => "models/src/sessions.rs",
    ModelsUser => "models/src/user.rs",
    PackageJson => "package.json",
    PageLogin => "resources/js/Pages/Auth/Login.tsx",
    PageRegister => "resources/js/Pages/Auth/Register.tsx",
    PageDashboard => "resources/js/Pages/Dashboard.tsx",
    PageWelcome => "resources/js/Pages/Welcome.tsx",
    JsApp => "resources/js/app.tsx",
    JsEnv => "resources/js/env.d.ts",
    GenAuthProps => "resources/js/generated/AuthProps.ts",
    GenDashboardProps => "resources/js/generated/DashboardProps.ts",
    GenFlashProps => "resources/js/generated/FlashProps.ts",
    GenLoginProps => "resources/js/generated/LoginProps.ts",
    GenRegisterProps => "resources/js/generated/RegisterProps.ts",
    GenSafeUser => "resources/js/generated/SafeUser.ts",
    GenSharedProps => "resources/js/generated/SharedProps.ts",
    GenWelcomeProps => "resources/js/generated/WelcomeProps.ts",
    JsRoutes => "resources/js/routes.ts",
    JsSsr => "resources/js/ssr.tsx",
    BinExportPayloads => "src/bin/export_payloads.rs",
    BinSeed => "src/bin/seed.rs",
    ConfigDevelopment => "config/development.yaml",
    ConfigProduction => "config/production.yaml",
    ConfigTest => "config/test.yaml",
    ControllerAuth => "src/controllers/auth.rs",
    ControllerDashboard => "src/controllers/dashboard.rs",
    ControllerMod => "src/controllers/mod.rs",
    ControllerShared => "src/controllers/shared.rs",
    ControllerWelcome => "src/controllers/welcome.rs",
    SrcApp => "src/app.rs",
    SrcError => "src/error.rs",
    InitializerAuth => "src/initializers/auth.rs",
    InitializerMod => "src/initializers/mod.rs",
    InitializerViewEngine => "src/initializers/view_engine.rs",
    SrcLib => "src/lib.rs",
    SrcMain => "src/main.rs",
    MailerAuth => "src/mailers/auth.rs",
    MailerWelcomeHtml => "src/mailers/auth/welcome.html",
    MailerWelcomeTxt => "src/mailers/auth/welcome.txt",
    MailerMod => "src/mailers/mod.rs",
    SrcModels => "src/models.rs",
    RouteAuth => "src/routes/auth.rs",
    RouteDashboard => "src/routes/dashboard.rs",
    RouteGenerated => "src/routes/generated.rs",
    RouteMod => "src/routes/mod.rs",
    RouteWelcome => "src/routes/welcome.rs",
    ServiceAuth => "src/services/auth.rs",
    ServiceMod => "src/services/mod.rs",
    SrcAssets => "src/assets.rs",
    TasksMod => "src/tasks/mod.rs",
    WorkerGenerated => "src/workers/generated.rs",
    WorkerMod => "src/workers/mod.rs",
    WorkerPurge => "src/workers/purge_expired_sessions.rs",
    TsConfig => "tsconfig.json",
    ViteConfig => "vite.config.ts",
    ViteSsrConfig => "vite.ssr.config.ts",
}

fn output_name(source_path: &str) -> String {
    source_path
        .strip_suffix(".gurthang")
        .unwrap_or(source_path)
        .to_owned()
}

fn write_file(destination: &Path, relative: &str, contents: &[u8]) -> Result<()> {
    let output = destination.join(relative);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| Error::io(format!("could not create {}", parent.display()), error))?;
    }
    fs::write(&output, contents)
        .map_err(|error| Error::io(format!("could not write {}", output.display()), error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_is_sorted() {
        let manifest = manifest();
        let mut expected = manifest.clone();
        expected.sort_unstable();
        assert_eq!(manifest, expected);
    }
}
