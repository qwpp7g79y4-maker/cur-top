//! SIGINT stops cur-top before it prints a report: exit 130, nothing on
//! standard output, one line on standard error, no panic.

#![cfg(unix)]

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use flate2::write::GzEncoder;
use flate2::Compression;

const SIGINT: i32 = 2;

extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

const HEADER: &str = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,line_item_unblended_rate,line_item_usage_amount,resource_tags\n";
const ROW: &str = "2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,3.50,0.0104,336,\"{\"\"user_team\"\":\"\"platform\"\"}\"\n";

/// A gzip file that unpacks to about 96 MiB of valid rows: small on disk,
/// but long enough to read that the signal arrives while cur-top is reading.
fn big_input() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sigint");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("big.csv.gz");
    let mut chunk = String::new();
    while chunk.len() < (1 << 20) {
        chunk.push_str(ROW);
    }
    let mut enc = GzEncoder::new(File::create(&path).unwrap(), Compression::fast());
    enc.write_all(HEADER.as_bytes()).unwrap();
    for _ in 0..96 {
        enc.write_all(chunk.as_bytes()).unwrap();
    }
    enc.finish().unwrap();
    path
}

#[test]
fn sigint_while_reading_exits_130_with_no_report_and_no_panic() {
    let input = big_input();
    let mut child = Command::new(env!("CARGO_BIN_EXE_cur-top"))
        .arg(&input)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let pid = i32::try_from(child.id()).unwrap();
    assert!(pid > 0);
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        child.try_wait().unwrap().is_none(),
        "cur-top finished before the signal was sent"
    );
    // SAFETY: `pid` is the child this test spawned and has not reaped.
    assert_eq!(unsafe { kill(pid, SIGINT) }, 0);

    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("cur-top did not stop after SIGINT");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut out = String::new();
    let mut err = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut err)
        .unwrap();
    assert_eq!(status.code(), Some(130), "status {status:?}, stderr {err}");
    assert!(out.is_empty(), "{out}");
    assert_eq!(err, "cur-top: interrupted by SIGINT\n");
}
