use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_and_version_are_available() {
    Command::cargo_bin("gurthang")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("new")
                .and(predicate::str::contains("run"))
                .and(predicate::str::contains("generate"))
                .and(predicate::str::contains("sync"))
                .and(predicate::str::contains("routes"))
                .and(predicate::str::contains("task"))
                .and(predicate::str::contains("db"))
                .and(predicate::str::contains("build"))
                .and(predicate::str::contains("middleware")),
        );

    Command::cargo_bin("gurthang")
        .unwrap()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("live reload"));

    Command::cargo_bin("gurthang")
        .unwrap()
        .arg("generate")
        .arg("--help")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("migration")
                .and(predicate::str::contains("model"))
                .and(predicate::str::contains("mailer"))
                .and(predicate::str::contains("task")),
        );

    Command::cargo_bin("gurthang")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn invalid_name_fails_without_creating_destination() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("destination");
    Command::cargo_bin("gurthang")
        .unwrap()
        .args(["new", "../escape", "--path"])
        .arg(&destination)
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid project name"));
    assert!(!destination.exists());
}

#[test]
fn missing_source_root_fails_with_guidance() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("destination");
    Command::cargo_bin("gurthang")
        .unwrap()
        .env("GURTHANG_ROOT", temp.path())
        .args(["new", "my-app", "--path"])
        .arg(&destination)
        .assert()
        .failure()
        .stderr(predicate::str::contains("GURTHANG_ROOT"));
    assert!(!destination.exists());
}

#[test]
fn new_without_gurthang_root_still_scaffolds() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("destination");
    Command::cargo_bin("gurthang")
        .unwrap()
        .env_remove("GURTHANG_ROOT")
        .args(["new", "my-app", "--path"])
        .arg(&destination)
        .assert()
        .success();
    let cargo = std::fs::read_to_string(destination.join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("gurthang = { path =") || cargo.contains("gurthang = { git ="),
        "generated Cargo.toml should depend on gurthang:\n{cargo}"
    );
}

#[test]
fn tools_help_lists_check_and_sync() {
    Command::cargo_bin("gurthang")
        .unwrap()
        .args(["tools", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("check").and(predicate::str::contains("sync")));
}

#[test]
fn tools_check_reports_project_requirements() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("app");
    Command::cargo_bin("gurthang")
        .unwrap()
        .args(["new", "tools-app", "--path"])
        .arg(&destination)
        .assert()
        .success();
    Command::cargo_bin("gurthang")
        .unwrap()
        .current_dir(&destination)
        .args(["tools", "check"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sqlx-cli").and(predicate::str::contains("mold")));
}
