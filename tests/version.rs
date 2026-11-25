//! `-V` and `--version` print the crate version. A debug build also prints
//! the commit that git reports for this tree; a release build does not.

use std::process::Command;

fn git_sha() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let sha = text.trim();
    (sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit())).then(|| sha.to_string())
}

#[test]
fn version_prints_the_crate_version_and_in_debug_the_commit() {
    let sha = git_sha();
    let expected = match (&sha, cfg!(debug_assertions)) {
        (Some(sha), true) => format!("cur-top {} ({sha})\n", env!("CARGO_PKG_VERSION")),
        _ => format!("cur-top {}\n", env!("CARGO_PKG_VERSION")),
    };
    for flag in ["--version", "-V"] {
        let out = Command::new(env!("CARGO_BIN_EXE_cur-top"))
            .arg(flag)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "{flag}");
        assert!(out.stderr.is_empty(), "{flag}");
        assert_eq!(String::from_utf8(out.stdout).unwrap(), expected, "{flag}");
    }
}
