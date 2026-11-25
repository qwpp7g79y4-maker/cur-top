# Contributing

Thanks for your interest in cur-top.

## Building and testing

```sh
cargo build
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

The minimum supported Rust version is 1.85, so also run `cargo +1.85 clippy --all-targets -- -D warnings` and `cargo +1.85 test`. These checks must pass before a change is merged.

Test fixtures must be synthetic. Never add a real account id, a real tag, or a real cost report. The shipped example, `examples/cur.csv`, is made up.

## Changing the report

The README examples and the files in `examples/` are checked against the binary by the tests. When the output changes, update `README.md`, `README.nl.md`, the `examples/cur.expected.*` files, and the man page together, and keep the test count in `README.md`, `README.nl.md`, and `QUALITY.md` equal to the real count (a test checks it).

## Commit messages

Commit messages are short English sentences that say what changed, for example `Add --output to write the report to a file`.

## Contact

PEPAI, pep_ai@icloud.com
