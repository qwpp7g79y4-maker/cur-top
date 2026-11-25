use std::cell::Cell;
use std::fs::{self, File};
use std::io::{self, BufRead, Cursor, Read};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use flate2::bufread::GzDecoder;

use crate::error::Error;
use crate::summary::{check_options, Acc, Options, Summary};
use crate::{
    MAX_GZIP_COMPRESSED_BYTES, MAX_GZIP_MEMBERS, MAX_GZIP_OVERHEAD_BYTES, MAX_UNCOMPRESSED_BYTES,
};

/// Text or JSON written to stdout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub stdout: String,
}

/// Read CUR files from disk and render the report.
pub fn execute_paths(paths: &[PathBuf], opts: &Options) -> Result<Output, Error> {
    execute_paths_limited(paths, opts, MAX_GZIP_COMPRESSED_BYTES)
}

/// [`execute_paths`] with `compressed_cap` as the limit on gzip bytes read.
pub fn execute_paths_limited(
    paths: &[PathBuf],
    opts: &Options,
    compressed_cap: u64,
) -> Result<Output, Error> {
    let summary = summarize_paths_limited(paths, opts, compressed_cap)?;
    Ok(Output {
        stdout: crate::render(&summary, opts.json),
    })
}

/// Read one or more `.csv` or `.csv.gz` files into one summary.
pub fn summarize_paths(paths: &[PathBuf], opts: &Options) -> Result<Summary, Error> {
    summarize_paths_limited(paths, opts, MAX_GZIP_COMPRESSED_BYTES)
}

fn summarize_paths_limited(
    paths: &[PathBuf],
    opts: &Options,
    compressed_cap: u64,
) -> Result<Summary, Error> {
    let column = check_options(opts)?;
    let mut acc = Acc::default();
    let budget = Rc::new(GzipBudget {
        total: Cell::new(0),
        cap: compressed_cap,
        over: Cell::new(false),
    });
    for (i, path) in paths.iter().enumerate() {
        if stop_requested() {
            return Err(Error::new("interrupted by SIGINT"));
        }
        let reader = open_decoded(path, Rc::clone(&budget))?;
        acc.ingest(reader, column, i + 1, MAX_UNCOMPRESSED_BYTES)?;
    }
    acc.finish(column, opts.top)
}

/// Path text for an error line. `escape_debug` keeps ESC and other controls out of the terminal.
/// A path that is not valid UTF-8 shows its raw bytes through `escape_ascii`, never U+FFFD.
fn shown_path(path: &Path) -> String {
    if let Some(text) = path.to_str() {
        if !text.contains('\u{2800}') {
            return text.escape_debug().to_string();
        }
        // U+2800 (braille blank) is invisible but not escaped by `escape_debug`.
        return text
            .chars()
            .map(|c| {
                if c == '\u{2800}' {
                    "\\u{2800}".to_string()
                } else {
                    c.escape_debug().to_string()
                }
            })
            .collect();
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().escape_ascii().to_string()
    }
    #[cfg(not(unix))]
    {
        path.display().to_string().escape_debug().to_string()
    }
}

static STOP: AtomicBool = AtomicBool::new(false);

/// Ask a running read to stop. A signal handler may call this: it is one
/// atomic store.
pub fn request_stop() {
    STOP.store(true, Ordering::SeqCst);
}

/// Has [`request_stop`] been called (SIGINT)?
pub fn stop_requested() -> bool {
    STOP.load(Ordering::SeqCst)
}

fn interrupted_error() -> io::Error {
    io::Error::other("interrupted by SIGINT")
}

/// Fails every read once a stop was requested.
struct StopOnRequest<R>(R);

impl<R: Read> Read for StopOnRequest<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if stop_requested() {
            return Err(interrupted_error());
        }
        self.0.read(buf)
    }
}

fn open_decoded(path: &Path, budget: Rc<GzipBudget>) -> Result<Box<dyn Read>, Error> {
    let meta =
        fs::metadata(path).map_err(|err| Error::new(format!("{}: {err}", shown_path(path))))?;
    if meta.is_dir() {
        return Err(Error::new(format!("{}: is a directory", shown_path(path))));
    }
    if !meta.is_file() {
        return Err(Error::new(format!("{}: not a file", shown_path(path))));
    }
    let mut file =
        File::open(path).map_err(|err| Error::new(format!("{}: {err}", shown_path(path))))?;
    let mut magic = [0u8; 4];
    let n = file
        .read(&mut magic)
        .map_err(|err| Error::new(format!("{}: {err}", shown_path(path))))?;
    let head = magic[..n].to_vec();
    let kind = classify(path, &head)?;
    let chained = Cursor::new(head).chain(file);
    match kind {
        Kind::Csv => Ok(Box::new(StopOnRequest(chained))),
        Kind::Gzip => Ok(Box::new(StopOnRequest(limited_gzip(chained, budget)))),
    }
}

/// Compressed bytes already read in this run, shared by every input file.
struct GzipBudget {
    total: Cell<u64>,
    cap: u64,
    /// Set once the total passes `cap`, so a later read cannot continue.
    over: Cell<bool>,
}

/// Per-member gzip limits. Unpacked bytes from an earlier member are not kept.
struct GzipMeters {
    /// Absolute consumed offset where the current member began.
    member_start: Cell<u64>,
    /// Unpacked bytes produced by the current member.
    member_uncompressed: Cell<u64>,
    /// Members started, including the one being read.
    members: Cell<u64>,
    budget: Rc<GzipBudget>,
}

/// Buffered reader that counts compressed bytes of the current member.
///
/// Bytes still sitting in `buf` are not counted toward the member: they may
/// belong to the next member. They do count toward the run-wide compressed
/// total, because they have already been read. A fill never pulls more than
/// the remaining member allowance plus one byte, and never more than the
/// bytes still allowed by the run total plus one byte.
struct Metered<R> {
    inner: R,
    buf: Vec<u8>,
    pos: usize,
    /// Bytes copied out of `inner`, including those still in `buf`.
    pulled: u64,
    meters: Rc<GzipMeters>,
}

const GZIP_FILL: u64 = 32 * 1024;

impl<R> Metered<R> {
    fn consumed(&self) -> u64 {
        let unread = (self.buf.len() - self.pos) as u64;
        self.pulled - unread
    }

    fn member_compressed(&self) -> io::Result<u64> {
        self.consumed()
            .checked_sub(self.meters.member_start.get())
            .ok_or_else(gzip_overhead_error)
    }

    fn allowance(&self) -> io::Result<u64> {
        self.meters
            .member_uncompressed
            .get()
            .checked_add(MAX_GZIP_OVERHEAD_BYTES)
            .ok_or_else(gzip_overhead_error)
    }

    fn check_allowance(&self) -> io::Result<()> {
        if self.member_compressed()? > self.allowance()? {
            Err(gzip_overhead_error())
        } else {
            Ok(())
        }
    }

    /// Count `n` compressed bytes toward the run total. `n == 0` is EOF.
    fn accept_compressed(&self, n: usize) -> io::Result<()> {
        if self.meters.budget.over.get() {
            return Err(gzip_total_error(self.meters.budget.cap));
        }
        if n == 0 {
            return Ok(());
        }
        let add = u64::try_from(n).map_err(|_| gzip_total_error(self.meters.budget.cap))?;
        let Some(total) = self.meters.budget.total.get().checked_add(add) else {
            self.meters.budget.over.set(true);
            return Err(gzip_total_error(self.meters.budget.cap));
        };
        if total > self.meters.budget.cap {
            self.meters.budget.over.set(true);
            return Err(gzip_total_error(self.meters.budget.cap));
        }
        self.meters.budget.total.set(total);
        Ok(())
    }
}

impl<R: Read> Read for Metered<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let data = self.fill_buf()?;
        let n = data.len().min(buf.len());
        buf[..n].copy_from_slice(&data[..n]);
        self.consume(n);
        Ok(n)
    }
}

impl<R: Read> BufRead for Metered<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.meters.budget.over.get() {
            return Err(gzip_total_error(self.meters.budget.cap));
        }
        if self.pos >= self.buf.len() {
            self.check_allowance()?;
            let member_room = self.allowance()? - self.member_compressed()?;
            let already = self.meters.budget.total.get();
            let total_room = self
                .meters
                .budget
                .cap
                .saturating_sub(already)
                .saturating_add(1);
            let want = member_room.saturating_add(1).min(GZIP_FILL).min(total_room);
            let cap = usize::try_from(want).unwrap_or(1).max(1);
            self.buf.resize(cap, 0);
            let n = self.inner.read(&mut self.buf)?;
            if let Err(err) = self.accept_compressed(n) {
                self.buf.clear();
                self.pos = 0;
                return Err(err);
            }
            self.buf.truncate(n);
            self.pos = 0;
            let add = u64::try_from(n).map_err(|_| gzip_overhead_error())?;
            self.pulled = self
                .pulled
                .checked_add(add)
                .ok_or_else(gzip_overhead_error)?;
        }
        Ok(&self.buf[self.pos..])
    }

    fn consume(&mut self, amt: usize) {
        let unread = self.buf.len() - self.pos;
        self.pos += amt.min(unread);
    }
}

enum State<R> {
    Ready(Box<Metered<R>>),
    Decoding(Box<GzDecoder<Metered<R>>>),
    Eof,
}

struct LimitedGzip<R> {
    state: State<R>,
    meters: Rc<GzipMeters>,
}

impl<R: Read> Read for LimitedGzip<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        loop {
            match &mut self.state {
                State::Eof => return Ok(0),
                State::Ready(_) => {
                    let State::Ready(boxed) = std::mem::replace(&mut self.state, State::Eof) else {
                        unreachable!()
                    };
                    let mut source = *boxed;
                    if source.fill_buf()?.is_empty() {
                        self.state = State::Eof;
                        return Ok(0);
                    }
                    let members = self
                        .meters
                        .members
                        .get()
                        .checked_add(1)
                        .ok_or_else(gzip_overhead_error)?;
                    if members > MAX_GZIP_MEMBERS {
                        return Err(gzip_overhead_error());
                    }
                    self.meters.members.set(members);
                    self.meters.member_start.set(source.consumed());
                    self.meters.member_uncompressed.set(0);
                    self.state = State::Decoding(Box::new(GzDecoder::new(source)));
                }
                State::Decoding(decoder) => match decoder.read(buf) {
                    Ok(0) => {
                        decoder.get_ref().check_allowance()?;
                        let State::Decoding(decoder) =
                            std::mem::replace(&mut self.state, State::Eof)
                        else {
                            unreachable!()
                        };
                        let source = decoder.into_inner();
                        // The next peek must not charge the finished member.
                        // Bytes left in the buffer belong to the next member.
                        self.meters.member_start.set(source.consumed());
                        self.meters.member_uncompressed.set(0);
                        self.state = State::Ready(Box::new(source));
                    }
                    Ok(n) => {
                        let add = u64::try_from(n).map_err(|_| gzip_overhead_error())?;
                        let total = self
                            .meters
                            .member_uncompressed
                            .get()
                            .checked_add(add)
                            .ok_or_else(gzip_overhead_error)?;
                        self.meters.member_uncompressed.set(total);
                        decoder.get_ref().check_allowance()?;
                        return Ok(n);
                    }
                    Err(err) => return Err(err),
                },
            }
        }
    }
}

fn gzip_overhead_error() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "cur-top-gzip-overhead")
}

fn gzip_total_error(cap: u64) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("cur-top-gzip-total:{cap}"),
    )
}

fn limited_gzip<R: Read>(reader: R, budget: Rc<GzipBudget>) -> LimitedGzip<R> {
    let meters = Rc::new(GzipMeters {
        member_start: Cell::new(0),
        member_uncompressed: Cell::new(0),
        members: Cell::new(0),
        budget,
    });
    let source = Metered {
        inner: reader,
        buf: Vec::new(),
        pos: 0,
        pulled: 0,
        meters: Rc::clone(&meters),
    };
    LimitedGzip {
        state: State::Ready(Box::new(source)),
        meters,
    }
}

/// Raw bytes of the last path component, so a non-UTF-8 name still has a suffix.
fn file_name_bytes(path: &Path) -> Vec<u8> {
    let Some(name) = path.file_name() else {
        return Vec::new();
    };
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        name.as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        name.to_str().unwrap_or("").as_bytes().to_vec()
    }
}

enum Kind {
    Csv,
    Gzip,
}

fn classify(path: &Path, magic: &[u8]) -> Result<Kind, Error> {
    let name = file_name_bytes(path);
    if magic.starts_with(b"PAR1") || name.ends_with(b".parquet") {
        return Err(Error::new(format!(
            "{}: parquet is not supported",
            shown_path(path)
        )));
    }
    if magic.starts_with(b"PK") || name.ends_with(b".zip") {
        return Err(Error::new(format!(
            "{}: zip is not supported",
            shown_path(path)
        )));
    }
    let gzip_magic = magic.starts_with(&[0x1f, 0x8b]);
    if name.ends_with(b".gz") || gzip_magic {
        if !gzip_magic {
            return Err(Error::new(format!("{}: not gzip", shown_path(path))));
        }
        return Ok(Kind::Gzip);
    }
    if name.ends_with(b".csv") {
        return Ok(Kind::Csv);
    }
    if std::str::from_utf8(&name).is_err() {
        return Err(Error::new(format!(
            "{}: name is not UTF-8 (bytes: {})",
            shown_path(path),
            name.escape_ascii()
        )));
    }
    Err(Error::new(format!(
        "{}: not a csv or csv.gz",
        shown_path(path)
    )))
}

#[cfg(test)]
mod tests {
    use super::summarize_paths;
    use crate::summary::Options;
    use crate::MAX_UNCOMPRESSED_BYTES;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::fs;
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Scratch {
        path: PathBuf,
    }

    impl Scratch {
        fn new() -> Self {
            static N: AtomicU64 = AtomicU64::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("cur-top-{}-{n}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn opts() -> Options {
        Options {
            amount_column: "line_item_unblended_cost".to_string(),
            top: 10,
            json: false,
        }
    }

    fn tiny() -> &'static str {
        "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.50,{}\n"
    }

    #[test]
    fn claim_c14_gzip_is_read_and_parquet_and_zip_are_rejected() {
        let scratch = Scratch::new();
        let csv_gz = scratch.path().join("export-00001.csv.gz");
        let mut enc = GzEncoder::new(Vec::new(), Compression::fast());
        enc.write_all(tiny().as_bytes()).unwrap();
        fs::write(&csv_gz, enc.finish().unwrap()).unwrap();
        let summary = summarize_paths(&[csv_gz], &opts()).unwrap();
        assert_eq!(summary.total_amount, "1.50");

        let gz = scratch.path().join("chunk.gz");
        let mut enc = GzEncoder::new(Vec::new(), Compression::fast());
        enc.write_all(tiny().as_bytes()).unwrap();
        fs::write(&gz, enc.finish().unwrap()).unwrap();
        let summary = summarize_paths(&[gz], &opts()).unwrap();
        assert_eq!(summary.total_amount, "1.50");

        let parquet = scratch.path().join("export.snappy.parquet");
        fs::write(&parquet, b"PAR1secret-body").unwrap();
        let err = summarize_paths(&[parquet], &opts()).unwrap_err();
        assert!(err.message().contains("parquet is not supported"));
        assert!(!err.message().contains("secret"));

        let zip = scratch.path().join("data.csv.gz");
        fs::write(&zip, b"PK\x03\x04secret").unwrap();
        let err = summarize_paths(std::slice::from_ref(&zip), &opts()).unwrap_err();
        assert!(err.message().contains("zip is not supported"));
        assert!(!err.message().contains("secret"));
    }

    #[test]
    fn claim_c16_manifest_has_no_sdk_athena_or_parquet() {
        let manifest = include_str!("../Cargo.toml");
        for banned in ["parquet", "athena", "aws-sdk", "reqwest", "tokio", "hyper"] {
            assert!(!manifest.contains(banned), "{banned}");
        }
        assert!(manifest.contains("flate2"));
        assert!(manifest.contains("csv"));
        for source in [
            include_str!("lib.rs"),
            include_str!("main.rs"),
            include_str!("summary.rs"),
            include_str!("report.rs"),
        ] {
            assert!(!source.contains("Command::new"));
            assert!(!source.contains("aws_sdk"));
            assert!(!source.contains("aws-sdk"));
            assert!(!source.contains("reqwest"));
            assert!(!source.contains("athena"));
        }
    }

    #[test]
    fn unpacked_cap_constant_and_a_cap_of_4_bytes_reject_without_echoing_the_input() {
        assert_eq!(MAX_UNCOMPRESSED_BYTES, 536_870_912);
        let scratch = Scratch::new();
        let path = scratch.path().join("wide.csv");
        fs::write(&path, tiny()).unwrap();
        let summary = summarize_paths(&[path], &opts()).unwrap();
        assert_eq!(summary.total_amount, "1.50");
        let mut acc = crate::summary::Acc::default();
        let payload = b"secretsecretsecret";
        let message = acc
            .ingest(
                std::io::Cursor::new(&payload[..]),
                "line_item_unblended_cost",
                1,
                4,
            )
            .unwrap_err()
            .message()
            .to_string();
        assert!(message.contains("file is larger than 4 bytes"), "{message}");
        assert!(!message.contains("secret"), "{message}");
    }

    #[test]
    #[cfg(unix)]
    fn directory_is_rejected_and_symlink_is_followed() {
        let scratch = Scratch::new();
        let dir = scratch.path().join("folder.csv");
        fs::create_dir(&dir).unwrap();
        let err = summarize_paths(&[dir], &opts()).unwrap_err();
        assert!(err.message().contains("is a directory"));

        let real = scratch.path().join("real.csv");
        fs::write(&real, tiny()).unwrap();
        let link = scratch.path().join("link.csv");
        symlink(&real, &link).unwrap();
        let summary = summarize_paths(&[link], &opts()).unwrap();
        assert_eq!(summary.total_amount, "1.50");
    }

    #[test]
    fn not_utf8() {
        let scratch = Scratch::new();
        let path = scratch.path().join("bad.csv");
        fs::write(&path, [0xff, 0xfe, b'\n']).unwrap();
        let err = summarize_paths(&[path], &opts()).unwrap_err();
        assert!(err.message().contains("not UTF-8"), "{}", err.message());
    }

    #[test]
    fn other_extension_rejected() {
        let scratch = Scratch::new();
        let path = scratch.path().join("notes.txt");
        fs::write(&path, tiny()).unwrap();
        let err = summarize_paths(&[path], &opts()).unwrap_err();
        assert!(err.message().contains("not a csv or csv.gz"));
    }

    #[test]
    fn not_gzip_payload() {
        let scratch = Scratch::new();
        let path = scratch.path().join("export.csv.gz");
        fs::write(&path, b"this is not gzip").unwrap();
        let err = summarize_paths(&[path], &opts()).unwrap_err();
        assert!(err.message().contains("not gzip"), "{}", err.message());
        assert!(!err.message().contains("this is"), "{}", err.message());
    }

    #[test]
    fn two_gzip_members_are_both_counted() {
        let scratch = Scratch::new();
        let path = scratch.path().join("m.csv.gz");
        let header_and_row = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n2026-11-01T00:00:00Z,A,U,1.00,{}\n";
        let second_row = "2026-11-01T00:00:00Z,B,U,5.00,{}\n";
        let mut bytes = Vec::new();
        for part in [header_and_row, second_row] {
            let mut enc = GzEncoder::new(Vec::new(), Compression::fast());
            enc.write_all(part.as_bytes()).unwrap();
            bytes.extend(enc.finish().unwrap());
        }
        fs::write(&path, bytes).unwrap();
        let summary = summarize_paths(&[path], &opts()).unwrap();
        assert_eq!(summary.rows, 2);
        assert_eq!(summary.total_amount, "6.00");
    }
}
