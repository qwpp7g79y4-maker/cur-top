//! `--output FILE`: the report goes to a file, written whole or not at all.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn run(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_cur-top"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("NO_COLOR", "1")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// An empty directory of its own under the cargo test directory.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("output")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

#[test]
fn output_holds_exactly_what_stdout_would_and_stdout_stays_empty() {
    let dir = scratch("same");
    for flag in [None, Some("--json"), Some("--md")] {
        let mut args: Vec<&str> = flag.into_iter().collect();
        args.push("examples/cur.csv");
        let (code, expected, err) = run(&args);
        assert_eq!(code, 0, "{err}");
        let file = dir.join("report.out");
        let file_text = file.to_str().unwrap();
        args.extend(["--output", file_text]);
        let (code, stdout, err) = run(&args);
        assert_eq!(code, 0, "{err}");
        assert!(stdout.is_empty(), "{stdout}");
        assert!(err.is_empty(), "{err}");
        assert_eq!(fs::read_to_string(&file).unwrap(), expected, "{flag:?}");
        fs::remove_file(&file).unwrap();
    }
    assert!(names(&dir).is_empty(), "{:?}", names(&dir));
}

#[test]
fn an_existing_output_file_is_replaced_and_no_temporary_file_is_left() {
    let dir = scratch("replace");
    let file = dir.join("report.txt");
    fs::write(&file, "old\n").unwrap();
    let (code, _, err) = run(&["--output", file.to_str().unwrap(), "examples/cur.csv"]);
    assert_eq!(code, 0, "{err}");
    let (_, expected, _) = run(&["examples/cur.csv"]);
    assert_eq!(fs::read_to_string(&file).unwrap(), expected);
    assert_eq!(names(&dir), ["report.txt"]);
}

#[test]
fn a_missing_parent_directory_is_exit_2_and_is_not_created() {
    let dir = scratch("missing");
    let file = dir.join("no-such-dir").join("report.txt");
    let (code, stdout, err) = run(&["--output", file.to_str().unwrap(), "examples/cur.csv"]);
    assert_eq!(code, 2, "{err}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(
        err.starts_with("cur-top: could not write report to "),
        "{err}"
    );
    assert_eq!(err.lines().count(), 1, "{err}");
    assert!(!err.contains("panicked"), "{err}");
    assert!(names(&dir).is_empty());
}

#[test]
fn a_directory_as_output_is_exit_2_and_left_alone() {
    let dir = scratch("isdir");
    let target = dir.join("target");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("keep"), "x").unwrap();
    let (code, _, err) = run(&["--output", target.to_str().unwrap(), "examples/cur.csv"]);
    assert_eq!(code, 2, "{err}");
    assert!(
        err.starts_with("cur-top: could not write report to "),
        "{err}"
    );
    assert_eq!(names(&dir), ["target"]);
    assert_eq!(names(&target), ["keep"]);
}

#[cfg(unix)]
#[test]
fn a_symlink_as_output_is_replaced_not_followed() {
    let dir = scratch("symlink");
    let victim = dir.join("victim.txt");
    fs::write(&victim, "precious\n").unwrap();
    let link = dir.join("report.txt");
    std::os::unix::fs::symlink(&victim, &link).unwrap();
    let (code, _, err) = run(&["--output", link.to_str().unwrap(), "examples/cur.csv"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(fs::read_to_string(&victim).unwrap(), "precious\n");
    assert!(!fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(fs::read_to_string(&link).unwrap().starts_with("cur-top\n"));
    assert_eq!(names(&dir), ["report.txt", "victim.txt"]);
}

#[test]
fn an_input_file_as_output_is_refused_and_not_overwritten() {
    let dir = scratch("same-as-input");
    let copy = dir.join("cur.csv");
    fs::copy("examples/cur.csv", &copy).unwrap();
    let before = fs::read(&copy).unwrap();
    let text = copy.to_str().unwrap();
    let (code, stdout, err) = run(&["--output", text, text]);
    assert_eq!(code, 2, "{err}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(err.contains("is also an input file"), "{err}");
    assert_eq!(fs::read(&copy).unwrap(), before);
    assert_eq!(names(&dir), ["cur.csv"]);

    // A second name for the same inode is the same file. Only Unix sees it.
    #[cfg(unix)]
    {
        let link = dir.join("link.csv");
        fs::hard_link(&copy, &link).unwrap();
        let (code, stdout, err) = run(&["--output", link.to_str().unwrap(), text]);
        assert_eq!(code, 2, "{err}");
        assert!(stdout.is_empty(), "{stdout}");
        assert!(err.contains("is also an input file"), "{err}");
        assert_eq!(fs::read(&copy).unwrap(), before);
        assert_eq!(fs::read(&link).unwrap(), before);
        assert_eq!(names(&dir), ["cur.csv", "link.csv"]);
    }
}

#[test]
fn an_input_error_keeps_an_existing_output_file() {
    let dir = scratch("input-error");
    let file = dir.join("report.txt");
    fs::write(&file, "keep\n").unwrap();
    let (code, stdout, err) = run(&["--output", file.to_str().unwrap(), "examples/missing.csv"]);
    assert_eq!(code, 2, "{err}");
    assert!(stdout.is_empty(), "{stdout}");
    let (_, _, plain) = run(&["examples/missing.csv"]);
    assert_eq!(err, plain, "the same error line as without --output");
    assert_eq!(fs::read_to_string(&file).unwrap(), "keep\n");
    assert_eq!(names(&dir), ["report.txt"]);
}

#[test]
fn the_output_path_in_an_error_has_its_control_characters_escaped() {
    let dir = scratch("escape");
    let file = dir.join("no\u{1b}[31m-dir\nx").join("report.txt");
    let (code, _, err) = run(&["--output", file.to_str().unwrap(), "examples/cur.csv"]);
    assert_eq!(code, 2, "{err}");
    assert!(!err.contains('\u{1b}'), "{err:?}");
    assert_eq!(err.lines().count(), 1, "{err:?}");
    assert!(err.contains("\\u{1b}"), "{err:?}");
}

#[test]
fn verbose_still_prints_its_context_line_with_output() {
    let dir = scratch("verbose");
    let file = dir.join("report.txt");
    let (code, stdout, err) = run(&[
        "--verbose",
        "--output",
        file.to_str().unwrap(),
        "examples/cur.csv",
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(stdout.is_empty());
    assert_eq!(err, "context: rows 3, service 2, usage_type 2, tag 1\n");
}

#[cfg(unix)]
#[test]
fn a_fifo_as_output_is_refused_and_stays_a_fifo() {
    use std::os::unix::fs::FileTypeExt;
    let dir = scratch("fifo");
    let fifo = dir.join("pipe");
    let made = Command::new("mkfifo").arg(&fifo).status().unwrap();
    assert!(made.success());
    let (code, stdout, err) = run(&["--output", fifo.to_str().unwrap(), "examples/cur.csv"]);
    assert_eq!(code, 2, "{err}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(err.contains("not a regular file"), "{err}");
    assert!(fs::symlink_metadata(&fifo).unwrap().file_type().is_fifo());
    assert_eq!(names(&dir), ["pipe"]);
}

#[cfg(unix)]
#[test]
fn a_unix_socket_as_output_is_refused_and_stays_a_socket() {
    use std::os::unix::fs::FileTypeExt;
    // a socket path must stay under SUN_LEN, so bind in a short temp path, not in the checkout
    let dir = std::env::temp_dir().join(format!("ct{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let sock = dir.join("s");
    let _listener = std::os::unix::net::UnixListener::bind(&sock).unwrap();
    let (code, stdout, err) = run(&["--output", sock.to_str().unwrap(), "examples/cur.csv"]);
    assert_eq!(code, 2, "{err}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(err.contains("not a regular file"), "{err}");
    assert!(fs::symlink_metadata(&sock).unwrap().file_type().is_socket());
    assert_eq!(names(&dir), ["s"]);
    drop(_listener);
    let _ = fs::remove_dir_all(&dir);
}
