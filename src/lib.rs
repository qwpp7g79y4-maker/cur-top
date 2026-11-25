//! Offline top costs from an AWS CUR 2.0 CSV export.
//!
//! The library reads bytes you already have. It does not open a socket, it does
//! not link an AWS SDK, and it does not query Athena. It sums a cost column
//! that is already in the file. It does not multiply a rate by a usage amount.

#![forbid(unsafe_code)]

mod columns;
mod error;
mod json_scan;
mod money;
mod report;
mod run;
mod summary;
mod tags;

pub use columns::{
    parse_amount_column, BILL_DICTIONARY_URL, DICTIONARY_DATE, DICTIONARY_URL,
    LINE_ITEM_DICTIONARY_URL, RESOURCE_TAGS_DICTIONARY_URL,
};
pub use error::Error;
pub use report::{render, render_markdown};
pub use run::{
    execute_paths, execute_paths_limited, request_stop, stop_requested, summarize_paths, Output,
};
pub use summary::{summarize_slices, Line, Options, Summary};

/// Largest uncompressed CSV accepted per file, in bytes (512 MiB).
/// The 100 MiB quality file fits under this cap.
pub const MAX_UNCOMPRESSED_BYTES: u64 = 536_870_912;

/// Compressed bytes one gzip member may spend beyond the bytes that member unpacks.
///
/// The allowance belongs to that member alone. Unpacked bytes from an earlier
/// member do not raise it, so a member that unpacks to nothing cannot spend
/// more than this many compressed bytes. The decoder may pull one 32 KiB
/// buffer before those unpacked bytes are counted, so the allowance stays
/// well above that buffer. Passing this limit, or [`MAX_GZIP_MEMBERS`], fails
/// the read.
pub const MAX_GZIP_OVERHEAD_BYTES: u64 = 1_048_576;

/// Most gzip members one file may contain.
///
/// Empty members unpack to nothing and are only a few dozen compressed bytes,
/// so the per-member allowance above does not bound how many of them a file
/// can chain together. 40000 covers a normal multi-member export and stops a
/// long chain of empty members before that chain dominates the read.
pub const MAX_GZIP_MEMBERS: u64 = 40_000;

/// Most compressed gzip bytes one run may read (1 GiB).
///
/// The total covers every member of every input file. A `.gz` name and a file
/// that starts with the gzip magic share this counter. Passing it fails the
/// read. [`MAX_GZIP_OVERHEAD_BYTES`] still limits each member on its own.
pub const MAX_GZIP_COMPRESSED_BYTES: u64 = 1_073_741_824;
