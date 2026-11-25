//! One CSV of at least 100 MiB, summed once, checked against the closed form.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use cur_top::{summarize_paths, Options};

#[test]
#[ignore = "builds a 100 MiB file; run on purpose"]
fn quality_benchmark_100_mb() {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("cur-top-quality-{}-{n}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("quality.csv");
    let result = write_and_sum(&path);
    let _ = fs::remove_dir_all(&dir);
    result.unwrap();
}

fn write_and_sum(path: &PathBuf) -> Result<(), String> {
    let header = b"bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n";
    let row = b"2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,0.01,{}\n";
    let target = 100 * 1024 * 1024;
    let file = File::create(path).map_err(|err| err.to_string())?;
    let mut out = BufWriter::new(file);
    out.write_all(header).map_err(|err| err.to_string())?;
    let mut bytes = header.len() as u64;
    let mut rows = 0u64;
    while bytes < target {
        out.write_all(row).map_err(|err| err.to_string())?;
        bytes += row.len() as u64;
        rows += 1;
    }
    out.flush().map_err(|err| err.to_string())?;
    drop(out);
    let len = fs::metadata(path).map_err(|err| err.to_string())?.len();
    if len < target {
        return Err(format!("file is {len} bytes"));
    }
    let summary = summarize_paths(
        std::slice::from_ref(path),
        &Options {
            amount_column: "line_item_unblended_cost".to_string(),
            top: 10,
            json: false,
        },
    )
    .map_err(|err| err.message().to_string())?;
    let whole = rows / 100;
    let frac = rows % 100;
    let expect = format!("{whole}.{frac:02}");
    if summary.total_amount != expect {
        return Err(format!(
            "total {} != {expect} for {rows} rows",
            summary.total_amount
        ));
    }
    if summary.rows != rows {
        return Err(format!("rows {} != {rows}", summary.rows));
    }
    if summary.service.len() != 1 || summary.service[0].key != "AmazonEC2" {
        return Err(format!("service {:?}", summary.service));
    }
    if summary.service[0].amount != expect {
        return Err(format!("service amount {}", summary.service[0].amount));
    }
    if summary.rows_outside_months != 0 || summary.rows_without_amount != 0 {
        return Err("unexpected skipped rows".to_string());
    }
    Ok(())
}
