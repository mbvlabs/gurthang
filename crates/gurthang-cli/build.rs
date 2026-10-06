use std::{fs, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=GURTHANG_GIT_URL");
    println!("cargo:rerun-if-env-changed=GURTHANG_GIT_REV");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    rerun_if_git_head_ref_changes();

    let url = std::env::var("GURTHANG_GIT_URL")
        .or_else(|_| std::env::var("CARGO_PKG_REPOSITORY"))
        .unwrap_or_else(|_| "https://github.com/mbvlabs/gurthang".into());
    println!("cargo:rustc-env=GURTHANG_GIT_URL={url}");

    let rev = std::env::var("GURTHANG_GIT_REV")
        .ok()
        .filter(|rev| !rev.is_empty())
        .or_else(|| {
            std::env::var("GITHUB_SHA")
                .ok()
                .filter(|sha| !sha.is_empty())
        })
        .or_else(git_rev)
        .unwrap_or_default();
    println!("cargo:rustc-env=GURTHANG_GIT_REV={rev}");
}

fn rerun_if_git_head_ref_changes() {
    let Ok(head) = fs::read_to_string("../../.git/HEAD") else {
        return;
    };
    let Some(reference) = head.strip_prefix("ref: ") else {
        return;
    };
    println!("cargo:rerun-if-changed=../../.git/{}", reference.trim());
}

fn git_rev() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let rev = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if rev.is_empty() { None } else { Some(rev) }
}
