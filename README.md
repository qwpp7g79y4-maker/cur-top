# cur-top

Offline top costs from an AWS CUR 2.0 CSV. The tool sums amounts that are already in the file.

## Why

`cur-top` reads a Cost and Usage Report 2.0 export and lists the largest services, usage types, and resource tags for the latest billing month, with the delta against the previous month.

Reads only the local `.csv` or `.csv.gz` files you name. It uses no network, no AWS SDK, no account, and no token, and it needs no paid service. It writes nothing unless you pass `--output`. It sums the cost column that is already in the file; it does not multiply a rate by a usage amount, and it does not estimate a price.

## Installation

Rust 1.85 or newer.

```sh
cargo install --locked --path .
```

The crate is not published and has no git tag. Release archives for Linux, Windows, and macOS are built by `.github/workflows/release.yml` when a `v*` tag is pushed; no tag exists yet. Supported systems are listed under Limitations (System requirements).

## Quick start

```sh
cat > my-cur.csv <<'CSV'
bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,line_item_unblended_rate,line_item_usage_amount,resource_tags
2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,3.50,0.0104,336,"{""user_team"":""platform""}"
2026-11-01T00:00:00Z,AmazonS3,TimedStorage-ByteHrs,1.00,0.023,43,"{""user_team"":""platform""}"
2026-10-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,0.0104,96,"{""user_team"":""platform""}"
CSV
```

The repository ships the same file as `examples/cur.csv`. The commands below read that copy, so nothing in the repository is overwritten.

```console
$ cur-top examples/cur.csv
cur-top
dictionary: https://docs.aws.amazon.com/cur/latest/userguide/table-dictionary-cur2.html
read: 2026-11-09
amount: line_item_unblended_cost
month: 2026-11
previous: 2026-10
previous_month_present: yes
rows: 3
rows_without_amount: 0
rows_without_tags: 0
rows_outside_months: 0

service
AmazonEC2 amount=3.50 previous=1.00 delta=2.50
AmazonS3 amount=1.00 previous=0.00 delta=1.00

usage_type
BoxUsage:t3.micro amount=3.50 previous=1.00 delta=2.50
TimedStorage-ByteHrs amount=1.00 previous=0.00 delta=1.00

tag
user_team=platform amount=4.50 previous=1.00 delta=3.50

totals
amount=4.50 previous=1.00 delta=3.50
```

## Aan de slag in 5 minuten

Korte route in het Nederlands; de volledige Nederlandse tekst staat in [README.nl.md](README.nl.md).

1. Installeer met `cargo install --locked --path .` (Rust 1.85 of nieuwer).
2. Neem een CUR 2.0-bestand (`.csv` of `.csv.gz`), of gebruik `examples/cur.csv`.
3. Draai `cur-top examples/cur.csv` voor de tekstuitvoer, of `--json` of `--md`.
4. Lees de bedragen per dienst, gebruiksoort en tag: huidige maand, vorige maand en het verschil. Alle bedragen komen uit het bestand zelf; er is niets geschat.
5. Een fout geeft exit 2 en één regel op standaardfout; lees "Exit codes" voor de rest.

## Options

```text
cur-top [OPTIONS] <FILE>...
```

- `<FILE>...`: one or more AWS CUR 2.0 `.csv` or `.csv.gz` files. At least one is required. There is no standard input.
- `--amount <COLUMN>`: the cost column to sum: `line_item_unblended_cost` (default), `line_item_blended_cost`, or `line_item_net_unblended_cost`.
- `--top <N>`: how many keys to list for each of service, usage type, and tag. 1 to 10000, default 10. Totals include every row of the two months, not only the listed keys.
- `--json`: print JSON instead of the text report.
- `--md`: print Markdown tables instead of the text report. Not together with `--json`.
- `--output <FILE>`: write the report to a file instead of standard output.
- `--quiet`, `--verbose`: only standard error changes. They cannot be combined.
- `--help` (`-h`) and `--version` (`-V`). A debug build adds the git commit to the version.

`--quiet` and `--verbose` change only standard error: a successful run prints nothing there, and `--verbose` adds one `context:` line (rows read and keys listed per table). The report on stdout and the exit codes stay; both together are a usage error (exit 2).

`--output FILE` writes the report to a file instead of standard output (which then stays empty). The report goes to a new file in the same directory and then replaces `FILE` by a rename, so `FILE` holds the whole report or is not changed. An existing file is replaced; a symbolic link at `FILE` is replaced as a link and never written through; a path that exists and is a pipe, a socket, or a device is refused with `not a regular file` and left as it was; a missing directory is not created; a `FILE` that is also an input file is refused (two hard links to one file are seen as the same file on Unix only, not on Windows). A write error (a missing directory, a directory as `FILE`, a full disk, no permission) exits 2 with `cur-top: could not write report to <path>: <reason>`. An input error leaves an existing `FILE` as it was.

The JSON report starts with `"schema_version": 1`. The number changes when a field is removed, renamed, or changes meaning; a new field does not change it. A failing run prints no JSON, only one error line on standard error and exit 2, so every JSON document has the field.

## Example

The commands below read the shipped `examples/cur.csv` and write nothing. The expected output of the text report is under Quick start.

```console
$ cur-top --json examples/cur.csv
{
  "schema_version": 1,
  "dictionary": "https://docs.aws.amazon.com/cur/latest/userguide/table-dictionary-cur2.html",
  "dictionary_date": "2026-11-09",
  "amount_column": "line_item_unblended_cost",
  "month": "2026-11",
  "previous_month": "2026-10",
  "previous_month_present": true,
  "rows": 3,
  "rows_without_amount": 0,
  "rows_without_tags": 0,
  "rows_outside_months": 0,
  "service": [
    {
      "key": "AmazonEC2",
      "amount": "3.50",
      "previous": "1.00",
      "delta": "2.50"
    },
    {
      "key": "AmazonS3",
      "amount": "1.00",
      "previous": "0.00",
      "delta": "1.00"
    }
  ],
  "usage_type": [
    {
      "key": "BoxUsage:t3.micro",
      "amount": "3.50",
      "previous": "1.00",
      "delta": "2.50"
    },
    {
      "key": "TimedStorage-ByteHrs",
      "amount": "1.00",
      "previous": "0.00",
      "delta": "1.00"
    }
  ],
  "tag": [
    {
      "key": "user_team=platform",
      "amount": "4.50",
      "previous": "1.00",
      "delta": "3.50"
    }
  ],
  "total": {
    "amount": "4.50",
    "previous": "1.00",
    "delta": "3.50"
  }
}
```

`--md` prints the same amounts as Markdown tables, one per group, instead of the text report. It cannot be combined with `--json` (usage error, exit 2). A `|` in a key becomes `\|` and a line break or other hidden character becomes `\u{..}`, so a key stays in its cell. A backslash, a backtick, and `* _ [ ] ( ) < > & ! ~ @` get a backslash in front (and so does a `#` at the start), so a renderer shows a key as text and not as a link, an image, HTML, or a heading. Amounts are not rounded again. The report says that the amounts are summed from the file and are not an estimate.

```console
$ cur-top --md examples/cur.csv
# cur-top

Month 2026-11 against 2026-10 (amount column `line_item_unblended_cost`). 3 rows read.

Not an estimate: the amounts are summed from the file.

## Service

| Key | Amount | Previous | Delta |
| --- | ---: | ---: | ---: |
| AmazonEC2 | 3.50 | 1.00 | 2.50 |
| AmazonS3 | 1.00 | 0.00 | 1.00 |

## Usage type

| Key | Amount | Previous | Delta |
| --- | ---: | ---: | ---: |
| BoxUsage:t3.micro | 3.50 | 1.00 | 2.50 |
| TimedStorage-ByteHrs | 1.00 | 0.00 | 1.00 |

## Tag

| Key | Amount | Previous | Delta |
| --- | ---: | ---: | ---: |
| user\_team=platform | 4.50 | 1.00 | 3.50 |

## Totals

| Amount | Previous | Delta |
| ---: | ---: | ---: |
| 4.50 | 1.00 | 3.50 |
```

## How it counts

C1. Each row adds `line_item_unblended_cost` as written. The tool does not multiply `line_item_unblended_rate` by `line_item_usage_amount`.
C2. Columns are the CUR 2.0 names `bill_billing_period_start_date`, `line_item_product_code`, `line_item_usage_type`, `line_item_unblended_cost`, `line_item_blended_cost`, `line_item_net_unblended_cost`, and `resource_tags`. All of them except `line_item_blended_cost` and `line_item_net_unblended_cost` are required, and `resource_tags` is a required column (it may be empty in a row). A name in the header must match exactly: it is not trimmed, so a header cell with a space around the name counts as a missing column. The two other cost columns are required only when chosen with `--amount`. Source: https://docs.aws.amazon.com/cur/latest/userguide/table-dictionary-cur2.html read on 2026-11-09.
C3. `--amount` accepts only `line_item_unblended_cost`, `line_item_blended_cost`, or `line_item_net_unblended_cost`.
C4. The month is the `YYYY-MM` of `bill_billing_period_start_date`. The current month is the latest month in the files, also when all its amounts are blank (its amounts are then `0.00`). The previous month is the calendar month before it. The month `0000-01` is valid and has no previous month: the report shows `previous: omitted`.
C5. Delta is the current amount minus the previous amount.
C6. A blank amount is skipped, and the error for a bad amount does not include the cell. ASCII space, tab, CR and LF around an amount are removed: a tab at either end of `1.00`, or a CR or LF at either end of a quoted `1.00`, is still 1.00. A form feed, a vertical tab, or a no-break space at either end is not removed and makes the amount invalid.
C7. Rows from months other than the current and previous month stay out of the totals.
C8. When the previous calendar month has no rows, previous amounts are `0.00` and `previous_month_present` is `no`.
C9. A negative amount in the CUR, such as a credit, is summed as written.
C10. The same row in two files is counted twice.
C11. Each `resource_tags` pair `key=value` receives the full line amount. A line with two tags is added to both. In a tag line, `\` becomes `\\` in the key and in the value, and `=` in the key becomes `\=`, so `a=b`/`c` and `a`/`b=c` stay apart, and the values ` x` and `\u{20}x` stay apart. `--json` shows the same tag line. Tag limits are those of `src/tags.rs`: at most 50 tags per cell, a key of 1 to 128 characters, a value of at most 256 characters, no control characters, no duplicate key, and a cell of at most 65536 bytes. JSON escapes in a tag cell are decoded, and a surrogate pair such as `\ud83d\ude00` is one character.
C12. A line with an empty `resource_tags` cell still counts toward the service and the usage type.
C13. Tables list the top `--top` keys by current amount, then by key. `--top` runs from 1 to 10000 inclusive. A key whose current amount is zero is skipped in the tables. Totals include every row in the two months.
C14. Data Exports gzip/csv is read when the first bytes are the gzip magic `1f 8b`, whatever the file name, also `.csv`. A name that ends in `.gz` must hold gzip: without the magic the file is rejected with `not gzip` and exit 2. Parquet and zip are rejected. Athena is not used. See https://docs.aws.amazon.com/cur/latest/userguide/dataexports-export-delivery.html read on 2026-11-09.
C15. `line_item_usage_account_id` and other extra columns are ignored and are not printed.
C16. There is no network call and no AWS SDK.
C17. An amount with more than 28 decimals, or digits that do not fit a 28-digit decimal unrounded, is invalid and exits 2; it is never rounded silently. An amount has at most 40 characters. An empty product code or usage type, or a value longer than 1024 bytes, is invalid. A monthly total, or a delta, that does not fit a 28-digit decimal exactly fails with `amount total does not fit`; it is never rounded. Every error exits 2. An invalid amount, product code, usage type, billing timestamp, or tag cell fails without echoing the cell.
C18. Output amounts are decimal text with at least two fraction digits and no scientific notation.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | The report was written (to standard output, or to `--output`). |
| 2 | An input or usage error: an unreadable or unsupported file, an invalid cell, a missing column, a bad option, `--output` that cannot be written or names an input file, or standard output that cannot be written (`could not write output`). One line on standard error, no report. |
| 130 | SIGINT (Ctrl-C) on Unix. |

On Unix, SIGINT (Ctrl-C) while the files are read stops the run with exit 130: no report, nothing on standard output, and one line `cur-top: interrupted by SIGINT` on standard error. Windows keeps its default Ctrl-C behaviour.

## Limitations

Version 1.0.0 reads AWS CUR 2.0 CSV only. Input is `.csv` or gzip `.csv.gz` (read as gzip when the file starts with the magic `1f 8b`). Parquet and zip are rejected. Athena is not used. There is no network and no AWS SDK. The tool does not multiply `line_item_unblended_rate` by `line_item_usage_amount`. Tags come from `resource_tags` as a JSON object, and each tag on a line receives that line's full amount. The same row in two files is counted twice. A symlink is followed. A directory is rejected. A file larger than 536870912 uncompressed bytes is rejected. A gzip file is rejected when one member's compressed bytes exceed the bytes that member unpacks by more than 1048576, when the file holds more than 40000 members, or when the compressed bytes of every member of every input file exceed 1073741824 (`gzip input is larger than 1073741824 bytes`, exit 2, no report). Measured on the release binary, one CUR member and then stored members of 1048573 bytes (209711 empty stored blocks, just under the per-member overhead), with one shorter stored member so the file is exact: 268435456 bytes exited 0 in 0.77 s (peak 2904 KB), 536870912 bytes exited 0 in 1.52 s (peak 2624 KB), and 1073741824 bytes exited 0 in 3.10 s (peak 2796 KB). The measured time grew by 1.97 times and by 4.03 times. A file of 1073741825 bytes exited 2 with that message and no report in 3.12 s (peak 2848 KB). The previous month is the calendar month before the latest billing period in the file. The tables are not a forecast. The quality test covers a CSV of at least 100 MiB. The 168-test suite (counted on Linux; eleven tests run on Linux only) and the 100 MiB timing are in [QUALITY.md](QUALITY.md).

More limits, each measured on the release binary (details in [QUALITY.md](QUALITY.md), "Open points"):

- File names. The suffix check is case sensitive: a name must end in `.csv` or `.gz` in lower case, so `UPPER.CSV` and `export.CSV` fail with `not a csv or csv.gz` and exit 2 (unless the file starts with the gzip magic). On Linux a name that is not valid UTF-8 is read when it ends in `.csv`; in an error it is shown as bytes, for example `bad\xff.txt`.
- Text report. In the keys of service and usage type, a space, a control, or an invisible or direction-changing character is shown as `\u{..}`, a backslash as `\\`, and the `=` of `amount=`, `previous=`, and `delta=` as `\=`, so two different keys never print the same line. `--json` prints the keys as they are, so a direction-changing or invisible character in a key is shown raw there (tag lines keep the escaping of C11); read `--json` with a program, not in a terminal that interprets such characters. A usage error on standard error shows a control, direction-changing, or invisible character of your arguments as `\u{..}` on one line, also after an escape sequence and also in an argument that is not valid UTF-8 (shown with U+FFFD). A file name in an error message shows control and invisible characters as `\u{..}` too.
- Input rules. A file is UTF-8 text, with a leading BOM and CRLF line ends allowed. `bill_billing_period_start_date` is a UTC time with a `Z`, with 1 to 6 fraction digits when there is a fraction, for example `2026-11-01T00:00:00Z` or `2026-11-01T00:00:00.123456Z`; a date alone, an offset such as `+01:00`, a space instead of the `T`, a lower case `t` or `z`, and 7 fraction digits are invalid. ASCII space, tab, CR and LF around the billing time, the product code, the usage type, the amount, and `resource_tags` are removed. A CR or LF is removed only when it is inside the cell, so that cell is quoted; an unquoted CR or LF ends the record. A form feed, a vertical tab, or a no-break space at either end is not removed and makes that field invalid. The CSV reader is tolerant: a quoted cell followed by text, as in `"A"x`, is read as the value `Ax`. Standard input is not read: `-` is a file name and fails with `No such file or directory`, and `/dev/stdin` fails (`not a file` for a pipe or a terminal).
- Row numbers. The number in `row N` of an error is the record of that file, counted from 1 after the header; a quoted cell with line breaks is one record. Every file starts again at 1, and the message names the file, for example `file 2: row 1: invalid amount`.
- Memory. The memory use is about 24 times the input when almost every row has its own service and usage type key: a 13 MB file with 300000 unique keys used 314 MiB, and a 92 MB file with 2000000 unique keys used 2.1 GiB. A 14 MB file with 20000 rows of 25 unique tags each used 303 MiB. The 536870912-byte limit is on the input, not on this state; by extrapolation (not run) a file at that limit with only unique keys would need about 12 GiB. A single cell is held in memory whole: a product code of 400 MiB (a 400 MiB file) was refused with `invalid line_item_product_code` and exit 2 after using 913 MiB. A file with few distinct keys needs little memory.
- A file that is cut off while it is being read. A CSV has no checksum, so a cut in the middle of the last field of the last row, without a final newline, can go unnoticed when the amount is the last column: `...,2.50` cut to `...,2` is summed as 2. A cut that leaves `2.` or a short row is an error. A gzip file detects a cut.
- Gzip errors. A gzip file that is cut off, has a wrong CRC, or has bytes after its last member fails with `could not read file` and exit 2; no report is printed.
- Gzip member overhead. Empty gzip members unpack to 0 bytes, so the limit of 536870912 unpacked bytes does not stop them. Each member may spend at most 1048576 compressed bytes beyond the bytes that member itself unpacks, and a file may hold at most 40000 members. Unpacked bytes from an earlier member do not raise either limit. Past either of those limits the tool prints `gzip overhead is larger than 1048576 bytes` and exits 2, with no report. The compressed bytes of every member of every input file may not exceed 1073741824. Past that total the tool prints `gzip input is larger than 1073741824 bytes` and exits 2, with no report. A `.csv.gz` name and a `.csv` name whose first bytes are the gzip magic use the same checks. Standard input is not read. Measured on the release binary, one member of 20971497 unpacked bytes (61192 compressed) followed by empty members of 20 bytes: 500000 members (10061192 bytes), 1000000 members (20061192 bytes), and 2000000 members (40061192 bytes) each exited 2 in 0.35 s (peak 2636 KB, 2688 KB, and 2896 KB). For these empty members of 20 bytes, the measured time stayed 0.35 s when the input grew by 4× and by 4× again. A heavier file, one CUR member followed by stored members of 1048573 bytes (209711 empty stored blocks, just under the per-member overhead) and one shorter stored member, exited 0 at 268435456 bytes in 0.77 s (peak 2904 KB), at 536870912 bytes in 1.52 s (peak 2624 KB), and at 1073741824 bytes in 3.10 s (peak 2796 KB). The measured time grew by 1.97 times and by 4.03 times. A file of 1073741825 bytes exited 2 with `gzip input is larger than 1073741824 bytes` and no report in 3.12 s (peak 2848 KB).
- Standard output. When the report cannot be written (closed stdout, a full disk, `/dev/full`, a closed pipe), also for `--help` and `--version`, the tool prints `could not write output` and exits 2. A stdout that was closed before the program started (`>&-`) is recognized on Linux only; on other systems it is not recognized. That check and the SIGINT handler are the only `unsafe` code of the project. They are in `src/stdout_probe.rs` and `src/interrupt.rs`, and the library keeps `#![forbid(unsafe_code)]`. A stdout that is open for reading only (`1<file`) is not recognized: on Linux the run exits 0 and writes nothing (not measured on the other systems).
- System requirements. The Linux release binary built with glibc needs GLIBC_2.34 or newer. A build for `x86_64-unknown-linux-musl` works and has no glibc requirement. A minimum version of macOS or Windows is not measured: the tests ran on Linux only, and for the two other systems the code was only type-checked and linted (see [QUALITY.md](QUALITY.md)).

## Maintenance

Report bugs through GitHub Issues: https://github.com/qwpp7g79y4-maker/cur-top/issues. After a release the tool is dormant. A new version appears only for a reported bug, a request, or a security problem; security always comes first.

## Support

Report bugs and questions through GitHub Issues: https://github.com/qwpp7g79y4-maker/cur-top/issues. For a security problem, read [SECURITY.md](SECURITY.md) first. Please include the command and a small synthetic CSV, never a real cost report.

## License

MIT OR Apache-2.0. See LICENSE-MIT and LICENSE-APACHE.

The licenses of the third-party crates built into the binary are listed in [THIRD-PARTY-LICENSES.html](THIRD-PARTY-LICENSES.html).

## Contact

- E-mail: pep_ai@icloud.com
- Site: https://www.pepai.nl
- X: https://x.com/pepai_nl
- Ko-fi: https://ko-fi.com/pepai
- Git: https://github.com/qwpp7g79y4-maker/cur-top
