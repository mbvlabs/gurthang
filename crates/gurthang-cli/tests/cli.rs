use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_and_version_are_available() {
    Command::cargo_bin("gurthang")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("new").and(predicate::str::contains("run")));

    Command::cargo_bin("gurthang")
        .unwrap()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("live reload"));

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
