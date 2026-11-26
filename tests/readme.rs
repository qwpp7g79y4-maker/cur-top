//! README.md is executable: console blocks match the binary, and every
//! numbered claim has a test.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn read(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {rel}: {err}"))
}

fn section<'a>(text: &'a str, heading: &str) -> &'a str {
    let marker = format!("{heading}\n");
    let start = text
        .find(&marker)
        .unwrap_or_else(|| panic!("missing heading {heading}"))
        + marker.len();
    let rest = &text[start..];
    match rest.find("\n## ") {
        Some(end) => &rest[..end],
        None => rest,
    }
}

fn rust_sources() -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut out = String::new();
    for dir in ["src", "tests"] {
        for entry in fs::read_dir(root.join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                out.push_str(&fs::read_to_string(&path).unwrap());
                out.push('\n');
            }
        }
    }
    out
}

fn console_blocks(doc: &str) -> Vec<(String, Vec<String>, String)> {
    let mut out = Vec::new();
    let mut lines = doc.lines();
    while let Some(line) = lines.next() {
        if line != "```console" {
            continue;
        }
        let call = lines.next().expect("empty console block");
        let args: Vec<String> = call
            .strip_prefix("$ cur-top ")
            .unwrap_or_else(|| panic!("console block does not start with `$ cur-top `: {call}"))
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let mut body = String::new();
        let mut closed = false;
        for line in lines.by_ref() {
            if line == "```" {
                closed = true;
                break;
            }
            body.push_str(line);
            body.push('\n');
        }
        assert!(closed, "console block not closed: {call}");
        out.push((call.to_string(), args, body));
    }
    assert!(!out.is_empty(), "README has no console block");
    out
}

#[test]
fn readme_sections_follow_the_documentation_standard() {
    let text = read("README.md");
    // A `## ` line inside a code fence is example output, not a heading.
    let mut fenced = false;
    let headings: Vec<_> = text
        .lines()
        .filter(|line| {
            if line.starts_with("```") {
                fenced = !fenced;
            }
            !fenced && line.starts_with("## ")
        })
        .map(|line| line.trim_start_matches("## "))
        .collect();
    assert_eq!(
        headings,
        vec![
            "Why",
            "Installation",
            "Quick start",
            "Quickstart in 5 minutes",
            "Options",
            "Example",
            "How it counts",
            "Exit codes",
            "Limitations",
            "Maintenance",
            "Support",
            "License",
            "Contact"
        ]
    );
}

#[test]
fn install_uses_a_local_path() {
    let readme = read("README.md");
    let install = section(&readme, "## Installation");
    assert!(install.contains("cargo install --locked --path ."));
    assert!(install.contains("1.85"));
    assert!(install.contains("cargo install cur-top"));
    assert!(!install.contains("not published"));
}

#[test]
fn quickstart_heredoc_matches_the_sample_file() {
    let readme = read("README.md");
    let mut lines = readme.lines();
    let mut found = false;
    for line in lines.by_ref() {
        if line == "cat > my-cur.csv <<'CSV'" {
            found = true;
            break;
        }
    }
    assert!(found, "quick start heredoc is missing");
    let mut body = String::new();
    for line in lines {
        if line == "CSV" {
            break;
        }
        body.push_str(line);
        body.push('\n');
    }
    assert_eq!(body, read("examples/cur.csv"));
}

#[test]
fn console_examples_match_the_binary() {
    let readme = read("README.md");
    for (call, args, expected) in console_blocks(&readme) {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_cur-top"));
        cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
        cmd.env("NO_COLOR", "1");
        cmd.args(&args);
        let out = cmd.output().unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(out.status.success(), "{call} failed: {stderr}");
        assert!(stderr.is_empty(), "{call} wrote to stderr: {stderr}");
        assert_eq!(stdout, expected, "console mismatch for {call}");
        assert!(!stdout.contains("123456789012"));
    }
}

#[test]
fn every_how_it_counts_claim_has_a_test() {
    let readme = read("README.md");
    let how = section(&readme, "## How it counts");
    let mut ids = Vec::new();
    for line in how.lines() {
        let Some(rest) = line.trim().strip_prefix('C') else {
            continue;
        };
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() || !rest[digits.len()..].starts_with('.') {
            continue;
        }
        ids.push(digits.parse::<u32>().unwrap());
    }
    assert_eq!(ids, (1..=18).collect::<Vec<_>>());
    let sources = rust_sources();
    for id in 1..=18 {
        let needle = format!("fn claim_c{id}_");
        assert!(sources.contains(&needle), "missing {needle}");
    }
    assert!(sources.contains("fn quality_benchmark_100_mb"));
}

#[test]
fn limitations_name_behaviors_the_tests_cover() {
    let readme = read("README.md");
    let limits = section(&readme, "## Limitations");
    for phrase in [
        "no network",
        "AWS SDK",
        "Parquet",
        "Athena",
        "does not multiply",
        "resource_tags",
        "csv.gz",
        "symlink",
        "directory",
        "536870912",
        "1048576",
        "counted twice",
        "previous month",
        "zip",
        "100 MiB",
        "not a forecast",
        "line_item_unblended_rate",
        "line_item_usage_amount",
    ] {
        assert!(limits.contains(phrase), "limitations missing {phrase}");
    }
    let sources = rust_sources();
    for needle in [
        "fn quality_benchmark_100_mb",
        "fn unpacked_size_limit_accepts_the_edge_and_rejects_one_byte_more",
        "fn directory_is_rejected_and_symlink_is_followed",
        "fn claim_c1_",
        "fn claim_c14_",
        "fn claim_c16_",
    ] {
        assert!(sources.contains(needle), "missing {needle}");
    }
}

#[test]
fn licenses_changelog_and_cargo_metadata() {
    let mit = read("LICENSE-MIT");
    let apache = read("LICENSE-APACHE");
    assert!(mit.contains("Copyright (c) 2026 PEPAI"));
    assert!(apache.contains("Copyright 2026 PEPAI"));
    let changelog = read("CHANGELOG.md");
    assert!(changelog.contains("## [0.1.0] - 2026-11-09"));
    assert!(changelog.contains("not published"));
    assert!(changelog.contains("no git tag"));
    let cargo = read("Cargo.toml");
    assert!(cargo.contains("MIT OR Apache-2.0"));
    assert!(cargo.contains("PEPAI <pep_ai@icloud.com>"));
    assert!(cargo.contains("rust-version = \"1.85\""));
    // Files that belong to the repository only stay out of the package.
    // Cargo writes the list on one line in the repository and over several
    // lines in the packaged Cargo.toml, so read from `exclude = [` to its `]`.
    let start = cargo
        .find("exclude = [")
        .expect("Cargo.toml has an exclude list");
    let exclude = cargo[start..]
        .split_once(']')
        .expect("the exclude list is closed")
        .0;
    for path in [
        "/.github/",
        "/.gitattributes",
        "/scripts/",
        concat!("/blue", "print.md"),
    ] {
        assert!(
            exclude.contains(&format!("\"{path}\"")),
            "{path} is not excluded"
        );
    }
    assert!(!cargo.contains(concat!("tas", "k.md")));
}

#[test]
fn documented_test_total_matches_real_count() {
    let attr = concat!("#[", "test]");
    let ignored = concat!("#[", "ignore");
    let sources = rust_sources();
    let real = sources.lines().filter(|line| line.trim() == attr).count()
        - sources
            .lines()
            .filter(|line| line.trim().starts_with(ignored))
            .count();
    let number_before = |text: &str, suffix: &str| -> usize {
        let end = text
            .find(suffix)
            .unwrap_or_else(|| panic!("missing {suffix}"));
        text[..end]
            .rsplit(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    assert_eq!(number_before(&read("README.md"), "-test suite"), real);
    let quality = read("QUALITY.md");
    let suite = quality.find("Suite: ").expect("missing Suite line");
    assert_eq!(
        quality[suite + 7..]
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<usize>()
            .unwrap(),
        real
    );
    assert!(quality.contains(&format!("| total | {real} | 0 |")));
}

#[test]
fn the_options_section_names_every_long_option_of_help() {
    let help = Command::new(env!("CARGO_BIN_EXE_cur-top"))
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8(help.stdout).unwrap();
    let readme = read("README.md");
    let options = section(&readme, "## Options");
    let mut found = 0;
    for word in help.split_whitespace() {
        if let Some(name) = word.strip_prefix("--") {
            let name = name.trim_end_matches(',');
            assert!(
                options.contains(&format!("`--{name}")),
                "--{name} is not documented"
            );
            found += 1;
        }
    }
    assert!(found >= 7, "found only {found} long options in --help");
    for code in ["| 0 |", "| 2 |", "| 130 |"] {
        assert!(section(&readme, "## Exit codes").contains(code), "{code}");
    }
}

/// Headings outside code fences, in order.
fn headings_outside_fences(text: &str) -> Vec<String> {
    let mut fenced = false;
    text.lines()
        .filter(|line| {
            if line.starts_with("```") {
                fenced = !fenced;
            }
            !fenced && line.starts_with("## ")
        })
        .map(|line| line.trim_start_matches("## ").to_string())
        .collect()
}

/// Every run of digits (with inner dots) in the text, without repeats.
fn numbers(text: &str) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    let mut current = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() || (c == '.' && !current.is_empty()) {
            current.push(c);
        } else {
            let number = current.trim_end_matches('.');
            if !number.is_empty() {
                out.insert(number.to_string());
            }
            current.clear();
        }
    }
    out
}

#[test]
fn the_dutch_readme_has_the_same_sections_examples_claims_and_numbers() {
    let en = read("README.md");
    let nl = read("README.nl.md");
    let map = [
        ("Why", "Waarom"),
        ("Installation", "Installatie"),
        ("Quick start", "Snelstart"),
        ("Quickstart in 5 minutes", "Aan de slag in 5 minuten"),
        ("Options", "Opties"),
        ("Example", "Voorbeeld"),
        ("How it counts", "Hoe het telt"),
        ("Exit codes", "Afsluitcodes"),
        ("Limitations", "Beperkingen"),
        ("Maintenance", "Onderhoud"),
        ("Support", "Ondersteuning"),
        ("License", "Licentie"),
        ("Contact", "Contact"),
    ];
    let expected: Vec<String> = map.iter().map(|(_, nl)| nl.to_string()).collect();
    assert_eq!(headings_outside_fences(&nl), expected);
    let blocks = |text: &str| -> Vec<(String, String)> {
        console_blocks(text)
            .into_iter()
            .map(|(call, _, body)| (call, body))
            .collect()
    };
    assert_eq!(blocks(&en), blocks(&nl), "console examples differ");
    let claims = |text: &str| -> Vec<String> {
        text.lines()
            .filter_map(|line| {
                let rest = line.strip_prefix('C')?;
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                (!digits.is_empty() && rest[digits.len()..].starts_with(". ")).then_some(digits)
            })
            .collect()
    };
    assert_eq!(claims(&en), claims(&nl));
    assert_eq!(
        numbers(&en),
        numbers(&nl),
        "the two READMEs state different numbers"
    );
    let real = rust_sources()
        .lines()
        .filter(|line| line.trim() == concat!("#[", "test]"))
        .count()
        - 1;
    assert!(nl.contains(&format!("{real} tests")), "{real} tests");
}

#[test]
fn the_example_files_match_the_binary() {
    for (flag, file) in [
        (None, "examples/cur.expected.txt"),
        (Some("--json"), "examples/cur.expected.json"),
        (Some("--md"), "examples/cur.expected.md"),
    ] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_cur-top"));
        cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
        cmd.args(flag).arg("examples/cur.csv");
        let out = cmd.output().unwrap();
        assert!(out.status.success(), "{file}");
        assert_eq!(String::from_utf8(out.stdout).unwrap(), read(file), "{file}");
    }
    let readme = read("examples/README.md");
    for file in ["cur.expected.txt", "cur.expected.json", "cur.expected.md"] {
        assert!(
            readme.contains(file),
            "{file} is not named in examples/README.md"
        );
    }
}

/// The status of each claim row (`C<n>`, `L<n>`, `Q<n>`) of a QUALITY file.
fn claim_statuses(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line.split(" | ").collect();
            let id = cells.first()?.strip_prefix("| ")?.trim();
            let mut chars = id.chars();
            let ok = matches!(chars.next(), Some('C' | 'L' | 'Q'))
                && chars.clone().count() > 0
                && chars.all(|c| c.is_ascii_digit());
            ok.then(|| {
                let status = cells.last().unwrap().trim_end_matches(" |").trim();
                (id.to_string(), status.to_string())
            })
        })
        .collect()
}

#[test]
fn quality_claim_summary_matches_the_table_rows() {
    for (file, tested, part, ignored, gap) in [
        (
            "QUALITY.md",
            "tested",
            "tested in part",
            "test exists, ignored",
            "gap",
        ),
        (
            "QUALITY.nl.md",
            "getest",
            "deels getest",
            "test bestaat, genegeerd",
            "gat",
        ),
    ] {
        let text = read(file);
        let rows = claim_statuses(&text);
        let count = |f: &dyn Fn(&str) -> bool| rows.iter().filter(|(_, s)| f(s)).count();
        let n_part = count(&|s| s.starts_with(part));
        let n_tested = count(&|s| s.starts_with(tested) && !s.starts_with(part));
        let n_ignored = count(&|s| s.starts_with(ignored));
        let n_gap = count(&|s| s.starts_with(gap));
        assert_eq!(n_tested + n_part + n_ignored + n_gap, rows.len(), "{file}");
        let summary = text
            .lines()
            .find(|l| l.contains(" claims: "))
            .unwrap_or_else(|| panic!("{file}: no summary line"));
        let mut bare = String::new();
        let mut depth = 0;
        for c in summary.chars() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ if depth == 0 => bare.push(c),
                _ => {}
            }
        }
        let numbers: Vec<usize> = bare
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|p| p.parse().ok())
            .collect();
        assert_eq!(
            numbers[..5],
            [rows.len(), n_tested, n_part, n_ignored, n_gap],
            "{file}: {summary}"
        );
    }
}
