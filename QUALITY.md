# Quality

This commit changes documentation only; the code commit is 1fc2cb1. Of the checks below, only `cargo fmt --check` and `cargo test --locked --offline -j 2 -- --test-threads 2` ran on it (2026-11-24); every other check was run on the earlier code commit 83ad5d9 (2026-11-20) and is not run again: not run on 1fc2cb1.

Suite: 168 passed, 1 ignored, 0 failed (on 1fc2cb1; Linux, `cargo test`, debug; eleven of the tests run on Linux only).

Record of 2026-11-20. Toolchains: rustc 1.99.0 and cargo 1.99.0 (stable), and cargo 1.85.1 (the minimum supported version, `+1.85`). Host: Linux, AMD Ryzen 9 5950X. Tested commit: `83ad5d99dde1847b5c25929cad7d86d52518b20e`. Every check below, except the two named at the top, ran on that commit; the commits after it change documents only (`*.md`, `man/`, `examples/` files that are not code). No tag, no publish. Every command ran under a memory limit of 16 GB, with `-j 2` and `--test-threads 2`.

## Commands

On 1fc2cb1 only the first row of this table and the `cargo test` row (debug) were run; the other rows are results on 83ad5d9 and are "not run" on 1fc2cb1. The change in 1fc2cb1 is that the hard-link part of `an_input_file_as_output_is_refused_and_not_overwritten` runs on Unix only.

| Command | Exit |
| --- | ---: |
| `cargo fmt --check` | 0 |
| `cargo clippy --locked --offline -j 2 --all-targets -- -D warnings` | 0 |
| `cargo +1.85 clippy --locked --offline -j 2 --all-targets -- -D warnings` | 0 |
| `cargo check --locked --offline -j 2 --tests --target x86_64-pc-windows-gnu` | 0 |
| `cargo check --locked --offline -j 2 --tests --target x86_64-apple-darwin` | 0 |
| `cargo clippy --locked --offline -j 2 --all-targets --target x86_64-pc-windows-gnu -- -D warnings` | 0 |
| `cargo clippy --locked --offline -j 2 --all-targets --target x86_64-apple-darwin -- -D warnings` | 0 |
| `cargo test --locked --offline -j 2 -- --test-threads 2` | 0 |
| `cargo test --locked --offline -j 2 --release -- --test-threads 2` | 0 |
| `cargo +1.85 test --locked --offline -j 2 -- --test-threads 2` | 0 |
| `cargo package --locked --offline --allow-dirty --no-verify --list` | 0 |
| `cargo build --release --locked --offline -j 2` | 0 |
| `cargo package --offline --allow-dirty`, then `cargo test --offline -j 2 -- --test-threads 2` in the unpacked `.crate` (an empty directory) | 0 (168 passed, 0 failed, 1 ignored) |
| `cargo about generate --offline --locked about.hbs -o <file>` | 0 (4 license groups) |
| `groff -ww -z -man -Tutf8 man/cur-top.1` | 0, no message |
| `bash scripts/check-glibc.sh target/release/cur-top 2.34` | 0 (highest symbol GLIBC_2.34) |
| `bash scripts/check-glibc.sh target/release/cur-top 2.33` (must fail) | 1 |
| `cargo test --release --locked --offline -j 2 --test quality -- --ignored --exact quality_benchmark_100_mb` | 0 |
| `cargo deny --locked check licenses` | 0 |
| `cargo audit --no-fetch` | 0 |

The `cargo check` and `cargo clippy` runs for the Windows and macOS targets type-check and lint the code. They do not run the tests.

`cargo package --list` has 47 paths (counted with the files of this record in place): `Cargo.toml`, `Cargo.toml.orig`, `Cargo.lock`, `.cargo_vcs_info.json`, `.gitignore`, `build.rs`, `about.hbs`, `about.toml`, `CHANGELOG.md`, `CONTRIBUTING.md`, both licenses, `QUALITY.md`, `QUALITY.nl.md`, `README.md`, `README.nl.md`, `SECURITY.md`, `deny.toml`, the files of `examples/`, `man/cur-top.1`, `src/*.rs`, and `tests/*.rs`. It has no `.github/`, no `scripts/`, and no file that is only for local work.

## Test result per target

Debug (`cargo test --locked --offline -j 2 -- --test-threads 2`) and release (the same with `--release`) give the same counts. `cargo +1.85 test` in debug gives them too.

| Target | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| unittests `src/lib.rs` | 70 | 0 | 0 |
| unittests `src/main.rs` | 0 | 0 | 0 |
| `tests/cli.rs` | 54 | 0 | 0 |
| `tests/docs_clean.rs` | 4 | 0 | 0 |
| `tests/handbook_help.rs` | 2 | 0 | 0 |
| `tests/markdown.rs` | 5 | 0 | 0 |
| `tests/quality.rs` | 0 | 0 | 1 |
| `tests/output.rs` | 11 | 0 | 0 |
| `tests/readme.rs` | 12 | 0 | 0 |
| `tests/schema.rs` | 2 | 0 | 0 |
| `tests/sigint.rs` | 1 | 0 | 0 |
| `tests/verbosity.rs` | 6 | 0 | 0 |
| `tests/version.rs` | 1 | 0 | 0 |
| doc-tests | 0 | 0 | 0 |
| total | 168 | 0 | 1 |

The 168 is the count on Linux. Ten tests in `tests/cli.rs` and one in `tests/verbosity.rs` (`a_full_stdout_is_exit_2_without_a_context_line`) run on Linux only. The ten in `tests/cli.rs` are `closed_full_and_null_stdout`, `help_and_version_to_closed_or_full_stdout_exit_2`, `stdio_null_stdout_exits_0_for_a_report_help_and_version`, `filename_with_esc_byte_exits_2_and_stderr_escapes_it`, three tests with a file name that is not UTF-8, `usage_error_with_a_non_utf8_argument_and_newline_stays_on_one_line`, `standard_input_is_not_read`, and `a_read_only_standard_output_exits_0_and_writes_nothing`. Seven more are Unix only (a symlink in `tests/cli.rs` and in `src/run.rs`, a device file, and `tests/sigint.rs`; `a_symlink_as_output_is_replaced_not_followed`, `a_fifo_as_output_is_refused_and_stays_a_fifo`, and `a_unix_socket_as_output_is_refused_and_stays_a_socket` too), so fewer tests run on macOS and Windows. `documented_test_total_matches_real_count` counts `#[test]` lines in the sources; it does not run them.

The ignored test is `quality_benchmark_100_mb`.

## Not run

- `quality_benchmark_100_mb` (`#[ignore]`) is not part of the suite; it ran separately (see "100 MiB"). Command: `cargo test --release --locked --test quality -- --ignored --exact quality_benchmark_100_mb`.
- A fuzz campaign.
- A mutation run.
- GitHub Actions. The workflows were not started; they have been red from billing, which is separate from the numbers above.
- Windows and macOS. Only the type check and the lint above ran. Tests that need a symlink or a device file are Unix only, and tests that need a closed or full standard output, a file name or an argument with bytes that are not UTF-8, a file name with a control character, or `/dev/stdin` are Linux only.
- The release workflow (`.github/workflows/release.yml`). It starts on a `v*` tag; the crate has no tag and is not published, so it has not run.
- The glibc check in CI (`glibc` job). The script ran on this host; the job itself did not run.
- `git commit`, `git push`, `git tag`, and `cargo publish`.

## 100 MiB

`tests/quality.rs` runs `quality_benchmark_100_mb`. It builds a CUR 2.0 CSV of at least 100 MiB (`100 * 1024 * 1024` bytes in the source) and sums it once.

Measured on 2026-11-20 (Linux, AMD Ryzen 9 5950X, rustc 1.99.0), release profile already built, on the tested commit: `quality_benchmark_100_mb` ok, 1 passed, 0 failed, finished in 1.05s, exit 0. This file has one key, so it says nothing about memory with many keys; see "Open points".

## Standard output by hand

Measured on 2026-11-19 with the debug and the release binary, for `examples/cur.csv`, `--help`, and `--version`. All three behave the same, in both profiles.

| Case | Exit | Message |
| --- | ---: | --- |
| `>&-` | 2 | `cur-top: could not write output` |
| `>/dev/full` | 2 | `cur-top: could not write output` |
| `>/dev/null` | 0 | none |
| `1<>/dev/null` | 0 | none |
| Python `subprocess.run(..., stdout=subprocess.DEVNULL)` | 0 | none |
| `\| true` (closed pipe) | 2 | `cur-top: could not write output`, no panic |
| `1<file` (a file opened for reading only), release binary, report | 0 | none, nothing written (see "Open points") |
| `/proc` replaced by an empty file system, output to a file | 0 | none (615, 587, and 14 bytes written) |
| `/proc` replaced by an empty file system, `>&-` | 2 | `cur-top: could not write output` |
| `/proc` replaced by an empty file system, `>/dev/null` | 0 | none |

## Known limits

- This page records checks. It is not a guarantee and it is not a fuzz report.
- The 100 MiB timing is from one run on 2026-11-20 (see above).
- Only a CUR 2.0 CSV or CSV gzip is read. Parquet and zip are rejected, and Athena is not used.
- The product code and usage type limit is 1024 bytes, not characters (`src/summary.rs`).
- The amount limit of 40 characters (`src/money.rs`) is tested at 40 and 41 characters by `claim_c17_amount_limit_is_40_characters`.
- A `.csv`-named file that starts with the gzip magic `1f 8b` is read as gzip (`src/run.rs`); `claim_c14_gzip_magic_under_a_csv_name_is_read_as_gzip` builds that case.
- "No network call" is checked only through the dependency list (`claim_c16_manifest_has_no_sdk_athena_or_parquet`). No test watches the system calls.
- Windows and macOS: see "Not run". A minimum version of macOS or Windows is not measured.
- The glibc claim (GLIBC_2.34) was read with `objdump -T` on one build and checked with `scripts/check-glibc.sh`. The `glibc` job of the CI builds on `ubuntu-22.04` and runs the same script (see "Not run").

## Open points

Known limits that are not fixed. Each was measured with the release binary of 2026-11-19 on synthetic files.

- Memory with many unique keys. The state is a map of keys, so memory follows the number of distinct keys, not only the input size. Measured peak resident memory (release binary): `uniq300k.csv` (13.3 MB, 300000 rows, each with its own service and usage type) 314 MiB, 1.4 s; `tags20k.csv` (13.7 MB, 20000 rows, 25 unique tags each) 303 MiB, 1.6 s; `uniq2m.csv` (91.8 MB, 2000000 unique keys) 2.1 GiB, 14.2 s. That is about 24 times the input for unique service and usage type keys and about 23 times for unique tags. The limit of 536870912 bytes is on the input, not on this state. Extrapolation, not run: a file at that limit with only unique keys would need about 12 GiB. The 100 MiB benchmark has one key and does not show this.
- Closed standard output on Linux only. `src/stdout_probe.rs` records whether file descriptor 1 was closed at start (`>&-`). It is compiled on Linux only; on other systems a closed standard output is not recognized. A write that fails (`/dev/full`, a closed pipe) exits 2 on every system. That module and `src/interrupt.rs` (the SIGINT handler) hold the only `unsafe` code of the project (`#[allow(unsafe_code)]` on both modules in `src/main.rs`); the library keeps `#![forbid(unsafe_code)]`. Not run on macOS or Windows.
- System requirements. The release binary built with glibc needs GLIBC_2.34 or newer (highest symbol, read with `objdump -T`). A build for `x86_64-unknown-linux-musl` works and passes the closed, full, and null standard output checks.
- File names. The suffix check is case sensitive: `UPPER.CSV` and `export.CSV` fail with `not a csv or csv.gz` and exit 2. A `.gz` name without the gzip magic (`notgz.csv.gz` holding the text `hello`) fails with `not gzip` and exit 2. Both are the documented behavior.
- A file cut off while it is read. With the amount as the last column and a cut inside the last field of the last row, without a final newline, the sum is wrong without an error. Measured: two rows `1.50` and `2.50` give `amount=4.00`; the same file cut to `...,2` gives `amount=3.50` and exit 0; cut to `...,2.` gives `invalid amount` and exit 2. A CSV has no checksum. The same data as gzip, cut off, gives `could not read file` and exit 2.
- Gzip errors are not told apart. A cut-off gzip file, a wrong CRC, and bytes after the last member all give `file 1: could not read file` and exit 2, with no report on standard output. Measured on the same two-row file.
- Gzip member overhead. Empty gzip members unpack to 0 bytes, so the cap of 536870912 unpacked bytes does not stop them. A gzip file is rejected when one member's compressed bytes exceed the bytes that member unpacks by more than 1048576, or when the file holds more than 40000 members (`gzip overhead is larger than 1048576 bytes`, exit 2, no report). The compressed bytes of every member of every input file may not exceed 1073741824 (`gzip input is larger than 1073741824 bytes`, exit 2, no report). Unpacked bytes from an earlier member do not raise the per-member overhead or the member cap. The same checks are used for a `.csv.gz` name and for a `.csv` name that starts with the gzip magic. Standard input is not read, so it is not a gzip path. Measured on the release binary on 2026-11-19, one member of 20971497 unpacked bytes (61192 compressed) followed by empty members of 20 bytes. 500000 members (10061192 bytes), 1000000 members (20061192 bytes), and 2000000 members (40061192 bytes) each exited 2 in 0.35 s, peak 2636 KB, 2688 KB, and 2896 KB. For these empty members of 20 bytes, the measured time stayed 0.35 s when the input grew by 4× and by 4× again. One CUR member, then stored members of 1048573 bytes (209711 empty stored blocks) and one shorter stored member, exited 0 at 268435456 bytes in 0.77 s (peak 2904 KB), at 536870912 bytes in 1.52 s (peak 2624 KB), and at 1073741824 bytes in 3.10 s (peak 2796 KB). The measured time grew by 1.97 times and by 4.03 times. A file of 1073741825 bytes exited 2 with `gzip input is larger than 1073741824 bytes` and no report in 3.12 s (peak 2848 KB). Before the per-member and member-count limits, on those same three files of 20-byte members, the release binary exited 0 in 1.45 s, 0 in 2.69 s, and 2 in 2.92 s.
- A single cell is held whole. A file of 419430546 bytes with one product code of 400 MiB was refused with `invalid line_item_product_code` and exit 2 after 1.0 s and a peak of 913 MiB. The cell limit is the file limit of 536870912 bytes.
- A tolerant CSV reader. A quoted cell followed by text, as in `"A"x`, is read as `Ax` (exit 0). The product code `Ax` is then reported; no amount changes.
- No standard input. `-` fails with `No such file or directory` and `/dev/stdin` fails with `not a file` (pipe or terminal) or `not a csv or csv.gz` (regular file), both exit 2.
- A standard output opened for reading only. `cur-top examples/cur.csv 1<ro.txt` exits 0, writes nothing, and leaves `ro.txt` unchanged: the failed write is not seen, as with `1</dev/null`. Only a standard output that was closed at start is recognized. Measured on Linux; a decision, not fixed.
- The release workflow (`.github/workflows/release.yml`) builds the release binaries, adds the license page, and publishes archives when a `v*` tag is pushed. Whether it has run: see "Not run".
- `--json` prints keys as they are. A key with a direction-changing or invisible character (no control character: those are rejected when read) shows raw in `--json`; the text report escapes it. This is a decision: JSON must hold the key as written.
- The tag line (text and `--json`) shows `\` as `\\` in a key and in a value, and `=` in a key as `\=`, so two different tags never give the same line. A value with `=` is not escaped, because the first unescaped `=` ends the key.

## Claim → test

Every claim of `README.md` and `QUALITY.md`, with the test that proves it. Test names are from the 2026-11-20 run. "Gap" means no test proves it, or only part of it.

| # | Claim | Test | Status |
| --- | --- | --- | --- |
| C1 | Adds the cost column as written; rate × usage is not computed | `claim_c1_unblended_is_not_rate_times_usage` | tested |
| C2 | CUR 2.0 column names, required and optional columns | `claim_c2_official_column_names_and_dictionary_date`, `missing_column_names_the_official_column`, `blended_column_is_summed_when_selected`, `net_unblended_column_is_summed_when_selected` | tested |
| C3 | `--amount` accepts only the three cost columns | `claim_c3_amount_columns_are_the_three_cost_fields`, `invalid_amount_column_hides_the_name` | tested |
| C4 | Month is `YYYY-MM`, current is the latest, previous is the calendar month before | `claim_c4_latest_month_and_calendar_previous`, `claim_c4_latest_month_with_only_blank_amounts_stays_current`, `previous_month_rolls_the_year` | tested |
| C5 | Delta is current minus previous | `claim_c5_delta_is_current_minus_previous` | tested |
| C6 | A blank amount is skipped; a bad amount does not show the cell | `claim_c6_blank_amount_is_skipped_without_the_cell`, `parquet_and_bad_amount_hide_the_payload` (cli) | tested |
| C7 | Other months stay out of the totals | `claim_c7_other_months_stay_out_of_the_totals` | tested |
| C8 | No previous month: `0.00` and `previous_month_present` no | `claim_c8_absent_previous_month_is_zero` | tested |
| C9 | A negative amount is summed as written | `claim_c9_negative_credit_lowers_the_total`, `amount_accepts_a_negative_credit` | tested |
| C10 | The same row in two files counts twice | `claim_c10_the_same_row_in_two_files_counts_twice` | tested |
| C11 | Each tag gets the full amount; key and value escaping; tag limits | `claim_c11_each_tag_receives_the_full_line_amount`, `tag_values_that_differ_only_by_a_backslash_stay_apart_in_text` (cli), `keys_that_differ_only_by_a_backslash_stay_two_lines_in_text` (cli), `tags_whose_joined_text_is_equal_stay_two_lines`, `equal_joined_tags_stay_two_lines` (cli), `tag_limits_accept_the_edge_and_reject_one_more`, `too_many_tags`, `duplicate_key_rejected`, `control_char_rejected` | tested |
| C12 | Empty `resource_tags` still counts for service and usage type | `claim_c12_empty_tags_still_count_for_the_service` | tested |
| C13 | Top `--top` keys, 1 to 10000, zero keys skipped, totals cover all rows | `claim_c13_top_is_limited_and_totals_cover_both_months`, `top_accepts_10000_and_rejects_10001`, `tie_breaks_by_key` | tested |
| C14 | gzip by the magic `1f 8b`, also under a `.csv` name; a `.gz` name without the magic is `not gzip`; Parquet and zip rejected | `claim_c14_gzip_is_read_and_parquet_and_zip_are_rejected`, `two_gzip_members_are_both_counted`, `claim_c14_gzip_magic_under_a_csv_name_is_read_as_gzip` (cli) | tested |
| C15 | `line_item_usage_account_id` and extra columns are ignored | `claim_c15_account_id_is_not_reported`, `account_id_column_is_absent_from_stdout` (cli) | tested |
| C16 | No network call and no AWS SDK | `claim_c16_manifest_has_no_sdk_athena_or_parquet` | gap: dependency list only, no network test |
| C17 | Amount limits, invalid cells exit 2 without the cell | `claim_c17_invalid_cells_hide_the_value`, `amount_with_more_than_28_decimals_exits_2_and_28_still_works` (cli), `amount_rejects_exponent_plus_and_blank`, `product_code_and_usage_type_stop_at_1024_bytes`, `usage_errors_exit_2` (cli), `claim_c17_amount_limit_is_40_characters` (cli) | tested |
| C18 | Decimal text, two fraction digits, no exponent | `claim_c18_text_has_no_exponent_and_two_digits`, `format_amount_keeps_scale_and_two_digits` | tested |
| C19 | `--quiet` and `--verbose` change only standard error: `--verbose` adds one `context:` line on success, `--quiet` none, both together exit 2 | `verbose_adds_one_context_line_on_stderr_and_keeps_stdout`, `quiet_prints_nothing_extra_and_keeps_stdout`, `an_error_line_is_the_same_with_quiet_and_verbose_and_has_no_context`, `quiet_with_verbose_is_a_usage_error_exit_2`, `a_full_stdout_is_exit_2_without_a_context_line`, `help_lists_quiet_and_verbose` (`tests/verbosity.rs`) | tested |
| L1 | Install from a local path | `install_uses_a_local_path` (`tests/readme.rs`) | tested |
| L2 | The quick start file is `examples/cur.csv` | `quickstart_heredoc_matches_the_sample_file` (`tests/readme.rs`) | tested |
| L3 | The text and JSON examples are what the binary prints | `console_examples_match_the_binary` (`tests/readme.rs`), `sample_text_matches_the_locked_report` | tested |
| L4 | A symlink is followed, a directory is rejected | `directory_is_rejected_and_symlink_is_followed`, `symlink_is_followed_and_directory_is_rejected` (cli) | tested |
| L5 | A file over 536870912 uncompressed bytes is rejected | `unpacked_size_limit_accepts_the_edge_and_rejects_one_byte_more` (cli) | tested |
| L6 | The tables are not a forecast | none | gap |
| L7 | The quality test covers a CSV of at least 100 MiB | `quality_benchmark_100_mb` | test exists, ignored, not run in the suite |
| L8 | Not published, no git tag | `licenses_changelog_and_cargo_metadata` (the changelog says so only) | gap: the repository state is not tested |
| L9 | The documented test total (README, this file) | `documented_test_total_matches_real_count` (counts `#[test]` lines, does not run them) | tested |
| L10 | Exit code 0 ok, 2 input or usage error | `usage_errors_exit_2`, `missing_file_exits_2`, `help_and_version_exit_0` (cli) | tested |
| L11 | The public documents, the source, the workflows, and the scripts hold no internal references | `public_documents_hold_no_internal_references`, `source_workflows_and_scripts_hold_no_internal_references`, `the_patterns_catch_each_kind_of_reference` (`tests/docs_clean.rs`) | tested |
| L12 | A usage error shows a control or direction character of the arguments as `\u{..}` on one line, also after an escape sequence and in an argument that is not UTF-8 | `usage_error_escapes_raw_direction_newline_and_cr`, `usage_error_with_escape_sequence_and_newline_stays_on_one_line`, `usage_error_with_a_lone_newline_argument_keeps_clap_line_breaks` (cli), `usage_error_with_a_non_utf8_argument_and_newline_stays_on_one_line` (cli, Linux) | tested |
| L13 | A closed, full, or broken standard output exits 2 with `could not write output`; `/dev/null` and a null standard output exit 0 | `closed_full_and_null_stdout`, `help_and_version_to_closed_or_full_stdout_exit_2`, `stdio_null_stdout_exits_0_for_a_report_help_and_version` (cli) | tested in part: Linux only, and a closed pipe has no test (measured by hand, see "Standard output by hand") |
| L14 | Input rules: UTF-8, BOM and CRLF, the billing time form, ASCII space, tab, CR and LF, a tolerant quote, a lower case file suffix | `billing_timestamp_accepts_the_utc_form_and_rejects_the_others`, `bom_and_crlf_are_accepted_and_a_quote_followed_by_text_is_joined`, `a_file_name_with_an_upper_case_suffix_is_rejected`, `utf8_bom_is_accepted` | tested |
| L15 | Standard input is not read | `standard_input_is_not_read` (cli, Linux) | tested |
| L16 | A standard output opened for reading only exits 0 and writes nothing | `a_read_only_standard_output_exits_0_and_writes_nothing` (cli, Linux) | tested |
| L17 | A CSV cut inside the last amount can be summed; a cut after the point is an error | `a_csv_cut_inside_the_last_amount_is_summed_and_a_cut_after_the_point_is_an_error` (cli) | tested |
| L18 | A damaged gzip file fails with `could not read file` and no report | `damaged_gzip_files_fail_with_could_not_read_file_and_no_report` (cli) | tested |
| L19 | The release binary needs GLIBC_2.34 or newer | none in the suite; `scripts/check-glibc.sh` in the CI job `glibc` | gap: the script ran here, the job has not run |
| L20 | Memory is about 24 times the input with many unique keys | none | gap: measured by hand (three files, see "Open points") |
| L21 | A single cell is held in memory whole | none | gap: measured by hand (one file) |
| L22 | A gzip file is rejected at more than 40000 members, when one member's compressed bytes exceed that member's unpacked bytes by more than 1048576, or when the compressed bytes of every member of every input file exceed 1073741824 (`gzip input is larger than 1073741824 bytes`, exit 2, no report). For empty members of 20 bytes, 500000, 1000000, and 2000000 members after one member of 20971497 unpacked bytes each exited 2 in 0.35 s (peak 2636 KB, 2688 KB, and 2896 KB); that 0.35 s stayed when the input grew by 4× and by 4× again. Stored members of 1048573 bytes (209711 empty stored blocks) after one CUR member, plus one shorter stored member, exited 0 at 268435456, 536870912, and 1073741824 bytes in 0.77 s, 1.52 s, and 3.10 s (peak 2904 KB, 2624 KB, and 2796 KB); the measured time grew by 1.97 times and by 4.03 times. 1073741825 bytes exited 2 in 3.12 s (peak 2848 KB) with no report | `many_empty_gzip_members_are_rejected_on_every_gzip_path`, `gzip_overhead_accepts_the_budget_and_rejects_one_more_member`, `a_large_prefix_plus_too_many_empty_gzip_members_is_rejected`, `one_member_cannot_use_an_earlier_members_unpacked_bytes`, `many_gzip_members_that_hold_rows_are_still_read`, `gzip_compressed_total_is_rejected_on_every_gzip_path`, `a_large_ordinary_gzip_under_the_compressed_cap_still_runs` (cli) | tested |
| L23 | `--md` prints the same amounts as the text and JSON reports; a pipe, hidden characters, and Markdown and HTML characters in a key are escaped; `--md` with `--json` is a usage error | `markdown_amounts_equal_text_and_json`, `a_pipe_and_a_line_break_in_a_key_stay_in_one_cell`, `markdown_and_html_in_a_key_are_escaped`, `md_with_json_is_a_usage_error`, `markdown_says_the_amounts_are_read_not_estimated` (`tests/markdown.rs`) | tested |
| L24 | SIGINT while the files are read exits 130, with no report, an empty standard output and one line on standard error (Unix) | `sigint_while_reading_exits_130_with_no_report_and_no_panic` (`tests/sigint.rs`, a real SIGINT) | tested |
| L25 | `--output FILE` writes the whole report through a temporary file and a rename; an existing file is replaced, a symbolic link is not followed, a missing directory is not created, an input file is refused, a write error is exit 2 with an escaped path | the eleven tests of `tests/output.rs`; an existing pipe or socket is refused with `not a regular file` | tested |
| L26 | Every JSON report starts with `schema_version` 1; a failing run prints no JSON | `every_json_path_has_schema_version_1_first`, `a_failing_json_run_prints_no_json_only_one_error_line` (`tests/schema.rs`) | tested |
| L27 | `--version` prints the crate version; a debug build adds the git commit when git gives one | `version_prints_the_crate_version_and_in_debug_the_commit` (`tests/version.rs`) | tested |
| L28 | `about.toml` accepts the same licenses as `deny.toml`, and `about.hbs` builds the third-party license page of the release archive | `about_toml_accepts_the_same_licenses_as_deny_toml` (`tests/docs_clean.rs`); the page itself was generated by hand with `cargo about` (4 license groups), not in CI | tested in part |
| L29 | `README.nl.md` has the same sections, console examples, claims, and numbers as `README.md` | `the_dutch_readme_has_the_same_sections_examples_claims_and_numbers`, `readme_sections_follow_the_documentation_standard`, `the_options_section_names_every_long_option_of_help` (`tests/readme.rs`) | tested |
| L30 | The files `examples/cur.expected.*` are what the binary prints for `examples/cur.csv` | `the_example_files_match_the_binary` (`tests/readme.rs`) | tested |
| Q1 | The 100 MiB timing of 2026-11-20 | `quality_benchmark_100_mb` | gap: ignored, run on purpose, not part of the suite |

50 claims: 40 tested, 2 tested in part (L13, where a closed pipe has no test, and L28, whose license page is generated by hand), 1 with an ignored test only (L7), 7 without a test that proves them (C16, which has only the dependency list, L6, L8, L19, L20, L21, Q1). `quality_claim_summary_matches_the_table_rows` counts the rows.

The man page is checked by `help_long_options_match_the_man_page_and_unknown_option_is_not_a_panic` (`tests/handbook_help.rs`): it holds the `--help` text verbatim and the same long options.
