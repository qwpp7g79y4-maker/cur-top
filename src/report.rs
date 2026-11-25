use crate::columns::{DICTIONARY_DATE, DICTIONARY_URL};
use crate::summary::{Line, Summary};
use serde_json::{json, Value};

/// The `schema_version` of the JSON report. It changes when a field is
/// removed, renamed, or changes meaning; a new field does not change it.
const SCHEMA_VERSION: u32 = 1;

pub fn render(summary: &Summary, json_mode: bool) -> String {
    if json_mode {
        render_json(summary)
    } else {
        render_text(summary)
    }
}

fn label(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("omitted")
}

fn render_text(summary: &Summary) -> String {
    let mut out = String::new();
    out.push_str("cur-top\n");
    out.push_str(&format!("dictionary: {DICTIONARY_URL}\n"));
    out.push_str(&format!("read: {DICTIONARY_DATE}\n"));
    out.push_str(&format!("amount: {}\n", summary.amount_column));
    out.push_str(&format!("month: {}\n", label(&summary.month)));
    out.push_str(&format!("previous: {}\n", label(&summary.previous_month)));
    let present = if summary.previous_month_present {
        "yes"
    } else {
        "no"
    };
    out.push_str(&format!("previous_month_present: {present}\n"));
    out.push_str(&format!("rows: {}\n", summary.rows));
    out.push_str(&format!(
        "rows_without_amount: {}\n",
        summary.rows_without_amount
    ));
    out.push_str(&format!(
        "rows_without_tags: {}\n",
        summary.rows_without_tags
    ));
    out.push_str(&format!(
        "rows_outside_months: {}\n",
        summary.rows_outside_months
    ));
    out.push('\n');
    push_group(&mut out, "service", &summary.service, true);
    push_group(&mut out, "usage_type", &summary.usage_type, true);
    // Tag keys and values already have `\` written as `\\` (and keys `=` as
    // `\=`) when they are summed, so a second backslash pass would double them.
    push_group(&mut out, "tag", &summary.tag, false);
    out.push_str("totals\n");
    out.push_str(&format!(
        "amount={} previous={} delta={}\n",
        summary.total_amount, summary.total_previous, summary.total_delta
    ));
    out
}

fn push_group(out: &mut String, title: &str, lines: &[Line], escape_backslash: bool) {
    out.push_str(title);
    out.push('\n');
    for line in lines {
        out.push_str(&format!(
            "{} amount={} previous={} delta={}\n",
            safe_key(&line.key, escape_backslash),
            line.amount,
            line.previous,
            line.delta
        ));
    }
    out.push('\n');
}

/// Keys come from the input. In the text report a space, a control, or an
/// invisible or direction-changing character in a key becomes `\u{..}`, and
/// the `=` of the words `amount=`, `previous=` and `delta=` becomes `\=`, so a
/// key cannot pose as one of those fields or reorder the line. A backslash
/// becomes `\\` (when `escape_backslash` is set), so a key that literally
/// contains `\u{20}` stays apart from a key that contains a space.
fn safe_key(key: &str, escape_backslash: bool) -> String {
    let mut out = String::with_capacity(key.len());
    for c in key.chars() {
        let hidden = is_hidden(c);
        if hidden {
            out.push_str(&format!("\\u{{{:x}}}", c as u32));
        } else if c == '\\' && escape_backslash {
            out.push_str("\\\\");
        } else {
            out.push(c);
        }
    }
    for word in ["amount=", "previous=", "delta="] {
        if out.contains(word) {
            out = out.replace(word, &word.replace('=', "\\="));
        }
    }
    out
}

/// A character shown as `\u{..}`: white space, a control, or an invisible or
/// direction-changing character.
fn is_hidden(c: char) -> bool {
    c.is_whitespace()
        || c.is_control()
        || matches!(
            c,
            '\u{ad}'
                | '\u{61c}'
                | '\u{180e}'
                | '\u{200b}'..='\u{200f}'
                | '\u{2028}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{34f}'
                | '\u{115f}'
                | '\u{1160}'
                | '\u{17b4}'
                | '\u{17b5}'
                | '\u{180b}'..='\u{180d}'
                | '\u{2800}'
                | '\u{3164}'
                | '\u{fe00}'..='\u{fe0f}'
                | '\u{feff}'
                | '\u{ffa0}'
                | '\u{fff9}'..='\u{fffb}'
                | '\u{e0000}'..='\u{e007f}'
        )
}

/// Markdown report: one table per group, the same amounts as the text report.
pub fn render_markdown(summary: &Summary) -> String {
    let mut out = String::new();
    out.push_str("# cur-top\n\n");
    out.push_str(&format!(
        "Month {} against {} (amount column `{}`). {} rows read.\n\n",
        cell(label(&summary.month)),
        cell(label(&summary.previous_month)),
        summary.amount_column,
        summary.rows
    ));
    out.push_str("Not an estimate: the amounts are summed from the file.\n\n");
    md_group(&mut out, "Service", &summary.service);
    md_group(&mut out, "Usage type", &summary.usage_type);
    // Tag keys already have their backslashes written as `\\`.
    md_group(&mut out, "Tag", &summary.tag);
    out.push_str("## Totals\n\n| Amount | Previous | Delta |\n| ---: | ---: | ---: |\n");
    out.push_str(&format!(
        "| {} | {} | {} |\n",
        summary.total_amount, summary.total_previous, summary.total_delta
    ));
    out
}

fn md_group(out: &mut String, title: &str, lines: &[Line]) {
    out.push_str(&format!(
        "## {title}\n\n| Key | Amount | Previous | Delta |\n| --- | ---: | ---: | ---: |\n"
    ));
    for line in lines {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            cell(&line.key),
            line.amount,
            line.previous,
            line.delta
        ));
    }
    out.push('\n');
}

/// A table cell: `|` becomes `\|`, and a line break or other hidden character
/// becomes `\u{..}`, so a key cannot end its cell or row early. A backslash is always doubled. The Markdown
/// and HTML characters ``\ ` * _ [ ] ( ) < > & ! ~ @`` get a backslash, and so
/// does a `#` at the start, a `:` before `/`, and the `.` after `www`, so no
/// renderer reads a key as a link (also a bare `https://` or `www.` address),
/// an image, HTML, strikethrough, or a heading. A plain space stays a space.
fn cell(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    for (index, &c) in chars.iter().enumerate() {
        let www_dot = c == '.'
            && index >= 3
            && chars[index - 3..index]
                .iter()
                .collect::<String>()
                .eq_ignore_ascii_case("www");
        if c == '|' {
            out.push_str("\\|");
        } else if "`*_[]()<>&!~@".contains(c)
            || (c == '#' && index == 0)
            || (c == ':' && chars.get(index + 1) == Some(&'/'))
            || www_dot
        {
            out.push('\\');
            out.push(c);
        } else if c == '\\' {
            out.push_str("\\\\");
        } else if is_hidden(c) && c != ' ' {
            out.push_str(&format!("\\u{{{:x}}}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

fn line_json(lines: &[Line]) -> Value {
    Value::Array(
        lines
            .iter()
            .map(|line| {
                json!({
                    "key": line.key,
                    "amount": line.amount,
                    "previous": line.previous,
                    "delta": line.delta,
                })
            })
            .collect(),
    )
}

fn render_json(summary: &Summary) -> String {
    let value = json!({
        "schema_version": SCHEMA_VERSION,
        "dictionary": DICTIONARY_URL,
        "dictionary_date": DICTIONARY_DATE,
        "amount_column": summary.amount_column,
        "month": summary.month,
        "previous_month": summary.previous_month,
        "previous_month_present": summary.previous_month_present,
        "rows": summary.rows,
        "rows_without_amount": summary.rows_without_amount,
        "rows_without_tags": summary.rows_without_tags,
        "rows_outside_months": summary.rows_outside_months,
        "service": line_json(&summary.service),
        "usage_type": line_json(&summary.usage_type),
        "tag": line_json(&summary.tag),
        "total": {
            "amount": summary.total_amount,
            "previous": summary.total_previous,
            "delta": summary.total_delta,
        }
    });
    let mut text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string());
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}
