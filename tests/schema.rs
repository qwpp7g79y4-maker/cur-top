//! Every JSON document starts with `"schema_version": 1`. A failing run
//! prints no JSON at all: the error is one line on standard error.

use std::fs;
use std::path::PathBuf;
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

fn assert_versioned(json: &str) {
    assert!(
        json.starts_with("{\n  \"schema_version\": 1,\n"),
        "schema_version is not the first key: {json}"
    );
    let value: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(
        value.as_object().unwrap().keys().next().unwrap(),
        "schema_version"
    );
}

#[test]
fn every_json_path_has_schema_version_1_first() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("schema");
    fs::create_dir_all(&dir).unwrap();
    let single = dir.join("single-month.csv");
    fs::write(
        &single,
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_net_unblended_cost,line_item_unblended_rate,line_item_usage_amount,resource_tags\n\
         2026-11-01T00:00:00Z,AmazonS3,T,1.25,1,1,\"{}\"\n",
    )
    .unwrap();
    let single = single.to_str().unwrap();
    let written = dir.join("written.json");
    let written_text = written.to_str().unwrap();

    let runs: Vec<Vec<&str>> = vec![
        vec!["--json", "examples/cur.csv"],
        vec!["examples/cur.csv", "--json", "--top", "1"],
        vec!["--json", "--quiet", "examples/cur.csv"],
        vec!["--json", "--verbose", "examples/cur.csv"],
        vec!["--json", "examples/cur.csv", "examples/cur.csv"],
        vec!["--json", "--amount", "line_item_net_unblended_cost", single],
    ];
    for args in runs {
        let (code, out, err) = run(&args);
        assert_eq!(code, 0, "{args:?}: {err}");
        assert_versioned(&out);
    }

    // The file written by --output is the same document.
    let (code, out, err) = run(&["--json", "--output", written_text, "examples/cur.csv"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.is_empty());
    assert_versioned(&fs::read_to_string(&written).unwrap());
}

#[test]
fn a_failing_json_run_prints_no_json_only_one_error_line() {
    for args in [
        &["--json", "examples/does-not-exist.csv"][..],
        &["--json", "--amount", "nope", "examples/cur.csv"][..],
        &["--json", "--md", "examples/cur.csv"][..],
    ] {
        let (code, out, err) = run(args);
        assert_eq!(code, 2, "{args:?}: {err}");
        assert!(out.is_empty(), "{args:?}: {out}");
        assert!(!err.contains("schema_version"), "{err}");
        assert!(!err.trim().is_empty());
    }
}
