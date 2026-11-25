use std::ffi::OsString;
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::Parser;

#[allow(unsafe_code)]
mod interrupt;
#[allow(unsafe_code)]
mod stdout_probe;

use cur_top::{
    parse_amount_column, render, render_markdown, stop_requested, summarize_paths, Options,
};

// `-V` and `--version` print this. A debug build appends the commit git
// reported at compile time; a release build is the crate version only.
#[cfg(all(debug_assertions, cur_top_git_sha))]
const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CUR_TOP_GIT_SHA"),
    ")"
);
#[cfg(not(all(debug_assertions, cur_top_git_sha)))]
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Top CUR 2.0 costs from a local CSV. Reads files only, no network.
#[derive(Parser)]
#[command(
    name = "cur-top",
    version = VERSION,
    about = "Top CUR 2.0 costs from a local CSV. Reads files only, no network.",
    after_help = "Exit codes: 0 ok, 2 input or usage error."
)]
struct Cli {
    /// AWS CUR 2.0 `.csv` or `.csv.gz` files.
    #[arg(required = true, value_name = "FILE")]
    files: Vec<PathBuf>,
    /// Cost column to sum. One of the three CUR 2.0 line-item cost columns.
    #[arg(long, default_value = "line_item_unblended_cost", value_parser = amount_parser)]
    amount: String,
    /// How many keys to list for each of service, usage type, and tag.
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u32).range(1..=10000))]
    top: u32,
    /// Print JSON instead of the text report.
    #[arg(long, conflicts_with = "md")]
    json: bool,
    /// Print a Markdown report (one table per group) instead of the text report.
    #[arg(long, conflicts_with = "json")]
    md: bool,
    /// Write the report to this file instead of standard output.
    #[arg(long, value_name = "FILE")]
    output: Option<PathBuf>,
    /// Print nothing on standard error after a successful run. No context line.
    #[arg(long, conflicts_with = "verbose")]
    quiet: bool,
    /// On success, print one context line on standard error.
    #[arg(long, conflicts_with = "quiet")]
    verbose: bool,
}

/// SIGINT: no report, one line on standard error, exit 130.
fn interrupted() -> ExitCode {
    let _ = writeln!(std::io::stderr(), "cur-top: interrupted by SIGINT");
    ExitCode::from(interrupt::EXIT_INTERRUPTED)
}

fn amount_parser(raw: &str) -> Result<String, String> {
    parse_amount_column(raw)
        .map(str::to_string)
        .map_err(|err| err.message().to_string())
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            if matches!(
                err.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                return write_stdout(&err.render().to_string());
            }
            let code = if err.use_stderr() { 2 } else { 0 };
            let text = safe_usage_error(&err);
            if err.use_stderr() {
                let _ = write!(std::io::stderr(), "{text}");
            } else {
                let _ = write!(stdout(), "{text}");
            }
            return ExitCode::from(code);
        }
    };
    let options = Options {
        amount_column: cli.amount,
        top: cli.top as usize,
        json: cli.json,
    };
    interrupt::install();
    if let Some(path) = &cli.output {
        if let Some(input) = cli.files.iter().find(|file| same_file(path, file)) {
            let _ = writeln!(
                std::io::stderr(),
                "cur-top: {}: is also an input file; --output would overwrite it",
                escape_raw(&input.to_string_lossy())
            );
            return ExitCode::from(2);
        }
    }
    match summarize_paths(&cli.files, &options) {
        // A stop that came in before the report is written prints nothing.
        Ok(_) | Err(_) if stop_requested() => interrupted(),
        Ok(summary) => {
            let text = if cli.md {
                render_markdown(&summary)
            } else {
                render(&summary, options.json)
            };
            let code = match &cli.output {
                Some(path) => write_file(path, &text),
                None => write_stdout(&text),
            };
            if cli.verbose && code == ExitCode::SUCCESS {
                // A closed standard error must not panic.
                let _ = writeln!(
                    std::io::stderr(),
                    "context: rows {}, service {}, usage_type {}, tag {}",
                    summary.rows,
                    summary.service.len(),
                    summary.usage_type.len(),
                    summary.tag.len()
                );
            }
            code
        }
        Err(err) => {
            let _ = writeln!(std::io::stderr(), "cur-top: {err}");
            ExitCode::from(2)
        }
    }
}

/// A character that can hide or move text on a terminal.
fn is_raw_danger(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{61c}' | '\u{200b}'..='\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{206f}' | '\u{feff}'
        )
}

fn escape_raw(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if is_raw_danger(c) {
            out.push_str(&format!("\\u{{{:x}}}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

/// The text of a clap usage error with nothing the caller typed left raw.
///
/// clap strips ANSI sequences from the arguments it echoes and shows an
/// argument that is not UTF-8 with U+FFFD, so the message cannot be searched
/// for the raw argument. Instead the arguments are judged by their lossy
/// text, escaped, and parsed again: every line break of that message is then
/// clap's own. When the second parse does not give the same kind of error,
/// every control or direction character of the first message is escaped.
fn safe_usage_error(err: &clap::Error) -> String {
    let rendered = err.render().to_string();
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let dangerous = |arg: &OsString| arg.to_string_lossy().chars().any(is_raw_danger);
    if !args.iter().any(dangerous) {
        return escape_all(&rendered, false);
    }
    let escaped = std::iter::once(OsString::from("cur-top")).chain(args.iter().map(|arg| {
        if dangerous(arg) {
            OsString::from(escape_raw(&arg.to_string_lossy()))
        } else {
            arg.clone()
        }
    }));
    match Cli::try_parse_from(escaped) {
        Err(second) if second.kind() == err.kind() => {
            escape_all(&second.render().to_string(), false)
        }
        // clap does not echo an argument in this message, so its own line
        // breaks stay.
        _ if err.kind() == ErrorKind::InvalidUtf8 => escape_all(&rendered, false),
        _ => escape_all(&rendered, true),
    }
}

/// Escape every dangerous character; a line break only when `lines` is set.
fn escape_all(text: &str, lines: bool) -> String {
    text.chars()
        .map(|c| {
            if is_raw_danger(c) && (lines || c != '\n') {
                escape_raw(&c.to_string())
            } else {
                c.to_string()
            }
        })
        .collect()
}

fn write_stdout(text: &str) -> ExitCode {
    let mut out = stdout().lock();
    // Rust treats a closed stdout (`>&-`) as a successful write, so
    // check that descriptor 1 is open before trusting the write.
    if !stdout_is_open() || out.write_all(text.as_bytes()).is_err() || out.flush().is_err() {
        let _ = writeln!(std::io::stderr(), "cur-top: could not write output");
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

/// False only when fd 1 was already closed at process start (`>&-`).
/// `1<>/dev/null` and `>/dev/null` stay open. Missing `/proc` stays open.
fn stdout_is_open() -> bool {
    !stdout_probe::stdout_was_closed_at_start()
}

/// Do both paths name one existing file? A path that does not exist yet is
/// never the same as an input.
fn same_file(output: &std::path::Path, input: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let (Ok(a), Ok(b)) = (std::fs::metadata(output), std::fs::metadata(input)) {
            if a.dev() == b.dev() && a.ino() == b.ino() {
                return true;
            }
        }
    }
    match (std::fs::canonicalize(output), std::fs::canonicalize(input)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Write the report to `path` whole or not at all: the bytes go to a new
/// file in the same directory, which then replaces `path` by a rename. An
/// existing file at `path` is replaced; a symbolic link at `path` is replaced
/// as a link and never written through. A path that exists and is a
/// special file (a pipe, a socket, a device) is refused. A missing directory is not created.
fn write_file(path: &std::path::Path, text: &str) -> ExitCode {
    if stop_requested() {
        return interrupted();
    }
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if !meta.is_file() && !meta.is_symlink() && !meta.is_dir() {
            let err = std::io::Error::other("not a regular file");
            return report_file_error(path, &err);
        }
    }
    let dir = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let tmp = match write_temp(&dir, path, text) {
        Ok(tmp) => tmp,
        Err(err) => return report_file_error(path, &err),
    };
    // After the rename the report is complete, so a later SIGINT cannot cut it.
    if stop_requested() {
        let _ = std::fs::remove_file(&tmp);
        return interrupted();
    }
    match std::fs::rename(&tmp, path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            let _ = std::fs::remove_file(&tmp);
            report_file_error(path, &err)
        }
    }
}

/// A new file next to `path` that holds `text` and is flushed to disk.
/// `create_new` refuses an existing name and a symbolic link, so nothing
/// that already exists is written through.
fn write_temp(
    dir: &std::path::Path,
    path: &std::path::Path,
    text: &str,
) -> std::io::Result<PathBuf> {
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::other("not a file name"))?;
    let mut last = None;
    for n in 0..100u32 {
        let mut tmp_name = OsString::from(".");
        tmp_name.push(name);
        tmp_name.push(format!(".cur-top-{}-{n}.tmp", std::process::id()));
        let tmp = dir.join(tmp_name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(mut file) => {
                let written = file
                    .write_all(text.as_bytes())
                    .and_then(|()| file.flush())
                    .and_then(|()| file.sync_all());
                return match written {
                    Ok(()) => Ok(tmp),
                    Err(err) => {
                        let _ = std::fs::remove_file(&tmp);
                        Err(err)
                    }
                };
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => last = Some(err),
            Err(err) => return Err(err),
        }
    }
    Err(last.unwrap_or_else(|| std::io::Error::other("no free temporary name")))
}

fn report_file_error(path: &std::path::Path, err: &std::io::Error) -> ExitCode {
    if stop_requested() {
        return interrupted();
    }
    let _ = writeln!(
        std::io::stderr(),
        "cur-top: could not write report to {}: {}",
        escape_raw(&path.to_string_lossy()),
        escape_raw(&err.to_string())
    );
    ExitCode::from(2)
}
