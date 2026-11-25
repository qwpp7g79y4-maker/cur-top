# Examples

`cur.csv` is a small, synthetic AWS CUR 2.0 export: three rows, two billing months (2026-10 and 2026-11), one tag. It holds no real account, and the commands below only read it.

Run these from the repository root. Each command prints exactly the file next to it:

| Command | Expected output |
| --- | --- |
| `cur-top examples/cur.csv` | `cur.expected.txt` |
| `cur-top --json examples/cur.csv` | `cur.expected.json` |
| `cur-top --md examples/cur.csv` | `cur.expected.md` |

To compare by hand:

```sh
cur-top examples/cur.csv | diff - examples/cur.expected.txt
```

No output from `diff` means the same bytes. To write a report to a file of your own, use `--output`, for example `cur-top --output /tmp/report.txt examples/cur.csv`; never point it at a file in this directory.

The test `the_example_files_match_the_binary` (`tests/readme.rs`) runs these three commands and compares the output with the expected files, so a change to the report fails the build until the files are updated.
