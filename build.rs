//! Asks git for the commit of this tree. A debug build prints it after the
//! version in `--version`. Nothing here invents a commit: a missing git, a
//! failed command, or a value that is not a full sha leaves the version as
//! it is. A release build does not receive the commit.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(cur_top_git_sha)");
    println!("cargo:rerun-if-changed=build.rs");
    track_git_head();
    if std::env::var("PROFILE").unwrap_or_default() != "debug" {
        return;
    }
    if let Some(sha) = git_sha() {
        println!("cargo:rustc-cfg=cur_top_git_sha");
        println!("cargo:rustc-env=CUR_TOP_GIT_SHA={sha}");
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()))
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(manifest_dir())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|text| text.trim().to_string())
}

/// A path inside the git directory, if it is a file. `--git-path` also
/// works in a worktree, where the branch ref lives in the common directory.
fn git_path(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(git(&["rev-parse", "--git-path", name])?);
    let path = if path.is_absolute() {
        path
    } else {
        manifest_dir().join(path)
    };
    path.is_file().then_some(path)
}

/// Rebuild when the commit changes.
fn track_git_head() {
    let Some(head) = git_path("HEAD") else {
        return;
    };
    println!("cargo:rerun-if-changed={}", head.display());
    let Ok(text) = std::fs::read_to_string(&head) else {
        return;
    };
    let Some(name) = text.trim().strip_prefix("ref:").map(str::trim) else {
        return;
    };
    if let Some(path) = git_path(name).or_else(|| git_path("packed-refs")) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn git_sha() -> Option<String> {
    let sha = git(&["rev-parse", "--verify", "HEAD"])?;
    (sha.len() == 40 && sha.bytes().all(|byte| byte.is_ascii_hexdigit())).then_some(sha)
}
