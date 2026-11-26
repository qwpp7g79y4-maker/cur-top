# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-11-19

Changes made after 2026-11-19; the version number did not change.

### Added

- `THIRD-PARTY-LICENSES.html`: the licenses of the third-party crates in the binary, generated with `cargo about generate --locked about.hbs`.
- Documentation: `README.md` follows the PEPAI documentation standard (Installation, Quick start, Options, Example, Exit codes, Support, Contact), `README.nl.md` is the Dutch version (a test keeps its headings, examples, claims, and numbers equal to the English one), `CONTRIBUTING.md`, `QUALITY.nl.md`, and `examples/README.md` with the expected output of the example (checked by a test). `SECURITY.md` names the contact address, and `QUALITY.md` names the tested commit and lists what was not run.
- `.github/workflows/release.yml`: on a `v*` tag it runs the CI workflow, builds the binary for five targets, checks the glibc floor, adds the license page, and attaches archives with SHA-256 files to the release. It has not run yet.
- `about.hbs` and `about.toml`: the third-party licenses of the crates in the binary, for the release archive (`cargo about generate about.hbs`). A test keeps `about.toml` in line with `deny.toml`.
- `build.rs`: a debug build prints the git commit after the version in `--version` (`cur-top 1.0.0 (<sha>)`). A release build, and a build without git, print the version only.
- `"schema_version": 1` is the first key of every JSON report. A failing run prints no JSON.
- `--output FILE` writes the report (text, `--json` or `--md`) to a file instead of standard output. The file is written whole through a temporary file in the same directory and a rename; an existing file is replaced, a symbolic link is replaced and not followed, a missing directory is not created, an input file is refused as output, and an existing pipe, socket, or device is refused with `not a regular file`. A write error exits 2 with `could not write report to`.
- SIGINT (Ctrl-C) on Unix while the files are read exits 130 with no report, an empty standard output, and the line `cur-top: interrupted by SIGINT` on standard error. Before, the process was ended by the signal and had no exit code.
- `--md` prints the report as Markdown tables with the same amounts as the text and JSON reports. `|` in a key is written `\|` and a line break or other hidden character `\u{..}`; a backslash, a backtick, and `* _ [ ] ( ) < > & ! ~ @` (and a `#` at the start, a `:` before `/`, and the `.` after `www`) get a backslash, so a key is never read as Markdown or HTML. `--md` with `--json` is a usage error, exit 2.
- `--quiet` and `--verbose`. A successful run prints nothing on stderr. `--verbose` adds one `context:` line on stderr (`context: rows 3, service 2, usage_type 2, tag 1`); `--quiet` never does. The report on stdout is unchanged, an error prints the same line with either option, and exit codes stay 0 and 2. `--quiet` together with `--verbose` is a usage error, exit 2. cur-top has no `what to do` line, so the options only decide the context line.

### Changed

- `README.md` and `README.nl.md` have a Maintenance (Onderhoud) section after Limitations: bugs through GitHub Issues, dormant after a release, new versions only for a bug, a request, or a security problem. The README tests list the new heading.
- The README has the sections of the documentation standard, in the same order as `README.nl.md`. The test suite is 168 tests (counted on Linux: 168 passed, 1 ignored, 0 failed), measured after the last change. `QUALITY.md` names the tested commit and lists what was not run.
- The stdout check in `src/stdout_probe.rs` is no longer the only `unsafe` code: the SIGINT handler in `src/interrupt.rs` is too. The library keeps `#![forbid(unsafe_code)]`.


### Changed

- Version 1.0.0. CI runs on Linux, Windows, and macOS, with a Rust 1.85 job, `cargo audit`, and `cargo deny` license checks.
- `quality_benchmark_100_mb` is ignored by default.
- Tests that need symlinks run on Unix only, tests that need a file name with a control character, a closed or full standard output, or a name that is not UTF-8 run on Linux only.
- `Cargo.toml` `exclude` keeps repository-only files out of the package. `homepage` is `https://www.pepai.nl`, and the `documentation` field is gone, as in cron-ledger and ci-bill.
- `README.md` says the product code and usage type limit is 1024 bytes, which is what the code and its test check.
- `QUALITY.md` holds the measured result (136 passed, 1 ignored, 0 failed; counted on Linux), a "Not run" list, known limits, "Open points", and a claim → test table that lists the claims without a test.
- `.gitattributes` sets `* text=auto eol=lf`, so a checkout with CRLF line endings still matches the man page and the README byte for byte.
- The README quick start writes `my-cur.csv`, so it no longer overwrites the shipped `examples/cur.csv`.

### Fixed

- A run is rejected when the compressed gzip bytes of every member of every input file exceed 1073741824 (`gzip input is larger than 1073741824 bytes`, exit 2, no report). Measured on the release binary: one CUR member, then stored members of 1048573 bytes (209711 empty stored blocks) and one shorter stored member, at 1073741824 compressed bytes exited 0 in 3.10 s (peak 2796 KB); 1073741825 bytes exited 2 with that message and no report in 3.12 s (peak 2848 KB). The same shape at 268435456 and 536870912 bytes exited 0 in 0.77 s and 1.52 s, so the measured time grew by 1.97 times and by 4.03 times.
- A gzip member no longer borrows unpacked bytes from an earlier member. A file is rejected when it holds more than 40000 members, or when one member's compressed bytes exceed the bytes that member unpacks by more than 1048576 (`gzip overhead is larger than 1048576 bytes`, exit 2, no report). Measured on the release binary: 52429 empty members (1048580 bytes) exited 2 with that message and no report; two members whose compressed bytes exceed the bytes they unpack by 650023 each (1300226 bytes) exited 0.
- A usage error with an argument that holds an escape sequence and a line break (`--top $'1\x1b[31m\nFAKE: line'`, or an unknown option `$'--x\x1b[31m\nFAKE: line'`), or a byte that is not valid UTF-8 and a line break (`$'--x\xff\nFAKE: line'`), no longer puts a fake line on standard error: the error is shown on one line with `\u{1b}` and `\u{a}`. The message is made again from the escaped arguments, because clap drops escape sequences from the text it echoes. The message "invalid UTF-8 was detected in one or more arguments" does not echo an argument and keeps its own line breaks. A lone line break as an argument no longer turns every line break of the message into `\u{a}`.
- A backslash in a tag value is shown as two backslashes in the tag line (text and `--json`), so the values ` x` and `\u{20}x`, and `amount=1` and `amount\=1`, are four different lines. Before, a tag value `\u{20}x` and ` x` gave the same text.
- Sums are exact. A monthly total or delta that does not fit a 28-digit decimal fails with `amount total does not fit` and exit 2; before, a rounded amount could be printed, and the result could depend on the order of the keys.
- An amount with more than 28 decimals is rejected as `invalid amount` (exit 2). It was read silently as `0.00`.
- White space is no longer trimmed in the Unicode sense. A header cell with spaces around a column name is a missing column, and an amount, a billing time, a product code, a usage type, or resource_tags with a no-break space, a form feed, or a vertical tab is rejected. ASCII space, tab, CR and LF around those fields are still removed.
- A tag key with `=` or `\` is escaped (`a\=b`, `a\\b`) in the tag lines, so `a=b`/`c` and `a`/`b=c` are two lines.
- JSON surrogate pairs in a tag cell (`\ud83d\ude00`) are read as one character.
- In the text report, a space, a control, or an invisible or direction-changing character in a key is shown as `\u{..}`, and the `=` of `amount=`, `previous=`, and `delta=` is escaped, so a key cannot pass for a field. A backslash in a service or usage type key is shown as two backslashes, so `A B` and a literal `A\u{20}B` are two different lines. The same `\u{..}` escaping applies to file names in error messages.
- A file name that is not valid UTF-8 can be read when it ends in `.csv` (Linux); in an error it is shown as bytes (`bad\xff.txt`). It was rejected before.
- The month `0000-01` is valid and the report shows `previous: omitted`. It printed a previous month before year 0.
- The latest month is the latest month in the files, also when all its amounts are blank. It was the month before.
- The row number of an error counts from the start of each file (`file 2: row 1: ...`). The row number of `wrong column count` counts records and not lines, so a quoted cell with line breaks no longer shifts it.
- A closed or full standard output (`>&-`, `/dev/full`, a closed pipe) exits 2 with `could not write output`, also for `--help` and `--version`. It exited 0 before. A valid run to `/dev/null` still exits 0. A closed standard output is recognized on Linux only.
- One `unsafe` block was added, in `src/stdout_probe.rs`, for that check on Linux. The library keeps `#![forbid(unsafe_code)]`.

### Documented

- Memory use (about 24 times the input with many unique keys), the case-sensitive file name check, a `.gz` name without gzip magic (`not gzip`), a file cut off while it is read, gzip errors (`could not read file`), and the system requirements (GLIBC_2.34 or newer for the release binary, a musl build works) are described in the README, the man page, and `QUALITY.md`. Also described: `--json` prints keys as they are, a minimum version of macOS or Windows is not measured, and the test total of 136 is the count on Linux. Also described: the input rules (UTF-8, the billing time form, no standard input, a tolerant CSV reader), a single large cell, and a standard output that is open for reading only.

### Added

- Tests `claim_c14_gzip_magic_under_a_csv_name_is_read_as_gzip` and `claim_c17_amount_limit_is_40_characters` in `tests/cli.rs`. Both match the README; no deviation found.
- Tests `usage_error_with_escape_sequence_and_newline_stays_on_one_line`, `usage_error_with_a_lone_newline_argument_keeps_clap_line_breaks`, `tag_values_that_differ_only_by_a_backslash_stay_apart_in_text`, and `stdio_null_stdout_exits_0_for_a_report_help_and_version` in `tests/cli.rs`. `tests/docs_clean.rs` checks the documents and the man page for local absolute paths, prefixes of local wrappers, names of local work files, and names of AI systems.
- Tests `usage_error_with_a_non_utf8_argument_and_newline_stays_on_one_line`, `billing_timestamp_accepts_the_utc_form_and_rejects_the_others`, `bom_and_crlf_are_accepted_and_a_quote_followed_by_text_is_joined`, `a_file_name_with_an_upper_case_suffix_is_rejected`, `standard_input_is_not_read`, `a_read_only_standard_output_exits_0_and_writes_nothing`, `a_csv_cut_inside_the_last_amount_is_summed_and_a_cut_after_the_point_is_an_error`, and `damaged_gzip_files_fail_with_could_not_read_file_and_no_report` in `tests/cli.rs`, and `source_workflows_and_scripts_hold_no_internal_references` in `tests/docs_clean.rs`, which also checks the source, the workflows, and the scripts.
- `scripts/check-glibc.sh` and a CI job that fails when the release binary needs a glibc symbol newer than GLIBC_2.34. The script is not part of the package.
- `SECURITY.md`.
- `.github/dependabot.yml`. The CI jobs keep no credentials after checkout, have a time limit on the test jobs, and run every test after a failure (`--no-fail-fast`).
- `man/cur-top.1`, and `tests/handbook_help.rs`, which checks that the man page holds the `--help` text and the same long options, also in its OPTIONS section.

This version is not published and has no git tag.

## [0.1.0] - 2026-11-09

### Added

- `cur-top` reads a local AWS CUR 2.0 CSV or CSV gzip and lists top costs by service, usage type, and resource tag.
- The delta is the latest billing month minus the previous calendar month.
- Amounts are the cost column already in the file. The tool does not multiply a rate by usage.

This version is not published and has no git tag.
