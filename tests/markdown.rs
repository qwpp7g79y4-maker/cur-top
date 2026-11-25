//! `--md`: the same amounts as the text and JSON reports, in tables.

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

/// The `amount previous delta` triples of the text report, in order.
fn text_triples(text: &str) -> Vec<[String; 3]> {
    text.lines()
        .filter(|line| line.contains(" amount=") && line.contains(" previous="))
        .map(|line| {
            let words: Vec<&str> = line.rsplitn(4, ' ').collect();
            [
                words[2].trim_start_matches("amount=").to_string(),
                words[1].trim_start_matches("previous=").to_string(),
                words[0].trim_start_matches("delta=").to_string(),
            ]
        })
        .collect()
}

fn json_triples(json: &str) -> Vec<[String; 3]> {
    let value: serde_json::Value = serde_json::from_str(json).unwrap();
    let mut out = Vec::new();
    for group in ["service", "usage_type", "tag"] {
        for line in value[group].as_array().unwrap() {
            out.push(
                ["amount", "previous", "delta"].map(|f| line[f].as_str().unwrap().to_string()),
            );
        }
    }
    out
}

/// The three number cells of every key row (not headers, dividers or totals).
fn md_triples(md: &str) -> Vec<[String; 3]> {
    md.lines()
        .filter(|line| {
            line.starts_with("| ") && !line.starts_with("| Key") && !line.contains("| ---")
        })
        .filter_map(|line| {
            let cells: Vec<&str> = line.trim_end_matches(" |").rsplitn(4, " | ").collect();
            (cells.len() == 4).then(|| {
                [
                    cells[2].to_string(),
                    cells[1].to_string(),
                    cells[0].to_string(),
                ]
            })
        })
        .collect()
}

#[test]
fn markdown_amounts_equal_text_and_json() {
    let (code, md, err) = run(&["--md", "examples/cur.csv"]);
    assert_eq!(code, 0, "{err}");
    let (_, text, _) = run(&["examples/cur.csv"]);
    let (_, json, _) = run(&["--json", "examples/cur.csv"]);
    let from_md = md_triples(&md);
    assert_eq!(from_md.len(), 5, "{md}");
    assert_eq!(from_md, text_triples(&text), "{md}");
    assert_eq!(from_md, json_triples(&json), "{md}");
    assert!(md.starts_with("# cur-top\n"), "{md}");
    assert!(md.contains("| 4.50 | 1.00 | 3.50 |"), "{md}");
    assert!(md.contains("Totals"), "{md}");
}

#[test]
fn markdown_says_the_amounts_are_read_not_estimated() {
    let (_, md, _) = run(&["--md", "examples/cur.csv"]);
    assert!(
        md.contains("Not an estimate: the amounts are summed from the file."),
        "{md}"
    );
}

#[test]
fn md_with_json_is_a_usage_error() {
    let (code, out, err) = run(&["--md", "--json", "examples/cur.csv"]);
    assert_eq!(code, 2, "{err}");
    assert!(out.is_empty(), "{out}");
    assert!(err.contains("cannot be used with"), "{err}");
}

#[test]
fn a_pipe_and_a_line_break_in_a_key_stay_in_one_cell() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("markdown-escape");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("cur.csv");
    std::fs::write(
        &file,
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,line_item_unblended_rate,line_item_usage_amount,resource_tags\n\
         2026-11-01T00:00:00Z,A,T,2.00,1,1,\"{\"\"k\"\":\"\"a|b\\u2028c\"\"}\"\n",
    )
    .unwrap();
    let (code, md, err) = run(&["--md", file.to_str().unwrap()]);
    assert_eq!(code, 0, "{err}");
    assert!(md.contains("| k=a\\|b\\u{2028}c | 2.00 |"), "{md}");
    for line in md.lines().filter(|l| l.starts_with("| k=")) {
        assert!(line.ends_with(" |"), "{line}");
    }
}

#[test]
fn markdown_and_html_in_a_key_are_escaped() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("markdown-escape-html");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("cur.csv");
    std::fs::write(
        &file,
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,line_item_unblended_rate,line_item_usage_amount,resource_tags\n\
         2026-11-01T00:00:00Z,\"`x`*_[l](http://e.com)<b>&amp;\",\"# h\\|\",2.00,1,1,\"{\"\"a\"\":\"\"<script>alert(1)</script>\"\",\"\"b\"\":\"\"![i](x)\"\"}\"\n",
    )
    .unwrap();
    let (code, md, err) = run(&["--md", file.to_str().unwrap()]);
    assert_eq!(code, 0, "{err}");
    assert!(
        md.contains("\\`x\\`\\*\\_\\[l\\]\\(http\\://e.com\\)\\<b\\>\\&amp;"),
        "{md}"
    );
    assert!(
        md.contains("a=\\<script\\>alert\\(1\\)\\</script\\>"),
        "{md}"
    );
    assert!(md.contains("b=\\!\\[i\\]\\(x\\)"), "{md}");
    assert!(md.contains("\\# h\\\\\\|"), "{md}");
    // a bare address, strikethrough, and an e-mail address must not render as a link or as text effects
    let file = dir.join("autolink.csv");
    std::fs::write(
        &file,
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,line_item_unblended_rate,line_item_usage_amount,resource_tags\n\
         2026-11-01T00:00:00Z,https://evil.example/login,WWW.evil.example,1.00,1,1,\"{\"\"k\"\":\"\"~~x~~\"\",\"\"m\"\":\"\"a@b.example\"\"}\"\n",
    )
    .unwrap();
    let (code, md2, err) = run(&["--md", file.to_str().unwrap()]);
    assert_eq!(code, 0, "{err}");
    assert!(md2.contains("| https\\://evil.example/login |"), "{md2}");
    assert!(md2.contains("| WWW\\.evil.example |"), "{md2}");
    assert!(md2.contains("| k=\\~\\~x\\~\\~ |"), "{md2}");
    assert!(md2.contains("| m=a\\@b.example |"), "{md2}");
    for bad in ["<script", "<b>", "![", "](", "`x`"] {
        for line in md.lines().filter(|l| l.starts_with("| ")) {
            for (at, _) in line.match_indices(bad) {
                assert!(line[..at].ends_with('\\'), "{bad} unescaped in {line}");
            }
        }
    }
}
