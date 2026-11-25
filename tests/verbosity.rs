//! `--quiet` and `--verbose`. A success prints nothing on stderr. `--verbose`
//! adds one `context:` line there. The report on stdout and the exit codes
//! stay what they were.

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

#[test]
fn help_lists_quiet_and_verbose() {
    let (code, stdout, stderr) = run(&["--help"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("--quiet"), "{stdout}");
    assert!(stdout.contains("--verbose"), "{stdout}");
}

#[test]
fn verbose_adds_one_context_line_on_stderr_and_keeps_stdout() {
    let (code, stdout, stderr) = run(&["examples/cur.csv"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr:?}");

    let (vcode, vout, verr) = run(&["--verbose", "examples/cur.csv"]);
    assert_eq!(vcode, 0, "{verr}");
    assert_eq!(vout, stdout, "verbose does not change the report");
    assert_eq!(
        verr, "context: rows 3, service 2, usage_type 2, tag 1\n",
        "{verr:?}"
    );

    // The option may also come after the file, and with `--json`.
    let (jcode, jout, jerr) = run(&["--json", "examples/cur.csv"]);
    assert_eq!(jcode, 0, "{jerr}");
    let (vjcode, vjout, vjerr) = run(&["examples/cur.csv", "--json", "--verbose"]);
    assert_eq!(vjcode, 0, "{vjerr}");
    assert_eq!(vjout, jout);
    assert_eq!(vjerr.lines().count(), 1, "{vjerr:?}");
    assert!(vjerr.starts_with("context: "), "{vjerr:?}");
}

#[test]
fn quiet_prints_nothing_extra_and_keeps_stdout() {
    let (_, stdout, _) = run(&["examples/cur.csv"]);
    let (qcode, qout, qerr) = run(&["--quiet", "examples/cur.csv"]);
    assert_eq!(qcode, 0, "{qerr}");
    assert_eq!(qout, stdout, "quiet does not change the report");
    assert!(qerr.is_empty(), "{qerr:?}");
}

#[test]
fn an_error_line_is_the_same_with_quiet_and_verbose_and_has_no_context() {
    let missing = "examples/does-not-exist.csv";
    let (code, stdout, stderr) = run(&[missing]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert_eq!(stderr.lines().count(), 1, "{stderr:?}");
    for flag in ["--quiet", "--verbose"] {
        let (c, o, e) = run(&[flag, missing]);
        assert_eq!(c, 2, "{e}");
        assert!(o.is_empty(), "{o}");
        assert_eq!(e, stderr, "{flag}");
    }
}

#[test]
fn quiet_with_verbose_is_a_usage_error_exit_2() {
    for args in [
        &["--quiet", "--verbose", "examples/cur.csv"][..],
        &["--verbose", "--quiet", "examples/cur.csv"][..],
        &["examples/cur.csv", "--quiet", "--verbose"][..],
    ] {
        let (code, stdout, stderr) = run(args);
        assert_eq!(code, 2, "{stderr}");
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.contains("error:"), "{stderr}");
        assert!(stderr.contains("cannot be used with"), "{stderr}");
        assert!(!stderr.contains("context:"), "{stderr}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn a_full_stdout_is_exit_2_without_a_context_line() {
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_cur-top"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["--verbose", "examples/cur.csv"])
        .stdin(Stdio::null())
        .stdout(full)
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(!stderr.contains("context:"), "{stderr}");
    assert!(stderr.contains("could not write output"), "{stderr}");
}
