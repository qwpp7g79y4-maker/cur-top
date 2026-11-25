use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};

use rust_decimal::Decimal;

use crate::columns::{
    parse_amount_column, BILL_BILLING_PERIOD_START_DATE, LINE_ITEM_PRODUCT_CODE,
    LINE_ITEM_USAGE_TYPE, RESOURCE_TAGS,
};
use crate::error::Error;
use crate::money::{format_amount, parse_amount, parse_billing_start, YearMonth};
use crate::tags::parse_resource_tags;

/// One ranked key in the current month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub key: String,
    pub amount: String,
    pub previous: String,
    pub delta: String,
}

/// Top costs for the latest billing month in the input, plus the previous month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub amount_column: String,
    pub month: Option<String>,
    pub previous_month: Option<String>,
    pub previous_month_present: bool,
    pub rows: u64,
    pub rows_without_amount: u64,
    pub rows_without_tags: u64,
    pub rows_outside_months: u64,
    pub service: Vec<Line>,
    pub usage_type: Vec<Line>,
    pub tag: Vec<Line>,
    pub total_amount: String,
    pub total_previous: String,
    pub total_delta: String,
}

/// Columns, top limit, and output shape shared by the library and the CLI.
#[derive(Debug, Clone)]
pub struct Options {
    pub amount_column: String,
    pub top: usize,
    pub json: bool,
}

type Sums = HashMap<String, HashMap<YearMonth, Slot>>;

/// Exact sum for one key and month. The positive and the negative parts are
/// kept apart and are wider than `Decimal`, so the order of the rows does not
/// decide whether the sum fits.
#[derive(Default)]
struct Slot {
    positive: Vec<u32>,
    negative: Vec<u32>,
    scale: u32,
}

impl Slot {
    fn add(&mut self, amount: Decimal) {
        if amount.scale() > self.scale {
            let grow = amount.scale() - self.scale;
            mul_pow10(&mut self.positive, grow);
            mul_pow10(&mut self.negative, grow);
            self.scale = amount.scale();
        }
        let mut magnitude = limbs_from_u128(amount.mantissa().unsigned_abs());
        mul_pow10(&mut magnitude, self.scale - amount.scale());
        if amount.is_sign_negative() {
            self.negative = add_mag(&self.negative, &magnitude);
        } else {
            self.positive = add_mag(&self.positive, &magnitude);
        }
    }

    fn value(&self) -> Result<Decimal, Error> {
        let (limbs, negative) = match cmp_mag(&self.positive, &self.negative) {
            Ordering::Greater => (sub_mag(&self.positive, &self.negative), false),
            Ordering::Less => (sub_mag(&self.negative, &self.positive), true),
            Ordering::Equal => (Vec::new(), false),
        };
        limbs_to_decimal(limbs, negative, self.scale)
    }
}

#[derive(Default)]
pub(crate) struct Acc {
    service: Sums,
    usage: Sums,
    tag: Sums,
    /// Rows per month, also rows without an amount.
    month_rows: HashMap<YearMonth, u64>,
    /// Months with at least one row, also a row without an amount.
    months_seen: HashSet<YearMonth>,
    /// Latest month in any row, also a row without an amount.
    latest_month: Option<YearMonth>,
    rows: u64,
    rows_without_amount: u64,
    rows_without_tags: u64,
}

pub(crate) fn check_options(opts: &Options) -> Result<&'static str, Error> {
    let column = parse_amount_column(&opts.amount_column)?;
    if opts.top == 0 || opts.top > 10_000 {
        return Err(Error::new("top must be from 1 to 10000"));
    }
    Ok(column)
}

/// Sum in-memory CSV documents. Each slice is one CUR file.
pub fn summarize_slices(parts: &[&[u8]], opts: &Options) -> Result<Summary, Error> {
    let column = check_options(opts)?;
    let mut acc = Acc::default();
    for (i, part) in parts.iter().enumerate() {
        acc.ingest(Cursor::new(*part), column, i + 1, u64::MAX)?;
    }
    acc.finish(column, opts.top)
}

impl Acc {
    pub(crate) fn ingest<R: Read>(
        &mut self,
        reader: R,
        amount_column: &str,
        file_no: usize,
        cap: u64,
    ) -> Result<(), Error> {
        self.ingest_inner(Bom::new(Cap::new(reader, cap)), amount_column, cap)
            .map_err(|err| Error::new(format!("file {file_no}: {err}")))
    }

    fn ingest_inner<R: Read>(
        &mut self,
        reader: R,
        amount_column: &str,
        cap: u64,
    ) -> Result<(), Error> {
        let mut rdr = csv::ReaderBuilder::new()
            .trim(csv::Trim::None)
            .flexible(false)
            .from_reader(reader);
        let headers = rdr.headers().map_err(|err| map_csv(err, cap))?.clone();
        let index = header_index(&headers)?;
        require(&index, amount_column)?;
        let mut file_rows = 0u64;
        for rec in rdr.records() {
            let rec = rec.map_err(|err| map_csv(err, cap))?;
            file_rows = file_rows
                .checked_add(1)
                .ok_or_else(|| Error::new("row total does not fit"))?;
            let row = file_rows;
            let month = parse_billing_start(field(&rec, &index, BILL_BILLING_PERIOD_START_DATE)?)
                .map_err(|err| at_row(row, err))?;
            let product = parse_label(
                field(&rec, &index, LINE_ITEM_PRODUCT_CODE)?,
                LINE_ITEM_PRODUCT_CODE,
            )
            .map_err(|err| at_row(row, err))?;
            let usage = parse_label(
                field(&rec, &index, LINE_ITEM_USAGE_TYPE)?,
                LINE_ITEM_USAGE_TYPE,
            )
            .map_err(|err| at_row(row, err))?;
            let amount = parse_amount(field(&rec, &index, amount_column)?)
                .map_err(|err| at_row(row, err))?;
            let tags = parse_resource_tags(field(&rec, &index, RESOURCE_TAGS)?)
                .map_err(|err| at_row(row, err))?;
            self.push_row(month, product, usage, amount, tags)
                .map_err(|err| at_row(row, err))?;
        }
        Ok(())
    }

    fn push_row(
        &mut self,
        month: YearMonth,
        product: String,
        usage: String,
        amount: Option<Decimal>,
        tags: Vec<(String, String)>,
    ) -> Result<(), Error> {
        self.rows = self
            .rows
            .checked_add(1)
            .ok_or_else(|| Error::new("row total does not fit"))?;
        if tags.is_empty() {
            self.rows_without_tags = self
                .rows_without_tags
                .checked_add(1)
                .ok_or_else(|| Error::new("row total does not fit"))?;
        }
        self.latest_month = self.latest_month.max(Some(month));
        self.months_seen.insert(month);
        let count = self.month_rows.entry(month).or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| Error::new("row total does not fit"))?;
        let Some(amount) = amount else {
            self.rows_without_amount = self
                .rows_without_amount
                .checked_add(1)
                .ok_or_else(|| Error::new("row total does not fit"))?;
            return Ok(());
        };
        add_into(&mut self.service, product, month, amount);
        add_into(&mut self.usage, usage, month, amount);
        for (key, value) in tags {
            // A key may contain `=`, so escape it: `a=b`/`c` and `a`/`b=c`
            // must stay two different lines.
            let key = key.replace('\\', "\\\\").replace('=', "\\=");
            // A backslash in the value is doubled too, so the value `\u{20}x`
            // and the value ` x` do not give the same line.
            let value = value.replace('\\', "\\\\");
            add_into(&mut self.tag, format!("{key}={value}"), month, amount);
        }
        Ok(())
    }

    pub(crate) fn finish(&self, amount_column: &str, top: usize) -> Result<Summary, Error> {
        let current = self.latest_month;
        let mut rows_outside = 0u64;
        if let Some(month) = current {
            let previous = month.previous();
            for (seen, count) in &self.month_rows {
                if *seen != month && Some(*seen) != previous {
                    rows_outside = rows_outside
                        .checked_add(*count)
                        .ok_or_else(|| Error::new("row total does not fit"))?;
                }
            }
        }
        let mut month = None;
        let mut previous_month = None;
        let mut previous_present = false;
        let mut service = Vec::new();
        let mut usage_type = Vec::new();
        let mut tag = Vec::new();
        let mut total_amount = Decimal::ZERO;
        let mut total_previous = Decimal::ZERO;
        if let Some(current) = current {
            let previous = current.previous();
            previous_present = previous.is_some_and(|p| self.months_seen.contains(&p));
            month = Some(current.label());
            previous_month = previous.map(|p| p.label());
            total_amount = month_total(&self.service, current)?;
            if let (true, Some(previous)) = (previous_present, previous) {
                total_previous = month_total(&self.service, previous)?;
            }
            service = rank(&self.service, current, previous, previous_present, top)?;
            usage_type = rank(&self.usage, current, previous, previous_present, top)?;
            tag = rank(&self.tag, current, previous, previous_present, top)?;
        }
        let total_delta = exact_sub(total_amount, total_previous)?;
        Ok(Summary {
            amount_column: amount_column.to_string(),
            month,
            previous_month,
            previous_month_present: previous_present,
            rows: self.rows,
            rows_without_amount: self.rows_without_amount,
            rows_without_tags: self.rows_without_tags,
            rows_outside_months: rows_outside,
            service,
            usage_type,
            tag,
            total_amount: format_amount(total_amount),
            total_previous: format_amount(total_previous),
            total_delta: format_amount(total_delta),
        })
    }
}

fn add_into(map: &mut Sums, key: String, month: YearMonth, amount: Decimal) {
    map.entry(key)
        .or_default()
        .entry(month)
        .or_default()
        .add(amount);
}

fn month_total(map: &Sums, month: YearMonth) -> Result<Decimal, Error> {
    // Decimal's own checked_add rounds instead of failing near the limit, and
    // HashMap order changes between runs. The total is therefore always the
    // exact sum, and it either fits as it is or is an error.
    let mut amounts = Vec::new();
    for months in map.values() {
        if let Some(slot) = months.get(&month) {
            amounts.push(slot.value()?);
        }
    }
    sum_positive_and_negative(amounts.into_iter())
}

/// Exact `left - right`, or an error when the result does not fit unrounded.
fn exact_sub(left: Decimal, right: Decimal) -> Result<Decimal, Error> {
    sum_positive_and_negative([left, -right].into_iter())
}

/// Exact sum of the positive parts and the negative parts.
///
/// The accumulator is wider than `Decimal`, so a partial sum may sit outside
/// the decimal range. The total is kept only when it fits exactly.
fn sum_positive_and_negative(amounts: impl Iterator<Item = Decimal>) -> Result<Decimal, Error> {
    let amounts: Vec<Decimal> = amounts.filter(|amount| !amount.is_zero()).collect();
    let Some(scale) = amounts.iter().map(Decimal::scale).max() else {
        return Ok(Decimal::ZERO);
    };
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    for amount in amounts {
        let mut magnitude = limbs_from_u128(amount.mantissa().unsigned_abs());
        mul_pow10(&mut magnitude, scale - amount.scale());
        if amount.is_sign_negative() {
            negative = add_mag(&negative, &magnitude);
        } else {
            positive = add_mag(&positive, &magnitude);
        }
    }
    let (limbs, negative_total) = match cmp_mag(&positive, &negative) {
        Ordering::Greater => (sub_mag(&positive, &negative), false),
        Ordering::Less => (sub_mag(&negative, &positive), true),
        Ordering::Equal => (Vec::new(), false),
    };
    limbs_to_decimal(limbs, negative_total, scale)
}

fn limbs_from_u128(mut value: u128) -> Vec<u32> {
    let mut limbs = Vec::new();
    while value > 0 {
        limbs.push(value as u32);
        value >>= 32;
    }
    limbs
}

fn trim(limbs: &mut Vec<u32>) {
    while limbs.last().is_some_and(|limb| *limb == 0) {
        limbs.pop();
    }
}

fn add_mag(left: &[u32], right: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(left.len().max(right.len()).saturating_add(1));
    let mut carry = 0u64;
    for i in 0..left.len().max(right.len()) {
        let left_limb = left.get(i).copied().map_or(0, u64::from);
        let right_limb = right.get(i).copied().map_or(0, u64::from);
        let sum = left_limb + right_limb + carry;
        out.push(sum as u32);
        carry = sum >> 32;
    }
    if carry > 0 {
        out.push(carry as u32);
    }
    trim(&mut out);
    out
}

fn sub_mag(left: &[u32], right: &[u32]) -> Vec<u32> {
    let mut out = left.to_vec();
    let mut borrow = 0u64;
    for (i, slot) in out.iter_mut().enumerate() {
        let right_limb = right.get(i).copied().map_or(0, u64::from);
        let current = u64::from(*slot);
        if current >= right_limb + borrow {
            *slot = (current - right_limb - borrow) as u32;
            borrow = 0;
        } else {
            let base = current + (1u64 << 32);
            *slot = (base - right_limb - borrow) as u32;
            borrow = 1;
        }
    }
    debug_assert_eq!(borrow, 0);
    trim(&mut out);
    out
}

fn cmp_mag(left: &[u32], right: &[u32]) -> Ordering {
    left.len()
        .cmp(&right.len())
        .then_with(|| left.iter().rev().cmp(right.iter().rev()))
}

/// `10^0` through `10^9`. `10^9` is the largest power of ten that fits in `u32`.
const POW10: [u32; 10] = [
    1,
    10,
    100,
    1_000,
    10_000,
    100_000,
    1_000_000,
    10_000_000,
    100_000_000,
    1_000_000_000,
];

fn mul_pow10(limbs: &mut Vec<u32>, mut exponent: u32) {
    while exponent > 0 {
        let step = exponent.min(9);
        mul_small(limbs, POW10[step as usize]);
        exponent -= step;
    }
}

fn mul_small(limbs: &mut Vec<u32>, factor: u32) {
    if factor == 1 || limbs.is_empty() {
        return;
    }
    let mut carry = 0u64;
    for limb in limbs.iter_mut() {
        let product = u64::from(*limb) * u64::from(factor) + carry;
        *limb = product as u32;
        carry = product >> 32;
    }
    if carry > 0 {
        limbs.push(carry as u32);
    }
}

fn div_small(limbs: &mut Vec<u32>, divisor: u32) -> u32 {
    let mut remainder = 0u64;
    for limb in limbs.iter_mut().rev() {
        let current = (remainder << 32) | u64::from(*limb);
        *limb = (current / u64::from(divisor)) as u32;
        remainder = current % u64::from(divisor);
    }
    trim(limbs);
    remainder as u32
}

fn limbs_to_decimal(mut limbs: Vec<u32>, negative: bool, mut scale: u32) -> Result<Decimal, Error> {
    loop {
        trim(&mut limbs);
        if scale <= Decimal::MAX_SCALE && limbs.len() <= 3 {
            let mut magnitude: i128 = 0;
            for (i, limb) in limbs.iter().enumerate() {
                magnitude |= i128::from(*limb) << (32 * i);
            }
            if negative {
                magnitude = -magnitude;
            }
            return Decimal::try_from_i128_with_scale(magnitude, scale)
                .map_err(|_| Error::new("amount total does not fit"));
        }
        if scale == 0 {
            return Err(Error::new("amount total does not fit"));
        }
        if div_small(&mut limbs, 10) != 0 {
            return Err(Error::new("amount total does not fit"));
        }
        scale -= 1;
    }
}

fn rank(
    map: &Sums,
    current: YearMonth,
    previous: Option<YearMonth>,
    previous_present: bool,
    top: usize,
) -> Result<Vec<Line>, Error> {
    let mut rows = Vec::new();
    for (key, months) in map {
        let amount = match months.get(&current) {
            Some(slot) => slot.value()?,
            None => Decimal::ZERO,
        };
        if amount.is_zero() {
            continue;
        }
        let prev = if let (true, Some(previous)) = (previous_present, previous) {
            match months.get(&previous) {
                Some(slot) => slot.value()?,
                None => Decimal::ZERO,
            }
        } else {
            Decimal::ZERO
        };
        rows.push((key.clone(), amount, prev));
    }
    rows.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    rows.truncate(top);
    let mut lines = Vec::with_capacity(rows.len());
    for (key, amount, prev) in rows {
        let delta = exact_sub(amount, prev)?;
        lines.push(Line {
            key,
            amount: format_amount(amount),
            previous: format_amount(prev),
            delta: format_amount(delta),
        });
    }
    Ok(lines)
}

fn header_index(headers: &csv::StringRecord) -> Result<HashMap<String, usize>, Error> {
    let mut index = HashMap::new();
    for (i, name) in headers.iter().enumerate() {
        if index.insert(name.to_string(), i).is_some() {
            return Err(Error::new("duplicate column"));
        }
    }
    Ok(index)
}

fn require(index: &HashMap<String, usize>, amount_column: &str) -> Result<(), Error> {
    for name in [
        BILL_BILLING_PERIOD_START_DATE,
        LINE_ITEM_PRODUCT_CODE,
        LINE_ITEM_USAGE_TYPE,
        RESOURCE_TAGS,
        amount_column,
    ] {
        if !index.contains_key(name) {
            return Err(Error::new(format!("missing column {name}")));
        }
    }
    Ok(())
}

fn field<'a>(
    rec: &'a csv::StringRecord,
    index: &HashMap<String, usize>,
    name: &str,
) -> Result<&'a str, Error> {
    let Some(i) = index.get(name) else {
        return Err(Error::new(format!("missing column {name}")));
    };
    rec.get(*i).ok_or_else(|| Error::new("wrong column count"))
}

fn parse_label(raw: &str, column: &str) -> Result<String, Error> {
    let Some(text) = crate::tags::trim_ascii_edges(raw) else {
        return Err(Error::new(format!("invalid {column}")));
    };
    if text.is_empty() || text.len() > 1024 || text.chars().any(|c| c.is_control()) {
        return Err(Error::new(format!("invalid {column}")));
    }
    Ok(text.to_string())
}

fn at_row(row: u64, err: Error) -> Error {
    Error::new(format!("row {row}: {}", err.message()))
}

fn map_csv(err: csv::Error, cap: u64) -> Error {
    match err.kind() {
        csv::ErrorKind::Utf8 { .. } => Error::new("not UTF-8"),
        csv::ErrorKind::UnequalLengths { pos, .. } => {
            // `record()` counts records (the header is 0), like `invalid
            // amount` does; `line()` would count a multi-line cell as lines.
            let row = pos.as_ref().map(|pos| pos.record()).unwrap_or(1);
            Error::new(format!("row {row}: wrong column count"))
        }
        csv::ErrorKind::Io(io) => map_io(io, cap),
        _ => Error::new("not CSV"),
    }
}

/// Digits after the internal gzip-total marker, if this text is that error.
fn gzip_compressed_cap(text: &str) -> Option<&str> {
    let rest = text.split_once("cur-top-gzip-total:")?.1;
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let limit = &rest[..end];
    if limit.is_empty() {
        None
    } else {
        Some(limit)
    }
}

fn map_io(err: &std::io::Error, cap: u64) -> Error {
    let text = err.to_string();
    if text.contains("cur-top-cap") {
        Error::new(format!("file is larger than {cap} bytes"))
    } else if text.contains("cur-top-gzip-overhead") {
        Error::new(format!(
            "gzip overhead is larger than {} bytes",
            crate::MAX_GZIP_OVERHEAD_BYTES
        ))
    } else if let Some(limit) = gzip_compressed_cap(&text) {
        Error::new(format!("gzip input is larger than {limit} bytes"))
    } else if err.kind() == std::io::ErrorKind::InvalidData {
        Error::new("not gzip")
    } else {
        Error::new("could not read file")
    }
}

/// Drops a leading UTF-8 BOM. csv 1.4 does not do this itself.
struct Bom<R> {
    inner: R,
    pending: Vec<u8>,
    started: bool,
}

impl<R> Bom<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            pending: Vec::new(),
            started: false,
        }
    }
}

impl<R: Read> Read for Bom<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if !self.started {
            self.started = true;
            let mut head = [0u8; 3];
            let mut filled = 0;
            while filled < 3 {
                let n = self.inner.read(&mut head[filled..])?;
                if n == 0 {
                    break;
                }
                filled += n;
            }
            if head[..filled].starts_with(&[0xEF, 0xBB, 0xBF]) {
                self.pending.extend_from_slice(&head[3..filled]);
            } else {
                self.pending.extend_from_slice(&head[..filled]);
            }
        }
        if !self.pending.is_empty() {
            let n = buf.len().min(self.pending.len());
            buf[..n].copy_from_slice(&self.pending[..n]);
            self.pending.drain(..n);
            return Ok(n);
        }
        self.inner.read(buf)
    }
}

struct Cap<R> {
    inner: R,
    read: u64,
    cap: u64,
}

impl<R> Cap<R> {
    fn new(inner: R, cap: u64) -> Self {
        Self {
            inner,
            read: 0,
            cap,
        }
    }
}

impl<R: Read> Read for Cap<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.read >= self.cap {
            let mut probe = [0u8; 1];
            let n = self.inner.read(&mut probe)?;
            if n == 0 {
                return Ok(0);
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "cur-top-cap",
            ));
        }
        let room = usize::try_from(self.cap - self.read).unwrap_or(usize::MAX);
        let want = buf.len().min(room);
        let n = self.inner.read(&mut buf[..want])?;
        self.read += u64::try_from(n).unwrap_or(u64::MAX);
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_label, summarize_slices, Options};
    use crate::render;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    const HEADER: &str = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n";

    #[test]
    fn label_unicode_white_at_the_edge_is_an_error() {
        for ws in ['\u{a0}', '\u{3000}', '\u{2028}'] {
            for raw in [format!("{ws}A"), format!("A{ws}")] {
                let err = parse_label(&raw, "product").unwrap_err();
                assert_eq!(err.message(), "invalid product");
            }
        }
        assert_eq!(parse_label(" \t\r\nA\n ", "product").unwrap(), "A");
    }

    fn options(column: &str, top: usize) -> Options {
        Options {
            amount_column: column.to_string(),
            top,
            json: false,
        }
    }

    fn finish(csv: &str) -> super::Summary {
        summarize_slices(&[csv.as_bytes()], &options("line_item_unblended_cost", 10)).unwrap()
    }

    fn finish_err(csv: &str) -> String {
        summarize_slices(&[csv.as_bytes()], &options("line_item_unblended_cost", 10))
            .unwrap_err()
            .message()
            .to_string()
    }

    #[test]
    fn claim_c1_unblended_is_not_rate_times_usage() {
        let rate = Decimal::from_str("0.0104").unwrap();
        let usage = Decimal::from(336);
        assert_ne!(rate * usage, Decimal::from_str("3.50").unwrap());
        let summary = finish(include_str!("../examples/cur.csv"));
        assert_eq!(summary.total_amount, "4.50");
        assert_eq!(summary.total_previous, "1.00");
        assert_eq!(summary.service[0].key, "AmazonEC2");
        assert_eq!(summary.service[0].amount, "3.50");
        assert_eq!(summary.rows, 3);
    }

    #[test]
    fn year_zero_january_is_a_valid_first_month() {
        let csv = format!("{HEADER}0000-01-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,{{}}\n");
        let summary = finish(&csv);
        assert_eq!(summary.month.as_deref(), Some("0000-01"));
        assert_eq!(summary.previous_month, None);
        assert!(!summary.previous_month_present);
        assert_eq!(summary.total_amount, "1.00");
        assert_eq!(summary.total_previous, "0.00");
        assert_eq!(summary.service[0].key, "AmazonEC2");
        assert_eq!(summary.service[0].previous, "0.00");
    }

    #[test]
    fn row_count_at_u64_max_reports_row_total_does_not_fit() {
        let csv = format!("{HEADER}2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,{{}}\n");
        let mut acc = super::Acc {
            rows: u64::MAX,
            ..Default::default()
        };
        let err = acc
            .ingest(csv.as_bytes(), "line_item_unblended_cost", 1, u64::MAX)
            .unwrap_err();
        assert!(
            err.message().contains("row total does not fit"),
            "{}",
            err.message()
        );
    }

    #[test]
    fn read_failure_reports_could_not_read_file() {
        struct Failing;
        impl std::io::Read for Failing {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            }
        }
        let mut acc = super::Acc::default();
        let err = acc
            .ingest(Failing, "line_item_unblended_cost", 1, u64::MAX)
            .unwrap_err();
        assert!(
            err.message().contains("could not read file"),
            "{}",
            err.message()
        );
    }

    #[test]
    fn claim_c4_latest_month_and_calendar_previous() {
        let csv = format!(
            "{HEADER}2026-01-15T01:02:03Z,AmazonEC2,BoxUsage:t3.micro,2.00,{{}}\n2025-12-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,{{}}\n"
        );
        let summary = finish(&csv);
        assert_eq!(summary.month.as_deref(), Some("2026-01"));
        assert_eq!(summary.previous_month.as_deref(), Some("2025-12"));
        assert!(summary.previous_month_present);
        assert_eq!(summary.total_amount, "2.00");
        assert_eq!(summary.total_previous, "1.00");
        assert_eq!(summary.rows_outside_months, 0);
    }

    #[test]
    fn claim_c4_latest_month_with_only_blank_amounts_stays_current() {
        let csv = format!(
            "{HEADER}2026-10-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,4.00,{{}}\n2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,,{{}}\n"
        );
        let summary = finish(&csv);
        assert_eq!(summary.month.as_deref(), Some("2026-11"));
        assert_eq!(summary.previous_month.as_deref(), Some("2026-10"));
        assert!(summary.previous_month_present);
        assert_eq!(summary.total_amount, "0.00");
        assert_eq!(summary.total_previous, "4.00");
        assert_eq!(summary.rows_without_amount, 1);

        // A previous month with rows but only blank amounts is still present.
        let csv = format!(
            "{HEADER}2026-10-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,,{{}}\n2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,2.00,{{}}\n"
        );
        let summary = finish(&csv);
        assert_eq!(summary.previous_month.as_deref(), Some("2026-10"));
        assert!(summary.previous_month_present);
        assert_eq!(summary.total_previous, "0.00");
        assert_eq!(summary.total_amount, "2.00");
        assert_eq!(summary.rows_without_amount, 1);
        assert_eq!(summary.rows_outside_months, 0);

        // A blank-amount row outside the two months still counts as outside.
        let csv = format!(
            "{HEADER}2026-09-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,,{{}}\n2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,2.00,{{}}\n"
        );
        let summary = finish(&csv);
        assert_eq!(summary.rows_without_amount, 1);
        assert_eq!(summary.rows_outside_months, 1);
    }

    #[test]
    fn claim_c5_delta_is_current_minus_previous() {
        let summary = finish(include_str!("../examples/cur.csv"));
        assert_eq!(summary.service[0].delta, "2.50");
        assert_eq!(summary.total_delta, "3.50");
        let csv = format!(
            "{HEADER}2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,{{}}\n2026-10-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,4.00,{{}}\n"
        );
        let summary = finish(&csv);
        assert_eq!(summary.total_delta, "-3.00");
        assert_eq!(summary.service[0].delta, "-3.00");
    }

    #[test]
    fn claim_c6_blank_amount_is_skipped_without_the_cell() {
        let csv = format!(
            "{HEADER}2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,,{{}}\n2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,{{}}\n"
        );
        let summary = finish(&csv);
        assert_eq!(summary.rows, 2);
        assert_eq!(summary.rows_without_amount, 1);
        assert_eq!(summary.total_amount, "1.00");
        let err = finish_err(&format!(
            "{HEADER}2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,secret,{{}}\n"
        ));
        assert!(err.contains("invalid amount"));
        assert!(!err.contains("secret"));
    }

    #[test]
    fn claim_c7_other_months_stay_out_of_the_totals() {
        let csv = format!(
            "{HEADER}2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,{{}}\n2026-09-01T00:00:00Z,AmazonRDS,RDS:Usage,9.00,{{}}\n"
        );
        let summary = finish(&csv);
        assert_eq!(summary.month.as_deref(), Some("2026-11"));
        assert_eq!(summary.total_amount, "1.00");
        assert_eq!(summary.total_previous, "0.00");
        assert_eq!(summary.rows_outside_months, 1);
        assert!(summary.service.iter().all(|line| line.key != "AmazonRDS"));
    }

    #[test]
    fn claim_c8_absent_previous_month_is_zero() {
        let csv =
            format!("{HEADER}2026-11-01T00:00:00.000Z,AmazonEC2,BoxUsage:t3.micro,2.00,{{}}\n");
        let summary = finish(&csv);
        assert_eq!(summary.previous_month.as_deref(), Some("2026-10"));
        assert!(!summary.previous_month_present);
        assert_eq!(summary.total_previous, "0.00");
        assert_eq!(summary.total_delta, "2.00");
        assert_eq!(summary.service[0].previous, "0.00");
    }

    #[test]
    fn claim_c9_negative_credit_lowers_the_total() {
        let csv = format!(
            "{HEADER}2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,2.00,{{}}\n2026-11-01T00:00:00Z,AmazonEC2,Credit, -1.50 ,{{}}\n"
        );
        let summary = finish(&csv);
        assert_eq!(summary.total_amount, "0.50");
        assert_eq!(summary.service[0].amount, "0.50");
    }

    #[test]
    fn claim_c10_the_same_row_in_two_files_counts_twice() {
        let one = format!("{HEADER}2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.25,{{}}\n");
        let summary = summarize_slices(
            &[one.as_bytes(), one.as_bytes()],
            &options("line_item_unblended_cost", 10),
        )
        .unwrap();
        assert_eq!(summary.rows, 2);
        assert_eq!(summary.total_amount, "2.50");
    }

    #[test]
    fn claim_c11_each_tag_receives_the_full_line_amount() {
        let csv = r#"bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags
2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,2.00,"{""user_team"":""platform"",""user_env"":""prod""}"
"#;
        let summary = finish(csv);
        assert_eq!(summary.service[0].amount, "2.00");
        assert_eq!(summary.tag.len(), 2);
        assert!(summary
            .tag
            .iter()
            .all(|line| line.amount == "2.00" && line.delta == "2.00"));
        let keys: Vec<&str> = summary.tag.iter().map(|line| line.key.as_str()).collect();
        assert!(keys.contains(&"user_team=platform"));
        assert!(keys.contains(&"user_env=prod"));
    }

    #[test]
    fn tags_whose_joined_text_is_equal_stay_two_lines() {
        let csv = r#"bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags
2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,"{""a"":""b=c""}"
2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,2.00,"{""a=b"":""c""}"
"#;
        let summary = finish(csv);
        assert_eq!(summary.tag.len(), 2);
        let line = |key: &str| summary.tag.iter().find(|line| line.key == key).unwrap();
        assert_eq!(line("a=b=c").amount, "1.00");
        assert_eq!(line("a\\=b=c").amount, "2.00");
    }

    #[test]
    fn claim_c12_empty_tags_still_count_for_the_service() {
        let csv = format!("{HEADER}2026-11-01T00:00:00Z,AmazonS3,TimedStorage-ByteHrs,1.25,\n");
        let summary = finish(&csv);
        assert_eq!(summary.total_amount, "1.25");
        assert_eq!(summary.service[0].key, "AmazonS3");
        assert!(summary.tag.is_empty());
        assert_eq!(summary.rows_without_tags, 1);
    }

    #[test]
    fn claim_c13_top_is_limited_and_totals_cover_both_months() {
        let summary = summarize_slices(
            &[include_str!("../examples/cur.csv").as_bytes()],
            &options("line_item_unblended_cost", 1),
        )
        .unwrap();
        assert_eq!(summary.service.len(), 1);
        assert_eq!(summary.service[0].key, "AmazonEC2");
        assert_eq!(summary.total_amount, "4.50");
        assert_eq!(summary.usage_type.len(), 1);
        assert_eq!(summary.usage_type[0].key, "BoxUsage:t3.micro");
        let csv = format!(
            "{HEADER}2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,{{}}\n2026-10-01T00:00:00Z,AmazonRDS,RDS:Usage,9.00,{{}}\n"
        );
        let summary = finish(&csv);
        assert_eq!(summary.total_previous, "9.00");
        assert!(summary.service.iter().all(|line| line.key != "AmazonRDS"));
        assert_eq!(summary.service[0].key, "AmazonEC2");
        assert_eq!(summary.service[0].previous, "0.00");
    }

    #[test]
    fn claim_c15_account_id_is_not_reported() {
        let csv = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags,line_item_usage_account_id,bill_payer_account_id\n2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,{},123456789012,123456789012\n";
        let summary = finish(csv);
        let text = render(&summary, false);
        let json = render(&summary, true);
        assert!(!text.contains("123456789012"));
        assert!(!json.contains("123456789012"));
        assert_eq!(summary.total_amount, "1.00");
    }

    #[test]
    fn claim_c17_invalid_cells_hide_the_value() {
        let cases = [
            (
                format!("{HEADER}secret,AmazonEC2,BoxUsage:t3.micro,1.00,{{}}\n"),
                "invalid bill_billing_period_start_date",
            ),
            (
                format!("{HEADER}2026-11-01T00:00:00Z,,BoxUsage:t3.micro,1.00,{{}}\n"),
                "invalid line_item_product_code",
            ),
            (
                format!("{HEADER}2026-11-01T00:00:00Z,AmazonEC2,,1.00,{{}}\n"),
                "invalid line_item_usage_type",
            ),
            (
                format!("{HEADER}2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,secret,{{}}\n"),
                "invalid amount",
            ),
            (
                format!(
                    "{HEADER}2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,{{\"secret\":1}}\n"
                ),
                "invalid resource_tags",
            ),
        ];
        for (csv, needle) in cases {
            let err = finish_err(&csv);
            assert!(err.contains(needle), "{err}");
            assert!(!err.contains("secret"), "{err}");
        }
    }

    #[test]
    fn claim_c18_text_has_no_exponent_and_two_digits() {
        let csv = format!(
            "{HEADER}2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,0.00000123,{{}}\n2026-11-01T00:00:00Z,AmazonS3,TimedStorage-ByteHrs,2,{{}}\n"
        );
        let summary = finish(&csv);
        assert_eq!(summary.total_amount, "2.00000123");
        assert_eq!(summary.service[0].amount, "2.00");
        for line in summary
            .service
            .iter()
            .chain(&summary.usage_type)
            .chain(&summary.tag)
        {
            for amount in [&line.amount, &line.previous, &line.delta] {
                assert!(!amount.contains(['e', 'E']), "{amount}");
            }
        }
        assert!(!summary.total_amount.contains(['e', 'E']));
        assert!(!summary.total_delta.contains(['e', 'E']));
    }

    #[test]
    fn hourly_stamp_uses_the_calendar_month() {
        let csv = format!("{HEADER}2026-11-09T18:30:00Z,AmazonEC2,BoxUsage:t3.micro,0.10,{{}}\n");
        let summary = finish(&csv);
        assert_eq!(summary.month.as_deref(), Some("2026-11"));
        assert_eq!(summary.total_amount, "0.10");
    }

    #[test]
    fn utf8_bom_is_accepted() {
        let mut csv = vec![0xEF, 0xBB, 0xBF];
        csv.extend(
            format!("{HEADER}2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,0.20,{{}}\n")
                .into_bytes(),
        );
        let summary = summarize_slices(&[&csv], &options("line_item_unblended_cost", 10)).unwrap();
        assert_eq!(summary.total_amount, "0.20");
        assert_eq!(summary.month.as_deref(), Some("2026-11"));
    }

    #[test]
    fn header_only_is_a_zero_report() {
        let summary = finish(HEADER);
        assert_eq!(summary.rows, 0);
        assert!(summary.month.is_none());
        assert!(summary.previous_month.is_none());
        assert!(!summary.previous_month_present);
        assert_eq!(summary.total_amount, "0.00");
        assert_eq!(summary.total_delta, "0.00");
        assert!(summary.service.is_empty());
    }

    #[test]
    fn missing_column_names_the_official_column() {
        let csv = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost\n";
        let err = finish_err(csv);
        assert_eq!(err, "file 1: missing column resource_tags");
    }

    #[test]
    fn duplicate_column_is_hidden() {
        let csv = "bill_billing_period_start_date,line_item_product_code,line_item_product_code,line_item_usage_type,line_item_unblended_cost,resource_tags\n";
        assert_eq!(finish_err(csv), "file 1: duplicate column");
    }

    #[test]
    fn empty_product_code_names_the_column_and_the_row() {
        let csv = format!(
            "{HEADER}2026-11-01T00:00:00Z,AmazonS3,u,1.00,{{}}\n2026-11-01T00:00:00Z, ,u,1.00,{{}}\n"
        );
        assert_eq!(
            finish_err(&csv),
            "file 1: row 2: invalid line_item_product_code"
        );
    }

    #[test]
    fn bad_usage_type_names_the_column() {
        let control = format!("{HEADER}2026-11-01T00:00:00Z,AmazonS3,\"a\x07b\",1.00,{{}}\n");
        assert_eq!(
            finish_err(&control),
            "file 1: row 1: invalid line_item_usage_type"
        );
        let long = format!(
            "{HEADER}2026-11-01T00:00:00Z,AmazonS3,{},1.00,{{}}\n",
            "u".repeat(1025)
        );
        assert_eq!(
            finish_err(&long),
            "file 1: row 1: invalid line_item_usage_type"
        );
    }

    #[test]
    fn wrong_column_count_hides_the_cell() {
        let csv = format!("{HEADER}2026-11-01T00:00:00Z,secret\n");
        let err = finish_err(&csv);
        assert!(err.contains("wrong column count"), "{err}");
        assert!(!err.contains("secret"), "{err}");
    }

    #[test]
    fn month_total_fits_when_only_the_final_sum_fits() {
        let max = "79228162514264337593543950335";
        // Keys sort a, m, z, so the running total adds both maxima first.
        let fit = format!(
            "{HEADER}2026-11-01T00:00:00Z,a,ua,{max},{{}}\n2026-11-01T00:00:00Z,m,um,{max},{{}}\n2026-11-01T00:00:00Z,z,uz,-{max},{{}}\n"
        );
        let summary = finish(&fit);
        assert_eq!(finish(&fit), summary);
        assert_eq!(summary.rows, 3);
        assert_eq!(summary.total_amount, format!("{max}.00"));
        assert_eq!(summary.total_previous, "0.00");
        assert_eq!(summary.total_delta, format!("{max}.00"));
        // Same shape with a scale: the partial sum leaves the decimal range,
        // and the cents cancel, so the exact total is still the maximum.
        let scaled = format!(
            "{HEADER}2026-11-01T00:00:00Z,a,ua,{max},{{}}\n2026-11-01T00:00:00Z,m,um,1.50,{{}}\n2026-11-01T00:00:00Z,z,uz,-1.50,{{}}\n"
        );
        let scaled = finish(&scaled);
        assert_eq!(scaled.total_amount, format!("{max}.00"));
        let overflow = format!(
            "{HEADER}2026-11-01T00:00:00Z,a,ua,{max},{{}}\n2026-11-01T00:00:00Z,m,um,1,{{}}\n2026-11-01T00:00:00Z,z,uz,{max},{{}}\n"
        );
        assert_eq!(finish_err(&overflow), "amount total does not fit");
    }

    #[test]
    fn one_key_sum_fits_when_only_the_final_sum_fits() {
        let max = "79228162514264337593543950335";
        let row = |amount: &str| format!("2026-11-01T00:00:00Z,svc,use,{amount},{{}}\n");
        let neg = format!("-{max}");
        let fit = format!("{HEADER}{}{}{}", row(max), row(max), row(&neg));
        let summary = finish(&fit);
        assert_eq!(summary.total_amount, format!("{max}.00"));
        assert_eq!(summary.service[0].amount, format!("{max}.00"));
        assert_eq!(summary.usage_type[0].amount, format!("{max}.00"));
        let moved = format!("{HEADER}{}{}{}", row(&neg), row(max), row(max));
        assert_eq!(finish(&moved), summary);
        let overflow = format!("{HEADER}{}{}{}", row(max), row(max), row("1"));
        assert_eq!(finish_err(&overflow), "amount total does not fit");
    }

    #[test]
    fn month_total_is_exact_or_fails_for_fractions_at_the_limit() {
        let m = "79228162514264337593543950334";
        let row = |key: &str, amount: &str| format!("2026-11-01T00:00:00Z,{key},u,{amount},{{}}\n");
        let build = |rows: &[(&str, &str)]| {
            let mut csv = HEADER.to_string();
            for (key, amount) in rows {
                csv.push_str(&row(key, amount));
            }
            csv
        };
        let want = "79228162514264337593543950335.00";
        // r1 and its reversed key order give the same exact total.
        for rows in [
            [("a", m), ("b", "0.5"), ("c", "0.5")],
            [("a", "0.5"), ("b", "0.5"), ("c", m)],
            [("a", "0.5"), ("b", m), ("c", "0.5")],
        ] {
            assert_eq!(finish(&build(&rows)).total_amount, want);
        }
        // r5: exact MAX from four keys.
        let r5 = build(&[("a", m), ("b", "0.4"), ("c", "0.4"), ("d", "0.2")]);
        assert_eq!(finish(&r5).total_amount, want);
        // r7: exact MAX + 0.4 does not fit, whatever the key names are.
        let max = "79228162514264337593543950335";
        assert_eq!(
            finish_err(&build(&[("a", max), ("b", "0.4")])),
            "amount total does not fit"
        );
        assert_eq!(
            finish_err(&build(&[("a", "0.4"), ("b", max)])),
            "amount total does not fit"
        );
        // d1: the delta of 334 against -0.5 is 334.5 and does not fit.
        let d1 = format!(
            "{HEADER}2026-11-01T00:00:00Z,a,u,{m},{{}}\n2026-10-01T00:00:00Z,a,u,-0.5,{{}}\n"
        );
        assert_eq!(finish_err(&d1), "amount total does not fit");
    }

    #[test]
    fn tie_breaks_by_key() {
        let csv = format!(
            "{HEADER}2026-11-01T00:00:00Z,AmazonS3,TimedStorage-ByteHrs,1.00,{{}}\n2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,{{}}\n"
        );
        let summary = finish(&csv);
        assert_eq!(summary.service[0].key, "AmazonEC2");
        assert_eq!(summary.service[1].key, "AmazonS3");
    }

    #[test]
    fn blended_column_is_summed_when_selected() {
        let csv = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_blended_cost,line_item_unblended_cost,resource_tags\n2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,0.40,9.00,{}\n";
        let summary =
            summarize_slices(&[csv.as_bytes()], &options("line_item_blended_cost", 10)).unwrap();
        assert_eq!(summary.amount_column, "line_item_blended_cost");
        assert_eq!(summary.total_amount, "0.40");
    }

    #[test]
    fn net_unblended_column_is_summed_when_selected() {
        let csv = "bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_net_unblended_cost,resource_tags\n2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,0.70,{}\n";
        let summary = summarize_slices(
            &[csv.as_bytes()],
            &options("line_item_net_unblended_cost", 10),
        )
        .unwrap();
        assert_eq!(summary.total_amount, "0.70");
        assert_eq!(summary.amount_column, "line_item_net_unblended_cost");
    }

    #[test]
    fn top_accepts_10000_and_rejects_10001() {
        let ok = summarize_slices(&[], &options("line_item_unblended_cost", 10_000));
        assert!(ok.is_ok());
        let err = summarize_slices(&[], &options("line_item_unblended_cost", 10_001)).unwrap_err();
        assert_eq!(err.message(), "top must be from 1 to 10000");
    }

    #[test]
    fn product_code_and_usage_type_stop_at_1024_bytes() {
        let edge = "a".repeat(1024);
        let over = "a".repeat(1025);
        let row = |code: &str, usage: &str| {
            format!("{HEADER}2026-11-01T00:00:00Z,{code},{usage},1.00,{{}}\n")
        };
        let summary = finish(&row(&edge, &edge));
        assert_eq!(summary.service[0].key, edge);
        assert!(finish_err(&row(&over, "u")).contains("invalid line_item_product_code"));
        assert!(finish_err(&row("p", &over)).contains("invalid line_item_usage_type"));
    }

    #[test]
    fn invalid_amount_column_hides_the_name() {
        let err = summarize_slices(&[], &options("secret", 10)).unwrap_err();
        assert_eq!(err.message(), "invalid amount column");
        assert!(!err.message().contains("secret"));
        let err = summarize_slices(&[], &options("line_item_unblended_cost", 0)).unwrap_err();
        assert_eq!(err.message(), "top must be from 1 to 10000");
    }

    #[test]
    fn sample_text_matches_the_locked_report() {
        let summary = finish(include_str!("../examples/cur.csv"));
        let text = render(&summary, false);
        assert_eq!(text, SAMPLE_TEXT);
        assert!(!text.contains("123456789012"));
    }

    const SAMPLE_TEXT: &str = "\
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
";
}
