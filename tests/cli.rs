//! Command-line behavior: exit codes, files, and the report the binary prints.

#[cfg(target_os = "linux")]
use std::ffi::OsStr;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("cur-top-cli-{}-{n}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn command(args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cur-top"));
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    cmd.env("NO_COLOR", "1");
    cmd.args(args);
    cmd
}

fn run(args: &[&str]) -> (i32, String, String) {
    let out = command(args).output().unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn help_and_version_exit_0() {
    let (code, stdout, stderr) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("no network"));
    assert!(stdout.contains("Exit codes: 0 ok, 2 input or usage error."));
    let (code, stdout, stderr) = run(&["--version"]);
    assert_eq!(code, 0);
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("cur-top 1.0.0"));
}

#[test]
fn usage_errors_exit_2() {
    for args in [
        &[][..],
        &["--no-such-flag"],
        &["--top=0", "examples/cur.csv"],
        &["--top=-1", "examples/cur.csv"],
        &["--amount=line_item_unblended_rate", "examples/cur.csv"],
    ] {
        let (code, stdout, stderr) = run(args);
        assert_eq!(code, 2, "{args:?} stdout={stdout} stderr={stderr}");
        assert!(stdout.is_empty(), "{stdout}");
        assert!(
            stderr.contains("cur-top") || stderr.contains("error"),
            "{stderr}"
        );
    }
}

#[test]
fn missing_file_exits_2() {
    let (code, stdout, stderr) = run(&["missing-cur.csv"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.starts_with("cur-top: "), "{stderr}");
    assert!(stderr.contains("missing-cur.csv"), "{stderr}");
}

#[test]
fn example_text_exits_0() {
    let (code, stdout, stderr) = run(&["examples/cur.csv"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("amount=4.50"));
    assert!(stdout.contains("user_team=platform"));
    assert!(stdout.contains("delta=3.50"));
    assert!(!stdout.contains("123456789012"));
    assert!(!stdout.contains("0.0104"));
}

#[test]
fn example_json_exits_0() {
    let (code, stdout, stderr) = run(&["--json", "examples/cur.csv"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.starts_with('{'));
    assert!(stdout.contains("\"amount\": \"4.50\""));
    assert!(stdout.contains("\"previous_month_present\": true"));
    assert!(!stdout.contains("123456789012"));
}

#[test]
fn top_one_drops_the_smaller_service() {
    let (code, stdout, stderr) = run(&["--top", "1", "examples/cur.csv"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("AmazonEC2"));
    assert!(!stdout.contains("AmazonS3"));
    assert!(stdout.contains("amount=4.50"));
}

#[test]
fn account_id_column_is_absent_from_stdout() {
    let scratch = Scratch::new();
    let path = scratch.path().join("with-account.csv");
    fs::write(
        &path,
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags,line_item_usage_account_id\n2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,{},123456789012\n",
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(!stdout.contains("123456789012"), "{stdout}");
    assert!(stdout.contains("amount=1.00"));
}

#[test]
#[cfg(unix)]
fn symlink_is_followed_and_directory_is_rejected() {
    let scratch = Scratch::new();
    let real = scratch.path().join("real.csv");
    fs::write(
        &real,
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.50,{}\n",
    )
    .unwrap();
    let link = scratch.path().join("link.csv");
    symlink(&real, &link).unwrap();
    let (code, stdout, stderr) = run(&[link.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("amount=1.50"));

    let dir = scratch.path().join("folder.csv");
    fs::create_dir(&dir).unwrap();
    let (code, stdout, stderr) = run(&[dir.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("is a directory"), "{stderr}");
}

#[test]
fn parquet_and_bad_amount_hide_the_payload() {
    let scratch = Scratch::new();
    let parquet = scratch.path().join("export.snappy.parquet");
    fs::write(&parquet, b"PAR1secret-body").unwrap();
    let (code, stdout, stderr) = run(&[parquet.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("parquet is not supported"), "{stderr}");
    assert!(!stderr.contains("secret"), "{stderr}");

    let bad = scratch.path().join("bad.csv");
    fs::write(
        &bad,
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,secret,{}\n",
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[bad.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("invalid amount"), "{stderr}");
    assert!(!stderr.contains("secret"), "{stderr}");
}

#[test]
fn duplicate_utf8_and_escaped_tag_key_exits_2() {
    let scratch = Scratch::new();
    let path = scratch.path().join("dup.csv");
    let cell = "{\"é\":\"x\",\"\\u00e9\":\"y\"}";
    let quoted = format!("\"{}\"", cell.replace('"', "\"\""));
    let csv = format!(
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n2026-11-01T00:00:00Z,A,U,1.00,{quoted}\n"
    );
    fs::write(&path, csv).unwrap();
    let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
    assert_eq!(code, 2, "stdout={stdout} stderr={stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("duplicate key"), "{stderr}");
    assert!(!stderr.contains('é'), "{stderr}");
}

#[test]
fn extreme_month_total_is_the_same_every_run() {
    let scratch = Scratch::new();
    let max = "79228162514264337593543950335";
    let fit = scratch.path().join("fit.csv");
    // Keys sort a, m, z. Amounts are MAX, -MAX, MAX, so the sorted sum fits.
    // Another order adds the two maxima first and does not fit.
    fs::write(
        &fit,
        three_rows(&[
            ("a", "ua", max),
            ("m", "um", &format!("-{max}")),
            ("z", "uz", max),
        ]),
    )
    .unwrap();
    let fit_arg = fit.to_str().unwrap();
    let mut first = None;
    for _ in 0..8 {
        let outcome = run(&[fit_arg]);
        match &first {
            None => first = Some(outcome),
            Some(prev) => assert_eq!(prev, &outcome),
        }
    }
    let (code, stdout, stderr) = first.unwrap();
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(
        stdout.contains(&format!(
            "totals\namount={max}.00 previous=0.00 delta={max}.00\n"
        )),
        "{stdout}"
    );
    assert!(!stdout.contains("panic"), "{stdout}");

    let cancel = scratch.path().join("cancel.csv");
    // Keys sort a, m, z. Amounts are MAX, MAX, -MAX. Key order adds the two
    // maxima first, which does not fit, but the real sum is MAX.
    fs::write(
        &cancel,
        three_rows(&[
            ("a", "ua", max),
            ("m", "um", max),
            ("z", "uz", &format!("-{max}")),
        ]),
    )
    .unwrap();
    let cancel_arg = cancel.to_str().unwrap();
    let mut cancel_first = None;
    for _ in 0..8 {
        let outcome = run(&[cancel_arg]);
        match &cancel_first {
            None => cancel_first = Some(outcome),
            Some(prev) => assert_eq!(prev, &outcome),
        }
    }
    let (code, stdout, stderr) = cancel_first.unwrap();
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(
        stdout.contains(&format!(
            "totals\namount={max}.00 previous=0.00 delta={max}.00\n"
        )),
        "{stdout}"
    );
    assert!(!stdout.contains("panic"), "{stdout}");

    let overflow = scratch.path().join("overflow.csv");
    fs::write(
        &overflow,
        three_rows(&[("a", "ua", max), ("m", "um", "1"), ("z", "uz", max)]),
    )
    .unwrap();
    let overflow_arg = overflow.to_str().unwrap();
    for _ in 0..3 {
        let (code, stdout, stderr) = run(&[overflow_arg]);
        assert_eq!(code, 2, "stdout={stdout} stderr={stderr}");
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.contains("amount total does not fit"), "{stderr}");
        assert!(!stderr.contains("panic"), "{stderr}");
    }
}

#[test]
fn one_key_sum_fits_or_does_not_fit_by_its_real_value() {
    let scratch = Scratch::new();
    let max = "79228162514264337593543950335";
    let fit = scratch.path().join("same-fit.csv");
    fs::write(
        &fit,
        three_rows(&[
            ("svc", "use", max),
            ("svc", "use", max),
            ("svc", "use", &format!("-{max}")),
        ]),
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[fit.to_str().unwrap()]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains(&format!("{max}.00")), "{stdout}");

    let over = scratch.path().join("same-over.csv");
    fs::write(
        &over,
        three_rows(&[
            ("svc", "use", max),
            ("svc", "use", max),
            ("svc", "use", "1"),
        ]),
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[over.to_str().unwrap()]);
    assert_eq!(code, 2, "stdout={stdout} stderr={stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("amount total does not fit"), "{stderr}");
    assert!(!stderr.contains("panic"), "{stderr}");
}

fn three_rows(rows: &[(&str, &str, &str)]) -> String {
    let mut csv = String::from(
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n",
    );
    for (product, usage, amount) in rows {
        csv.push_str("2026-11-01T00:00:00Z,");
        csv.push_str(product);
        csv.push(',');
        csv.push_str(usage);
        csv.push(',');
        csv.push_str(amount);
        csv.push_str(",{}\n");
    }
    csv
}

#[test]
fn non_ascii_billing_timestamp_exits_2_without_the_cell() {
    let scratch = Scratch::new();
    for (i, mark) in ["é", "€", "😀"].into_iter().enumerate() {
        let path = scratch.path().join(format!("ts-{i}.csv"));
        let csv = format!(
            "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n2026-11-01T00:00:0{mark}Z,A,U,1.00,{{}}\n"
        );
        fs::write(&path, csv).unwrap();
        let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
        assert_eq!(code, 2, "{mark} stdout={stdout} stderr={stderr}");
        assert!(stdout.is_empty(), "{stdout}");
        assert!(
            stderr.contains("invalid bill_billing_period_start_date"),
            "{stderr}"
        );
        assert!(!stderr.contains(mark), "{stderr}");
        assert!(!stderr.contains("panic"), "{stderr}");
    }
}

#[test]
fn equal_joined_tags_stay_two_lines() {
    let scratch = Scratch::new();
    let file = scratch.path().join("eq.csv");
    fs::write(
        &file,
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n\
2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,\"{\"\"a\"\":\"\"b=c\"\"}\"\n\
2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,2.00,\"{\"\"a=b\"\":\"\"c\"\"}\"\n",
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[file.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("\na=b=c amount=1.00 "), "{stdout}");
    assert!(stdout.contains("\na\\=b=c amount=2.00 "), "{stdout}");
    assert!(!stdout.contains("\na=b=c amount=3.00"), "{stdout}");
}

#[test]
fn keys_that_differ_only_by_a_backslash_stay_two_lines_in_text() {
    let scratch = Scratch::new();
    let file = scratch.path().join("spacekeys.csv");
    fs::write(
        &file,
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n\
2026-11-01T00:00:00Z,A B,U,1.00,{}\n\
2026-11-01T00:00:00Z,A\\u{20}B,U,2.00,{}\n\
2026-11-01T00:00:00Z,amount=999.00,U,4.00,{}\n\
2026-11-01T00:00:00Z,amount\\=999.00,U,8.00,{}\n",
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[file.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("\nA\\u{20}B amount=1.00 "), "{stdout}");
    assert!(stdout.contains("\nA\\\\u{20}B amount=2.00 "), "{stdout}");
    assert!(
        stdout.contains("\namount\\=999.00 amount=4.00 "),
        "{stdout}"
    );
    assert!(
        stdout.contains("\namount\\\\=999.00 amount=8.00 "),
        "{stdout}"
    );
}

#[test]
fn wrong_column_count_row_number_counts_records_not_lines() {
    let scratch = Scratch::new();
    let file = scratch.path().join("ml_short.csv");
    fs::write(
        &file,
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags,note\n\
2026-11-01T00:00:00Z,A,U,1.00,{},\"one\ntwo\nthree\"\n\
2026-11-01T00:00:00Z,A,U\n",
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[file.to_str().unwrap()]);
    assert_eq!(code, 2, "{stdout}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(
        stderr.contains("file 1: row 2: wrong column count"),
        "{stderr}"
    );
}

#[test]
fn text_keys_hide_invisible_characters_and_fake_amounts() {
    let scratch = Scratch::new();
    let file = scratch.path().join("hide.csv");
    fs::write(
        &file,
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n\
2026-11-01T00:00:00Z,Amazon\u{202e}EC2,Box\u{200b}Usage,1.00,\"{\"\"a\"\":\"\"x amount=999.00 y\\u202e\"\"}\"\n",
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[file.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        !stdout.contains('\u{202e}') && !stdout.contains('\u{200b}'),
        "{stdout}"
    );
    assert!(
        stdout.contains("Amazon\\u{202e}EC2 amount=1.00 "),
        "{stdout}"
    );
    assert!(
        stdout.contains("Box\\u{200b}Usage amount=1.00 "),
        "{stdout}"
    );
    assert!(
        stdout.contains("a=x\\u{20}amount\\=999.00\\u{20}y\\u{202e} amount=1.00 "),
        "{stdout}"
    );
}

#[test]
fn product_code_cannot_pose_as_amount_field() {
    let scratch = Scratch::new();
    let file = scratch.path().join("pose.csv");
    fs::write(
        &file,
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n\
2026-11-01T00:00:00Z,amount=999.00,BoxUsage,1.00,\n",
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[file.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("amount\\=999.00 amount=1.00 "), "{stdout}");
    assert!(!stdout.contains("\namount=999.00"), "{stdout}");
}

#[test]
fn product_code_hides_hangul_filler() {
    let scratch = Scratch::new();
    let file = scratch.path().join("filler.csv");
    fs::write(
        &file,
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n\
2026-11-01T00:00:00Z,Amazon\u{3164}EC2,BoxUsage,1.00,\n",
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[file.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(!stdout.contains('\u{3164}'), "{stdout}");
    assert!(
        stdout.contains("Amazon\\u{3164}EC2 amount=1.00 "),
        "{stdout}"
    );
}

#[test]
fn product_code_hides_tag_character_and_keeps_amount() {
    let scratch = Scratch::new();
    let head = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n";
    let plain = scratch.path().join("plain.csv");
    let tagged = scratch.path().join("tagged.csv");
    fs::write(
        &plain,
        format!("{head}2026-11-01T00:00:00Z,AmazonEC2,BoxUsage,1.00,\n"),
    )
    .unwrap();
    fs::write(
        &tagged,
        format!("{head}2026-11-01T00:00:00Z,Amazon\u{e0001}EC2,BoxUsage,1.00,\n"),
    )
    .unwrap();
    let (code, plain_out, stderr) = run(&[plain.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    let (code, stdout, stderr) = run(&[tagged.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(!stdout.contains('\u{e0001}'), "{stdout}");
    assert!(
        stdout.contains("Amazon\\u{e0001}EC2 amount=1.00 "),
        "{stdout}"
    );
    assert!(plain_out.contains("AmazonEC2 amount=1.00 "), "{plain_out}");
}

#[cfg(target_os = "linux")]
fn run_shell_redirect(redirect: &str) -> (i32, String) {
    let script = format!("exec \"$0\" examples/cur.csv {redirect}");
    let out = Command::new("sh")
        .args(["-c", &script, env!("CARGO_BIN_EXE_cur-top")])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[cfg(target_os = "linux")]
#[test]
fn closed_full_and_null_stdout() {
    let (code, stderr) = run_shell_redirect(">&-");
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("could not write output"), "{stderr}");
    assert!(!stderr.contains("panic"), "{stderr}");
    let (code, stderr) = run_shell_redirect(">/dev/full");
    assert_eq!(code, 2, "{stderr}");
    assert!(!stderr.contains("panic"), "{stderr}");
    let (code, stderr) = run_shell_redirect(">/dev/null");
    assert_eq!(code, 0, "{stderr}");
    let (code, stderr) = run_shell_redirect("1<>/dev/null");
    assert_eq!(code, 0, "{stderr}");
}

/// What a program does when it starts the tool with its output thrown away
/// (`subprocess.DEVNULL` in Python): exit 0, no message.
#[cfg(target_os = "linux")]
#[test]
fn stdio_null_stdout_exits_0_for_a_report_help_and_version() {
    for args in [&["examples/cur.csv"][..], &["--help"], &["--version"]] {
        let status = command(args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .unwrap();
        assert_eq!(status.status.code(), Some(0), "{args:?}");
        assert!(status.stderr.is_empty(), "{:?}", status.stderr);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn help_and_version_to_closed_or_full_stdout_exit_2() {
    for flag in ["--help", "--version"] {
        for redirect in [">&-", ">/dev/full"] {
            let script = format!("exec \"$0\" {flag} {redirect}");
            let out = Command::new("sh")
                .args(["-c", &script, env!("CARGO_BIN_EXE_cur-top")])
                .output()
                .unwrap();
            let stderr = String::from_utf8(out.stderr).unwrap();
            assert_eq!(out.status.code(), Some(2), "{flag} {redirect}: {stderr}");
            assert!(stderr.contains("could not write output"), "{stderr}");
        }
    }
}

#[test]
fn amount_with_more_than_28_decimals_exits_2_and_28_still_works() {
    let scratch = Scratch::new();
    let header = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n";
    let long = scratch.path().join("long.csv");
    fs::write(
        &long,
        format!("{header}2026-11-01T00:00:00Z,A,U,0.00000000000000000000000000001,{{}}\n"),
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[long.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("invalid amount"), "{stderr}");

    let ok = scratch.path().join("ok.csv");
    fs::write(
        &ok,
        format!("{header}2026-11-01T00:00:00Z,A,U,0.0000000000000000000000000001,{{}}\n"),
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[ok.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    // 28 decimals are kept as they are: 0. then 27 zeros then 1, not 0.00.
    let exact = format!("0.{}1", "0".repeat(27));
    assert_eq!(exact.len(), 30, "{exact}");
    assert!(
        stdout.contains(&format!(
            "\ntotals\namount={exact} previous=0.00 delta={exact}\n"
        )),
        "{stdout}"
    );
    assert!(stdout.contains(&format!("\nA amount={exact} ")), "{stdout}");
    assert!(stdout.contains(&format!("\nU amount={exact} ")), "{stdout}");
}

// Windows rejects control characters 1 to 31 in file names, so the file
// cannot be created there.
#[test]
#[cfg(target_os = "linux")]
fn filename_with_esc_byte_exits_2_and_stderr_escapes_it() {
    let scratch = Scratch::new();
    let path = scratch.path().join("bad\u{1b}.txt");
    fs::write(&path, b"x").unwrap();
    let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
    assert_eq!(code, 2, "{stdout:?} {stderr:?}");
    assert!(stdout.is_empty(), "{stdout:?}");
    assert!(
        !stderr.as_bytes().contains(&0x1b),
        "raw ESC byte in stderr: {stderr:?}"
    );
    assert!(stderr.contains("\\u{1b}"), "missing escape: {stderr:?}");
    assert!(
        stderr.contains("not a csv or csv.gz"),
        "reason changed: {stderr:?}"
    );
}

#[test]
fn filename_with_braille_blank_is_escaped_on_error_and_still_counts_when_valid() {
    let scratch = Scratch::new();
    let bad = scratch.path().join("a\u{2800}b.csv.gz");
    fs::write(&bad, b"not gzip content").unwrap();
    let (code, stdout, stderr) = run(&[bad.to_str().unwrap()]);
    assert_eq!(code, 2, "{stdout:?} {stderr:?}");
    assert!(stdout.is_empty(), "{stdout:?}");
    assert!(!stderr.contains('\u{2800}'), "raw U+2800: {stderr:?}");
    assert!(
        stderr.contains("a\\u{2800}b.csv.gz"),
        "missing escape: {stderr:?}"
    );

    let good = scratch.path().join("ok\u{2800}.csv");
    fs::write(&good, three_rows(&[("AmazonEC2", "BoxUsage", "1.50")])).unwrap();
    let (code, stdout, stderr) = run(&[good.to_str().unwrap()]);
    assert_eq!(code, 0, "{stdout:?} {stderr:?}");
    assert!(stdout.contains("amount=1.50"), "{stdout:?}");
}

#[test]
fn nbsp_before_an_amount_or_a_date_exits_2_and_counts_nothing() {
    let scratch = Scratch::new();
    let good_date = "2026-11-01T00:00:00Z";
    let cases = [
        ("amount", format!("{good_date},A,U,\u{a0}1,{{}}\n")),
        ("amount_trailing", format!("{good_date},A,U,1\u{a0},{{}}\n")),
        ("line_sep", format!("{good_date},A,U,\u{2028}1,{{}}\n")),
        ("ideographic", format!("{good_date},A,U,\u{3000}1,{{}}\n")),
        ("date", format!("\u{a0}{good_date},A,U,1,{{}}\n")),
    ];
    let header = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n";
    for (name, row) in cases {
        let path = scratch.path().join(format!("{name}.csv"));
        fs::write(&path, format!("{header}{row}")).unwrap();
        let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
        assert_eq!(code, 2, "{name}: {stderr}");
        assert!(stdout.is_empty(), "{name}: {stdout}");
    }
    let ok = scratch.path().join("space.csv");
    fs::write(&ok, format!("{header}{good_date},A,U, 1,{{}}\n")).unwrap();
    let (code, _, stderr) = run(&[ok.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
}

#[test]
#[cfg(unix)]
fn device_file_exits_2_as_not_a_file() {
    let (code, stdout, stderr) = run(&["/dev/null"]);
    assert_eq!(code, 2, "stdout={stdout} stderr={stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("/dev/null: not a file"), "{stderr}");
}

// macOS file systems refuse names that are not UTF-8 (EILSEQ), so the
// non-UTF-8 name tests run on Linux only.
#[test]
#[cfg(target_os = "linux")]
fn non_utf8_csv_name_with_path_error_shows_byte_escape_not_replacement_char() {
    use std::os::unix::ffi::OsStrExt;
    let scratch = Scratch::new();
    let path = scratch.path().join(OsStr::from_bytes(b"bad\xff.csv"));
    fs::create_dir(&path).unwrap();
    let out = command(&[]).arg(&path).output().unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(2), "{stderr:?}");
    assert!(out.stdout.is_empty(), "{:?}", out.stdout);
    assert!(
        stderr.contains("bad\\xff.csv"),
        "missing byte escape: {stderr:?}"
    );
    assert!(!stderr.contains('\u{fffd}'), "U+FFFD in stderr: {stderr:?}");
}

#[test]
fn usage_error_escapes_raw_direction_newline_and_cr() {
    for (arg, escaped) in [
        ("--amount=a\u{202e}b", "\\u{202e}"),
        ("--amount=a\nb", "\\u{a}"),
        ("--top=1\rx", "\\u{d}"),
        ("--bogus\rx", "\\u{d}"),
    ] {
        let (code, stdout, stderr) = run(&[arg]);
        assert_eq!(code, 2, "{arg:?}");
        assert!(stdout.is_empty(), "{stdout}");
        assert!(
            !stderr.contains('\u{202e}') && !stderr.contains('\r'),
            "{stderr:?}"
        );
        assert!(!stderr.contains("a\nb"), "{stderr:?}");
        assert!(stderr.contains(escaped), "{stderr:?}");
    }
}

/// clap strips ANSI sequences from the text it echoes, so a raw argument
/// that holds an escape sequence plus a line break must still not start a
/// new line on stderr.
#[test]
fn usage_error_with_escape_sequence_and_newline_stays_on_one_line() {
    for (flag, value) in [
        ("--top", "1\x1b[31m\nFAKE: line"),
        ("--top", "1\x1b[0m\nFAKE: line"),
        ("--amount", "1\x1b[31m\nFAKE: line"),
        ("--top", "1\x1b]0;t\x07\nFAKE: osc"),
    ] {
        for joined in [false, true] {
            let args: Vec<String> = if joined {
                vec![format!("{flag}={value}"), "examples/cur.csv".into()]
            } else {
                vec![flag.into(), value.into(), "examples/cur.csv".into()]
            };
            let refs: Vec<&str> = args.iter().map(String::as_str).collect();
            let (code, stdout, stderr) = run(&refs);
            assert_eq!(code, 2, "{args:?}");
            assert!(stdout.is_empty(), "{stdout:?}");
            assert_no_loose_line(&stderr, &args);
        }
    }
    for arg in [
        "--x\x1b[31m\nFAKE: line",
        "--x\x1b]0;t\x07\nFAKE: osc",
        "--x\r\x1b[2K\nFAKE: line",
    ] {
        let (code, stdout, stderr) = run(&[arg]);
        assert_eq!(code, 2, "{arg:?}");
        assert!(stdout.is_empty(), "{stdout:?}");
        assert_no_loose_line(&stderr, &[arg.to_string()]);
    }
}

fn assert_no_loose_line(stderr: &str, args: &[String]) {
    assert!(
        !stderr.contains('\x1b') && !stderr.contains('\x07') && !stderr.contains('\r'),
        "raw control character on stderr: {stderr:?} for {args:?}"
    );
    assert!(
        stderr.lines().all(|line| !line.contains("FAKE")
            || line.contains("\\u{a}")
            || line.contains("\\u{d}")),
        "a line with the typed text but no escape: {stderr:?} for {args:?}"
    );
    assert!(
        stderr.lines().all(|line| !line.starts_with("FAKE")),
        "fake line on stderr: {stderr:?} for {args:?}"
    );
    assert!(stderr.contains("\\u{1b}"), "no \\u{{1b}}: {stderr:?}");
    assert!(stderr.contains("\\u{a}"), "no \\u{{a}}: {stderr:?}");
}

/// A lone line break argument must not turn every line break of the
/// message into `\u{a}`: the message keeps the shape of the same error
/// without the odd argument.
#[test]
fn usage_error_with_a_lone_newline_argument_keeps_clap_line_breaks() {
    let (code, _, plain) = run(&["--top", "x", "a", "examples/cur.csv"]);
    assert_eq!(code, 2);
    let (code, stdout, odd) = run(&["--top", "x", "\n", "examples/cur.csv"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(odd.lines().count(), plain.lines().count(), "{odd:?}");
}

/// A clap error shows an argument that is not UTF-8 with U+FFFD, and a line
/// break in it must not start a new line on stderr either.
#[test]
#[cfg(target_os = "linux")]
fn usage_error_with_a_non_utf8_argument_and_newline_stays_on_one_line() {
    use std::os::unix::ffi::OsStrExt;
    let cases: [&[&[u8]]; 6] = [
        &[b"--x\xff\nFAKE: line"],
        &[b"--json=\xff\nFAKE: v"],
        &[b"--x\n\xffFAKE: line"],
        &[b"--x\xff\x1b[31m\nFAKE: line"],
        &[b"--top", b"\xff\nFAKE: line"],
        &[b"--amount=\xff\r\nFAKE: line"],
    ];
    for case in cases {
        let args: Vec<&OsStr> = case.iter().map(|bytes| OsStr::from_bytes(bytes)).collect();
        let (code, stdout, stderr) = run_os(&args);
        assert_eq!(code, 2, "{case:?}");
        assert!(stdout.is_empty(), "{stdout:?}");
        assert!(
            !stderr.contains('\x1b') && !stderr.contains('\r'),
            "raw control character on stderr: {stderr:?} for {case:?}"
        );
        assert!(
            stderr.lines().all(|line| !line.contains("FAKE")
                || line.contains("\\u{a}")
                || line.contains("\\u{d}")),
            "a fake line on stderr: {stderr:?} for {case:?}"
        );
        if stderr.starts_with("error: invalid UTF-8") {
            // The message does not echo the argument: clap's own line
            // breaks stay.
            assert!(stderr.lines().count() >= 3, "{stderr:?}");
        } else {
            assert!(stderr.contains("\\u{a}"), "no \\u{{a}}: {stderr:?}");
        }
    }
}

#[test]
fn tag_values_that_differ_only_by_a_backslash_stay_apart_in_text() {
    let scratch = Scratch::new();
    let file = scratch.path().join("tagval.csv");
    fs::write(
        &file,
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n\
2026-11-01T00:00:00Z,A,U,1.00,\"{\"\"a\"\":\"\" x\"\"}\"\n\
2026-11-01T00:00:00Z,A,U,2.00,\"{\"\"a\"\":\"\"\\\\u{20}x\"\"}\"\n\
2026-11-01T00:00:00Z,A,U,4.00,\"{\"\"a\"\":\"\"amount=1\"\"}\"\n\
2026-11-01T00:00:00Z,A,U,8.00,\"{\"\"a\"\":\"\"amount\\\\=1\"\"}\"\n",
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[file.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("\na=\\u{20}x amount=1.00 "), "{stdout}");
    assert!(stdout.contains("\na=\\\\u{20}x amount=2.00 "), "{stdout}");
    assert!(stdout.contains("\na=amount\\=1 amount=4.00 "), "{stdout}");
    assert!(stdout.contains("\na=amount\\\\=1 amount=8.00 "), "{stdout}");
}

/// Writes a gzip whose unpacked size is exactly `total` bytes: a header, one
/// row, then blank lines (the CSV reader skips them) so the file stays tiny.
fn write_gz_of_exact_size(path: &Path, total: u64) {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let text = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n2026-11-01T00:00:00Z,A,U,1.00,{}\n";
    let mut enc = GzEncoder::new(fs::File::create(path).unwrap(), Compression::fast());
    enc.write_all(text.as_bytes()).unwrap();
    let mut left = total - text.len() as u64;
    let block = vec![b'\n'; 1 << 20];
    while left > 0 {
        let n = left.min(block.len() as u64) as usize;
        enc.write_all(&block[..n]).unwrap();
        left -= n as u64;
    }
    enc.finish().unwrap();
}

#[test]
fn unpacked_size_limit_accepts_the_edge_and_rejects_one_byte_more() {
    const LIMIT: u64 = 536_870_912;
    let scratch = Scratch::new();
    let edge = scratch.path().join("edge.csv.gz");
    write_gz_of_exact_size(&edge, LIMIT);
    assert!(fs::metadata(&edge).unwrap().len() < 4 << 20);
    let (code, _, stderr) = run(&[edge.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");

    let over = scratch.path().join("over.csv.gz");
    write_gz_of_exact_size(&over, LIMIT + 1);
    let (code, stdout, stderr) = run(&[over.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(
        stderr.contains("file is larger than 536870912 bytes"),
        "{stderr}"
    );
}

#[test]
fn row_number_restarts_in_file_2_for_an_amount_or_field_error() {
    let header = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n";
    let good =
        format!("{header}2026-11-01T00:00:00Z,A,U,1.00,{{}}\n2026-11-01T00:00:00Z,B,U,2.00,{{}}\n");
    for (bad_row, expected) in [
        ("2026-11-01T00:00:00Z,A,U,secret,{}\n", "invalid amount"),
        ("not-a-date,A,U,1.00,{}\n", "file 2: row 1:"),
    ] {
        let scratch = Scratch::new();
        let one = scratch.path().join("one.csv");
        let two = scratch.path().join("two.csv");
        fs::write(&one, &good).unwrap();
        fs::write(&two, format!("{header}{bad_row}")).unwrap();
        let (code, stdout, stderr) = run(&[one.to_str().unwrap(), two.to_str().unwrap()]);
        assert_eq!(code, 2);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.contains("file 2: row 1:"), "{stderr}");
        assert!(stderr.contains(expected), "{stderr}");
        assert!(!stderr.contains("secret"), "{stderr}");
    }
}

#[cfg(target_os = "linux")]
const NON_UTF8_CSV: &str = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,{}\n";

#[test]
#[cfg(target_os = "linux")]
fn non_utf8_name_ending_in_csv_counts_and_later_path_is_read() {
    use std::os::unix::ffi::OsStrExt;
    let scratch = Scratch::new();
    let first = scratch.path().join(OsStr::from_bytes(b"bad\xff.csv"));
    fs::write(&first, NON_UTF8_CSV).unwrap();
    let (code, stdout, stderr) = run_os(&[first.as_os_str()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("amount=1.00"), "{stdout}");
    let second = scratch.path().join("second.csv");
    fs::write(&second, NON_UTF8_CSV).unwrap();
    let (code, stdout, stderr) = run_os(&[first.as_os_str(), second.as_os_str()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("amount=2.00"), "{stdout}");
}

#[test]
#[cfg(target_os = "linux")]
fn non_utf8_name_without_suffix_says_name_is_not_utf8() {
    use std::os::unix::ffi::OsStrExt;
    let scratch = Scratch::new();
    let path = scratch.path().join(OsStr::from_bytes(b"bad\xff.txt"));
    fs::write(&path, NON_UTF8_CSV).unwrap();
    let (code, stdout, stderr) = run_os(&[path.as_os_str()]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("name is not UTF-8"), "{stderr}");
    assert!(stderr.contains("\\xff"), "{stderr}");
    assert!(!stderr.contains("not a csv or csv.gz"), "{stderr}");
}

#[cfg(target_os = "linux")]
fn run_os(args: &[&OsStr]) -> (i32, String, String) {
    let mut cmd = command(&[]);
    cmd.args(args);
    let out = cmd.output().unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

const AMOUNT_HEADER: &str = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n";

#[test]
fn claim_c14_gzip_magic_under_a_csv_name_is_read_as_gzip() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let scratch = Scratch::new();
    let mut enc = GzEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(format!("{AMOUNT_HEADER}2026-11-01T00:00:00Z,A,U,1.50,{{}}\n").as_bytes())
        .unwrap();
    let bytes = enc.finish().unwrap();
    assert_eq!(&bytes[..2], &[0x1f, 0x8b]);
    let path = scratch.path().join("export.csv");
    fs::write(&path, &bytes).unwrap();
    let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("1.50"), "{stdout}");
}

#[test]
fn claim_c17_amount_limit_is_40_characters() {
    let scratch = Scratch::new();
    let amount = |text: &str| {
        let path = scratch.path().join("amount.csv");
        fs::write(
            &path,
            format!("{AMOUNT_HEADER}2026-11-01T00:00:00Z,A,U,{text},{{}}\n"),
        )
        .unwrap();
        run(&[path.to_str().unwrap()])
    };
    let at_limit = format!("{}1.5", "0".repeat(37));
    assert_eq!(at_limit.len(), 40);
    let (code, stdout, stderr) = amount(&at_limit);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("1.50"), "{stdout}");

    let negative = format!("-{}1.5", "0".repeat(36));
    assert_eq!(negative.len(), 40);
    let (code, _, stderr) = amount(&negative);
    assert_eq!(code, 0, "{stderr}");

    let over = format!("{}1.5", "0".repeat(38));
    assert_eq!(over.len(), 41);
    let (code, stdout, stderr) = amount(&over);
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("invalid amount"), "{stderr}");
    assert!(!stderr.contains(&over), "{stderr}");
}

fn run_csv(name: &str, text: &str) -> (i32, String, String) {
    let scratch = Scratch::new();
    let file = scratch.path().join(name);
    fs::write(&file, text).unwrap();
    run(&[file.to_str().unwrap()])
}

/// The input rules of the README, on the binary.
#[test]
fn billing_timestamp_accepts_the_utc_form_and_rejects_the_others() {
    for ok in [
        "2026-11-01T00:00:00Z",
        "2026-11-01T00:00:00.1Z",
        "2026-11-01T00:00:00.123456Z",
        " 2026-11-01T00:00:00Z ",
    ] {
        let (code, stdout, stderr) =
            run_csv("t.csv", &format!("{AMOUNT_HEADER}{ok},A,U,1.00,{{}}\n"));
        assert_eq!(code, 0, "{ok:?}: {stderr}");
        assert!(stdout.contains("month: 2026-11\n"), "{ok:?}: {stdout}");
    }
    for bad in [
        "2026-11-01T00:00:00.1234567Z",
        "2026-11-01",
        "2026-11-01T00:00:00+01:00",
        "2026-11-01 00:00:00",
        "2026-11-01t00:00:00z",
        "2026-02-29T00:00:00Z",
    ] {
        let (code, stdout, stderr) =
            run_csv("t.csv", &format!("{AMOUNT_HEADER}{bad},A,U,1.00,{{}}\n"));
        assert_eq!(code, 2, "{bad:?}");
        assert!(stdout.is_empty(), "{stdout}");
        assert!(
            stderr.contains("invalid bill_billing_period_start_date"),
            "{stderr}"
        );
        assert!(!stderr.contains(bad), "the cell is echoed: {stderr}");
    }
}

#[test]
fn bom_and_crlf_are_accepted_and_a_quote_followed_by_text_is_joined() {
    let crlf = AMOUNT_HEADER.replace('\n', "\r\n");
    let (code, stdout, stderr) = run_csv(
        "bom.csv",
        &format!("\u{feff}{crlf}2026-11-01T00:00:00Z,A,U,1.00,{{}}\r\n"),
    );
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("\nA amount=1.00 "), "{stdout}");
    // The CSV reader is tolerant: `"A"x` is the value `Ax`.
    let (code, stdout, stderr) = run_csv(
        "quote.csv",
        &format!("{AMOUNT_HEADER}2026-11-01T00:00:00Z,\"A\"x,U,1.00,\"{{}}\"\n"),
    );
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("\nAx amount=1.00 "), "{stdout}");
}

#[test]
fn a_file_name_with_an_upper_case_suffix_is_rejected() {
    let (code, stdout, stderr) = run_csv("UPPER.CSV", AMOUNT_HEADER);
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(
        stderr.contains("UPPER.CSV: not a csv or csv.gz"),
        "{stderr}"
    );
    let (code, stdout, stderr) = run_csv("export.CSV", AMOUNT_HEADER);
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(
        stderr.contains("export.CSV: not a csv or csv.gz"),
        "{stderr}"
    );
}

/// Standard input is not read: `-` is a file name, and `/dev/stdin` is
/// `not a file` when it is a pipe or a terminal.
#[test]
#[cfg(target_os = "linux")]
fn standard_input_is_not_read() {
    let scratch = Scratch::new();
    let out = command(&["-"])
        .current_dir(scratch.path())
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("-: No such file or directory"), "{stderr}");
    let out = command(&["/dev/stdin"])
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("/dev/stdin: not a file"), "{stderr}");
}

/// A standard output that was opened for reading only is not recognized: the
/// run exits 0 and writes nothing. This pins the limit that the README names.
#[test]
#[cfg(target_os = "linux")]
fn a_read_only_standard_output_exits_0_and_writes_nothing() {
    let scratch = Scratch::new();
    let file = scratch.path().join("ro.txt");
    fs::write(&file, "keep").unwrap();
    let out = command(&["examples/cur.csv"])
        .stdout(fs::File::open(&file).unwrap())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty(), "{:?}", out.stderr);
    assert_eq!(fs::read_to_string(&file).unwrap(), "keep");
}

/// A CSV has no checksum: a cut inside the last field of the last row goes
/// unnoticed when the amount is the last column, a cut that leaves `2.` does
/// not. The README names this.
#[test]
fn a_csv_cut_inside_the_last_amount_is_summed_and_a_cut_after_the_point_is_an_error() {
    let header = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,resource_tags,line_item_unblended_cost\n";
    let row = "2026-11-01T00:00:00Z,A,U,{},";
    let (code, stdout, stderr) = run_csv("whole.csv", &format!("{header}{row}1.50\n{row}2.50\n"));
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("\namount=4.00 "), "{stdout}");
    let (code, stdout, stderr) = run_csv("cut.csv", &format!("{header}{row}1.50\n{row}2"));
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("\namount=3.50 "), "{stdout}");
    let (code, stdout, stderr) = run_csv("point.csv", &format!("{header}{row}1.50\n{row}2."));
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains("invalid amount"), "{stderr}");
}

/// A gzip file that is cut off, has a wrong CRC, or has bytes after its last
/// member fails the same way, with no report.
#[test]
fn damaged_gzip_files_fail_with_could_not_read_file_and_no_report() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut enc = GzEncoder::new(Vec::new(), Compression::fast());
    let text = format!(
        "{AMOUNT_HEADER}2026-11-01T00:00:00Z,A,U,1.50,{{}}\n2026-11-01T00:00:00Z,A,U,2.50,{{}}\n"
    );
    enc.write_all(text.as_bytes()).unwrap();
    let whole = enc.finish().unwrap();
    let scratch = Scratch::new();
    let good = scratch.path().join("good.csv.gz");
    fs::write(&good, &whole).unwrap();
    let (code, stdout, stderr) = run(&[good.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("\namount=4.00 "), "{stdout}");

    let mut cut = whole.clone();
    cut.truncate(whole.len() - 10);
    let mut crc = whole.clone();
    let at = crc.len() - 8;
    crc[at] ^= 0xff;
    let mut junk = whole.clone();
    junk.extend_from_slice(b"junk");
    for (name, bytes) in [("cut", cut), ("crc", crc), ("junk", junk)] {
        let file = scratch.path().join(format!("{name}.csv.gz"));
        fs::write(&file, bytes).unwrap();
        let (code, stdout, stderr) = run(&[file.to_str().unwrap()]);
        assert_eq!(code, 2, "{name}: {stderr}");
        assert!(stdout.is_empty(), "{name}: {stdout}");
        assert!(
            stderr.contains("file 1: could not read file"),
            "{name}: {stderr}"
        );
    }
}

fn gzip_encode(payload: &[u8]) -> Vec<u8> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut enc = GzEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(payload).unwrap();
    enc.finish().unwrap()
}

/// One stored-block gzip member. `empty_blocks` non-final empty blocks come
/// first, so the compressed size can exceed the unpacked size by about five
/// bytes per block. The last block is final.
fn stored_gzip(payload: &[u8], empty_blocks: usize) -> Vec<u8> {
    let mut deflate = Vec::with_capacity(empty_blocks * 5 + payload.len() + 5);
    for _ in 0..empty_blocks {
        deflate.extend_from_slice(&[0x00, 0x00, 0x00, 0xff, 0xff]);
    }
    if payload.is_empty() {
        if empty_blocks == 0 {
            deflate.extend_from_slice(&[0x01, 0x00, 0x00, 0xff, 0xff]);
        } else {
            let last = deflate.len() - 5;
            deflate[last] = 0x01;
        }
    } else {
        let mut rest = payload;
        while !rest.is_empty() {
            let n = rest.len().min(65535);
            let (chunk, tail) = rest.split_at(n);
            rest = tail;
            let final_bit = if rest.is_empty() { 0x01 } else { 0x00 };
            let len = u16::try_from(n).unwrap();
            deflate.push(final_bit);
            deflate.extend_from_slice(&len.to_le_bytes());
            deflate.extend_from_slice(&(!len).to_le_bytes());
            deflate.extend_from_slice(chunk);
        }
    }
    let mut crc = flate2::Crc::new();
    crc.update(payload);
    let mut out = Vec::with_capacity(18 + deflate.len());
    out.extend_from_slice(&[0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff]);
    out.extend_from_slice(&deflate);
    out.extend_from_slice(&crc.sum().to_le_bytes());
    let isize = u32::try_from(payload.len() & 0xffff_ffff).unwrap();
    out.extend_from_slice(&isize.to_le_bytes());
    out
}

/// Header plus enough copies of one CUR row to reach `min_len` bytes.
fn cur_text_at_least(min_len: usize) -> (String, u64) {
    let header = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n";
    let row = "2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,3.50,{}\n";
    let mut text = String::with_capacity(min_len + row.len());
    text.push_str(header);
    let mut rows: u64 = 0;
    while text.len() < min_len {
        text.push_str(row);
        rows += 1;
    }
    (text, rows)
}

fn repeat_member(member: &[u8], count: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(member.len().saturating_mul(count));
    let chunk_n = 4096.min(count.max(1));
    let chunk = member.repeat(chunk_n);
    let mut left = count;
    while left > 0 {
        if left >= chunk_n {
            out.extend_from_slice(&chunk);
            left -= chunk_n;
        } else {
            out.extend(member.repeat(left));
            break;
        }
    }
    out
}

fn gzip_overhead_message() -> String {
    format!(
        "gzip overhead is larger than {} bytes",
        cur_top::MAX_GZIP_OVERHEAD_BYTES
    )
}

/// Reject when `run` does not finish in 5 seconds with exit 2 and the gzip
/// overhead message. `label` is included in the failure text.
fn assert_gzip_overhead_rejected(label: &str, args: &[&str]) {
    use std::time::{Duration, Instant};

    let started = Instant::now();
    let (code, stdout, stderr) = run(args);
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(5),
        "{label} took {elapsed:?}: stdout={stdout} stderr={stderr}"
    );
    let message = gzip_overhead_message();
    assert_eq!(
        code, 2,
        "{label} took {elapsed:?}: stdout={stdout} stderr={stderr}"
    );
    assert!(stdout.is_empty(), "{label}: {stdout}");
    assert!(stderr.contains(&message), "{label}: {stderr}");
    if let Some(rest) = label.strip_prefix("file ") {
        assert!(
            stderr.contains(&format!("file {rest}: {message}")),
            "{label}: {stderr}"
        );
    }
}

/// One valid gzip member, then `members` copies of one empty member.
fn gzip_then_empty_members(members: u64) -> (Vec<u8>, &'static str) {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let text = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n2026-11-01T00:00:00Z,A,U,1.00,{}\n";
    let mut enc = GzEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(text.as_bytes()).unwrap();
    let mut bytes = enc.finish().unwrap();
    let mut enc = GzEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(b"").unwrap();
    let empty = enc.finish().unwrap();
    let count = usize::try_from(members).unwrap();
    bytes.reserve(empty.len().saturating_mul(count));
    for _ in 0..count {
        bytes.extend_from_slice(&empty);
    }
    (bytes, text)
}

/// Empty gzip members unpack to 0 bytes, so the 536870912-byte cap never
/// fires. Every gzip path (a `.csv.gz` name, a `.csv` name with the gzip
/// magic, and a later file) must reject 200000 of them within 5 seconds.
#[test]
fn many_empty_gzip_members_are_rejected_on_every_gzip_path() {
    use std::time::{Duration, Instant};

    let (bytes, text) = gzip_then_empty_members(200_000);
    let scratch = Scratch::new();
    let good = scratch.path().join("good.csv");
    fs::write(&good, text).unwrap();
    let message = format!(
        "gzip overhead is larger than {} bytes",
        cur_top::MAX_GZIP_OVERHEAD_BYTES
    );
    for name in ["chain.csv.gz", "chain.csv"] {
        let path = scratch.path().join(name);
        fs::write(&path, &bytes).unwrap();
        let started = Instant::now();
        let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{name} took {:?}",
            started.elapsed()
        );
        assert_eq!(code, 2, "{name}: stdout={stdout} stderr={stderr}");
        assert!(stdout.is_empty(), "{name}: {stdout}");
        assert!(stderr.contains(&message), "{name}: {stderr}");
        assert!(stderr.contains("file 1:"), "{name}: {stderr}");
    }
    let path = scratch.path().join("chain.csv.gz");
    let started = Instant::now();
    let (code, stdout, stderr) = run(&[good.to_str().unwrap(), path.to_str().unwrap()]);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "file 2 took {:?}",
        started.elapsed()
    );
    assert_eq!(code, 2, "stdout={stdout} stderr={stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains(&format!("file 2: {message}")), "{stderr}");
}

/// One member may spend 1048576 compressed bytes beyond the bytes it unpacks.
/// One stored block under that line is accepted; one more is rejected. Two
/// empty members that are each inside that line, after one data member, are
/// together past the old whole-file sum and are still accepted.
#[test]
fn gzip_overhead_accepts_the_budget_and_rejects_one_more_member() {
    let text = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n2026-11-01T00:00:00Z,A,U,1.00,{}\n";
    let payload = text.as_bytes();
    // compressed = payload + 10 byte header + 8 byte trailer + 5 byte final
    // stored header + 5 bytes per leading empty block.
    let fixed: u64 = 23;
    let under_blocks = (cur_top::MAX_GZIP_OVERHEAD_BYTES - fixed) / 5;
    let under = stored_gzip(payload, usize::try_from(under_blocks).unwrap());
    let over = stored_gzip(payload, usize::try_from(under_blocks + 1).unwrap());
    assert!(under.len() as u64 <= payload.len() as u64 + cur_top::MAX_GZIP_OVERHEAD_BYTES);
    assert!(over.len() as u64 > payload.len() as u64 + cur_top::MAX_GZIP_OVERHEAD_BYTES);

    let scratch = Scratch::new();
    let message = gzip_overhead_message();
    let over_path = scratch.path().join("over.csv.gz");
    fs::write(&over_path, &over).unwrap();
    let (code, stdout, stderr) = run(&[over_path.to_str().unwrap()]);
    assert_eq!(code, 2, "stdout={stdout} stderr={stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(stderr.contains(&message), "{stderr}");

    for name in ["under.csv.gz", "under.csv"] {
        let path = scratch.path().join(name);
        fs::write(&path, &under).unwrap();
        let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
        assert_eq!(code, 0, "{name}: {stderr}");
        assert!(stdout.contains("amount=1.00"), "{name}: {stdout}");
    }

    // Each empty member stays inside its own allowance. Two of them, after the
    // data member, pass the old whole-file sum without passing the member cap.
    let empty_n = (cur_top::MAX_GZIP_OVERHEAD_BYTES - 18) / 5;
    let empty = stored_gzip(b"", usize::try_from(empty_n).unwrap());
    assert!(empty.len() as u64 <= cur_top::MAX_GZIP_OVERHEAD_BYTES);
    let mut chain = stored_gzip(payload, 0);
    chain.extend_from_slice(&empty);
    chain.extend_from_slice(&empty);
    let members_in_chain: u64 = 3;
    assert!(members_in_chain <= cur_top::MAX_GZIP_MEMBERS);
    let old_allowance = text.len() as u64 + cur_top::MAX_GZIP_OVERHEAD_BYTES;
    assert!(
        chain.len() as u64 > old_allowance,
        "the chain must be past the old whole-file sum: file {} allowance {old_allowance}",
        chain.len()
    );
    for name in ["chain.csv.gz", "chain.csv"] {
        let path = scratch.path().join(name);
        fs::write(&path, &chain).unwrap();
        let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
        assert_eq!(code, 0, "{name}: {stderr}");
        assert!(stdout.contains("amount=1.00"), "{name}: {stdout}");
    }
}

/// A few megabytes of unpacked CUR rows, then more empty members than the
/// member cap. The unpacked prefix used to pay for those members. Every gzip
/// path must still exit 2 within 5 seconds.
#[test]
fn a_large_prefix_plus_too_many_empty_gzip_members_is_rejected() {
    let (text, _rows) = cur_text_at_least(4 * 1024 * 1024);
    let prefix = gzip_encode(text.as_bytes());
    let empty = gzip_encode(b"");
    let empties = usize::try_from(cur_top::MAX_GZIP_MEMBERS * 2).unwrap();
    let mut bytes = prefix.clone();
    bytes.extend(repeat_member(&empty, empties));
    let old_allowance = text.len() as u64 + cur_top::MAX_GZIP_OVERHEAD_BYTES;
    assert!(
        empties as u64 > cur_top::MAX_GZIP_MEMBERS,
        "the chain must pass the member cap"
    );
    assert!(
        bytes.len() as u64 <= old_allowance,
        "setup is already over the old whole-file allowance: file {} allowance {old_allowance}",
        bytes.len()
    );

    let scratch = Scratch::new();
    let good = scratch.path().join("good.csv");
    fs::write(&good, &text).unwrap();
    for name in ["chain.csv.gz", "chain.csv"] {
        let path = scratch.path().join(name);
        fs::write(&path, &bytes).unwrap();
        assert_gzip_overhead_rejected(name, &[path.to_str().unwrap()]);
    }
    let path = scratch.path().join("chain.csv.gz");
    assert_gzip_overhead_rejected("file 2", &[good.to_str().unwrap(), path.to_str().unwrap()]);
}

/// One member's compressed bytes may exceed that member's own unpacked bytes
/// by at most 1048576. A large earlier member does not raise the allowance.
#[test]
fn one_member_cannot_use_an_earlier_members_unpacked_bytes() {
    let (text, rows) = cur_text_at_least(2 * 1024 * 1024);
    let cents = rows * 350;
    let amount = format!("{}.{:02}", cents / 100, cents % 100);
    let prefix = gzip_encode(text.as_bytes());
    // Pure empty stored blocks: compressed size is 18 + 5*n.
    let under_n = (cur_top::MAX_GZIP_OVERHEAD_BYTES - 18) / 5;
    let over_n = under_n + 1;
    let under = stored_gzip(b"", usize::try_from(under_n).unwrap());
    let over = stored_gzip(b"", usize::try_from(over_n).unwrap());
    assert!(under.len() as u64 <= cur_top::MAX_GZIP_OVERHEAD_BYTES);
    assert!(over.len() as u64 > cur_top::MAX_GZIP_OVERHEAD_BYTES);
    let old_allowance = text.len() as u64 + cur_top::MAX_GZIP_OVERHEAD_BYTES;
    let over_file_len = prefix.len() as u64 + over.len() as u64;
    assert!(
        over_file_len <= old_allowance,
        "setup is already over the old whole-file allowance: file {over_file_len} allowance {old_allowance}"
    );

    let scratch = Scratch::new();
    let alone = scratch.path().join("prefix.csv.gz");
    fs::write(&alone, &prefix).unwrap();
    let (code, stdout, stderr) = run(&[alone.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains(&format!("amount={amount}")), "{stdout}");

    let mut legal = prefix.clone();
    legal.extend_from_slice(&under);
    for name in ["legal.csv.gz", "legal.csv"] {
        let path = scratch.path().join(name);
        fs::write(&path, &legal).unwrap();
        let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
        assert_eq!(code, 0, "{name}: {stderr}");
        assert!(
            stdout.contains(&format!("amount={amount}")),
            "{name}: {stdout}"
        );
    }

    let mut illegal = prefix;
    illegal.extend_from_slice(&over);
    for name in ["illegal.csv.gz", "illegal.csv"] {
        let path = scratch.path().join(name);
        fs::write(&path, &illegal).unwrap();
        assert_gzip_overhead_rejected(name, &[path.to_str().unwrap()]);
    }
}

/// Many members that each hold rows stay under the member cap and still sum.
#[test]
fn many_gzip_members_that_hold_rows_are_still_read() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use flate2::GzBuilder;
    use std::io::Write;

    let members: usize = 20_000;
    assert!(members as u64 <= cur_top::MAX_GZIP_MEMBERS);
    let header = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n";
    let rows = "2026-11-01T00:00:00Z,A,U,1.00,{}\n2026-11-01T00:00:00Z,B,U,2.00,{}\n";
    let mut first = GzBuilder::new()
        .filename("export.csv")
        .comment("rows")
        .extra(vec![1, 2, 3, 4])
        .write(Vec::new(), Compression::fast());
    first.write_all(header.as_bytes()).unwrap();
    first.write_all(rows.as_bytes()).unwrap();
    let mut bytes = first.finish().unwrap();
    let mut rest = GzEncoder::new(Vec::new(), Compression::fast());
    rest.write_all(rows.as_bytes()).unwrap();
    let row_member = rest.finish().unwrap();
    bytes.extend(repeat_member(&row_member, members - 1));

    let scratch = Scratch::new();
    for name in ["many.csv.gz", "many.csv"] {
        let path = scratch.path().join(name);
        fs::write(&path, &bytes).unwrap();
        let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
        assert_eq!(code, 0, "{name}: {stderr}");
        // 20000 members, each 1.00 + 2.00.
        assert!(stdout.contains("amount=60000.00"), "{name}: {stdout}");
    }
}

/// One CUR member, then empty stored members. Together they pass `min_len`
/// compressed bytes. Each empty member stays inside the per-member allowance.
fn gzip_then_stored_over(min_len: u64) -> (Vec<u8>, String) {
    let text = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n2026-11-01T00:00:00Z,A,U,1.00,{}\n".to_string();
    let mut bytes = gzip_encode(text.as_bytes());
    let empty = stored_gzip(b"", 8_000);
    assert!(empty.len() as u64 <= cur_top::MAX_GZIP_OVERHEAD_BYTES);
    assert!(bytes.len() as u64 <= cur_top::MAX_GZIP_OVERHEAD_BYTES);
    let mut members: u64 = 1;
    while (bytes.len() as u64) < min_len {
        bytes.extend_from_slice(&empty);
        members += 1;
        assert!(members <= cur_top::MAX_GZIP_MEMBERS);
    }
    (bytes, text)
}

/// Compressed bytes are counted across every gzip member. A chain of stored
/// members stays under the per-member allowance and the member cap, and is
/// still rejected once the total passes. The total used here is far under
/// the binary's cap, so the file stays small.
#[test]
fn gzip_compressed_total_is_rejected_on_every_gzip_path() {
    assert_eq!(cur_top::MAX_GZIP_COMPRESSED_BYTES, 1_073_741_824);
    // Past one member's allowance, still far under the binary cap of 1 GiB.
    let (bytes, text) = gzip_then_stored_over(cur_top::MAX_GZIP_OVERHEAD_BYTES + 80_000);
    let cap = bytes.len() as u64;
    assert!(cap < cur_top::MAX_GZIP_COMPRESSED_BYTES);
    assert!(cap > cur_top::MAX_GZIP_OVERHEAD_BYTES);

    let scratch = Scratch::new();
    let good = scratch.path().join("good.csv");
    fs::write(&good, &text).unwrap();
    let opts = cur_top::Options {
        amount_column: "line_item_unblended_cost".to_string(),
        top: 10,
        json: false,
    };
    // The file is exactly `cap` bytes. One byte less rejects it; `cap` accepts it.
    let edge = cap - 1;
    for name in ["chain.csv.gz", "chain.csv"] {
        let path = scratch.path().join(name);
        fs::write(&path, &bytes).unwrap();
        let err =
            cur_top::execute_paths_limited(std::slice::from_ref(&path), &opts, edge).unwrap_err();
        assert!(
            err.message()
                .contains(&format!("gzip input is larger than {edge} bytes")),
            "{name}: {}",
            err.message()
        );
        assert!(
            err.message().contains("file 1:"),
            "{name}: {}",
            err.message()
        );
        assert!(!err.message().contains("1.00"), "{name}: {}", err.message());
        let summary =
            cur_top::execute_paths_limited(std::slice::from_ref(&path), &opts, cap).expect(name);
        assert!(summary.stdout.contains("amount=1.00"), "{name}");
    }
    let path = scratch.path().join("chain.csv.gz");
    let err = cur_top::execute_paths_limited(&[good.clone(), path], &opts, edge).unwrap_err();
    assert!(
        err.message()
            .contains(&format!("file 2: gzip input is larger than {edge} bytes")),
        "{}",
        err.message()
    );
}

/// A normal gzip of a few megabytes stays under the total compressed cap.
#[test]
fn a_large_ordinary_gzip_under_the_compressed_cap_still_runs() {
    let (text, rows) = cur_text_at_least(2 * 1024 * 1024);
    let cents = rows * 350;
    let amount = format!("{}.{:02}", cents / 100, cents % 100);
    let bytes = stored_gzip(text.as_bytes(), 0);
    assert!(bytes.len() as u64 > cur_top::MAX_GZIP_OVERHEAD_BYTES);
    assert!(bytes.len() as u64 <= cur_top::MAX_GZIP_COMPRESSED_BYTES);
    assert!(text.len() as u64 <= cur_top::MAX_UNCOMPRESSED_BYTES);

    let scratch = Scratch::new();
    for name in ["wide.csv.gz", "wide.csv"] {
        let path = scratch.path().join(name);
        fs::write(&path, &bytes).unwrap();
        let (code, stdout, stderr) = run(&[path.to_str().unwrap()]);
        assert_eq!(code, 0, "{name}: {stderr}");
        assert!(
            stdout.contains(&format!("amount={amount}")),
            "{name}: {stdout}"
        );
    }
}
