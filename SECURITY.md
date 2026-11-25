# Security policy

## Supported versions

| Version | Supported |
| --- | --- |
| 1.0.x | Yes |

Security fixes go into the latest release only. The crate is not published and has no git tag.

## Reporting a vulnerability

Please report security problems privately, not in a public issue: open the **Security** tab of this repository and choose **Report a vulnerability**. If that is not possible, write to pep_ai@icloud.com. Examples are a CSV cell, file name, or tag that can add a fake line or a terminal escape to the report or to an error, an error that echoes a cell it should hide, a way to make cur-top write, run, or connect to something, a file that makes it read without the stated size limit or hang, and input that changes an amount without an error.

The stated size limits are 536870912 uncompressed bytes per file and, for gzip, 1048576 compressed bytes beyond the bytes that one member itself unpacks, at most 40000 gzip members in one file, and 1073741824 compressed bytes across every member of every input file. Unpacked bytes from an earlier member do not raise the per-member overhead or the member cap. Empty gzip members unpack to nothing, so the member cap stops a long chain of tiny members, and the compressed-byte total stops a chain of members that are each just under the per-member overhead. Past that total the tool prints `gzip input is larger than 1073741824 bytes` and exits 2, with no report. Measured on the release binary: one member of 20971497 unpacked bytes followed by 500000, 1000000, and 2000000 empty members of 20 bytes (10061192, 20061192, and 40061192 bytes) each exited 2 in 0.35 s (peak 2636 KB, 2688 KB, and 2896 KB) with no report. For these empty members of 20 bytes, the measured time stayed 0.35 s when the input grew by 4× and by 4× again. One CUR member, then stored members of 1048573 bytes (209711 empty stored blocks) and one shorter stored member, exited 0 at 268435456 bytes in 0.77 s (peak 2904 KB), at 536870912 bytes in 1.52 s (peak 2624 KB), and at 1073741824 bytes in 3.10 s (peak 2796 KB). The measured time grew by 1.97 times and by 4.03 times. A file of 1073741825 bytes exited 2 with that message and no report in 3.12 s (peak 2848 KB).

Memory use with many unique keys is a documented limit (see the README, "More limits"), not a vulnerability by itself.

Please include:

- the cur-top version (the `version` field in `Cargo.toml`, or the commit you built) and your operating system;
- the exact command;
- a minimal synthetic CSV or `.csv.gz` that reproduces the problem, never a real cost report;
- what cur-top printed and what you expected.

We aim to respond within 7 days (best effort). Once a fix is released, the report can be credited in the changelog if you wish.
