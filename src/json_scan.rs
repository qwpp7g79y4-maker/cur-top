use crate::error::Error;

/// Reject objects that repeat a key. A value that does not start with `{` is left to the JSON parser.
pub(crate) fn reject_duplicate_keys(text: &str) -> Result<(), Error> {
    let bytes = text.as_bytes();
    let mut i = 0;
    skip_ws(bytes, &mut i);
    if i >= bytes.len() || bytes[i] != b'{' {
        return Ok(());
    }
    let mut depth = 0u32;
    scan_value(bytes, &mut i, &mut depth)?;
    Ok(())
}

fn scan_value(bytes: &[u8], i: &mut usize, depth: &mut u32) -> Result<(), Error> {
    skip_ws(bytes, i);
    if *i >= bytes.len() {
        return Err(Error::new("not JSON"));
    }
    match bytes[*i] {
        b'{' => scan_object(bytes, i, depth),
        b'[' => scan_array(bytes, i, depth),
        b'"' => {
            let _ = scan_string(bytes, i)?;
            Ok(())
        }
        b't' => scan_lit(bytes, i, b"true"),
        b'f' => scan_lit(bytes, i, b"false"),
        b'n' => scan_lit(bytes, i, b"null"),
        b'-' | b'0'..=b'9' => scan_number(bytes, i),
        _ => Err(Error::new("not JSON")),
    }
}

fn scan_object(bytes: &[u8], i: &mut usize, depth: &mut u32) -> Result<(), Error> {
    *depth = depth.checked_add(1).ok_or_else(|| Error::new("not JSON"))?;
    if *depth > 64 {
        return Err(Error::new("not JSON"));
    }
    *i += 1;
    let mut keys = Vec::<String>::new();
    loop {
        skip_ws(bytes, i);
        if *i >= bytes.len() {
            return Err(Error::new("not JSON"));
        }
        if bytes[*i] == b'}' {
            *i += 1;
            *depth -= 1;
            return Ok(());
        }
        if !keys.is_empty() {
            if bytes[*i] != b',' {
                return Err(Error::new("not JSON"));
            }
            *i += 1;
            skip_ws(bytes, i);
        }
        let key = scan_string(bytes, i)?;
        if keys.iter().any(|seen| seen == &key) {
            return Err(Error::new("duplicate key"));
        }
        keys.push(key);
        skip_ws(bytes, i);
        if *i >= bytes.len() || bytes[*i] != b':' {
            return Err(Error::new("not JSON"));
        }
        *i += 1;
        scan_value(bytes, i, depth)?;
    }
}

fn scan_array(bytes: &[u8], i: &mut usize, depth: &mut u32) -> Result<(), Error> {
    *depth = depth.checked_add(1).ok_or_else(|| Error::new("not JSON"))?;
    if *depth > 64 {
        return Err(Error::new("not JSON"));
    }
    *i += 1;
    let mut any = false;
    loop {
        skip_ws(bytes, i);
        if *i >= bytes.len() {
            return Err(Error::new("not JSON"));
        }
        if bytes[*i] == b']' {
            *i += 1;
            *depth -= 1;
            return Ok(());
        }
        if any {
            if bytes[*i] != b',' {
                return Err(Error::new("not JSON"));
            }
            *i += 1;
        }
        any = true;
        scan_value(bytes, i, depth)?;
    }
}

fn scan_string(bytes: &[u8], i: &mut usize) -> Result<String, Error> {
    if *i >= bytes.len() || bytes[*i] != b'"' {
        return Err(Error::new("not JSON"));
    }
    *i += 1;
    let mut out = String::new();
    while *i < bytes.len() {
        let byte = bytes[*i];
        match byte {
            b'"' => {
                *i += 1;
                return Ok(out);
            }
            b'\\' => {
                *i += 1;
                if *i >= bytes.len() {
                    return Err(Error::new("not JSON"));
                }
                let esc = bytes[*i];
                *i += 1;
                match esc {
                    b'"' | b'\\' | b'/' => out.push(esc as char),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => out.push(decode_hex(bytes, i)?),
                    _ => return Err(Error::new("not JSON")),
                }
            }
            0x00..=0x7f => {
                out.push(byte as char);
                *i += 1;
            }
            // Raw UTF-8 and `\uXXXX` must become the same scalar, or a duplicate key slips through.
            _ => out.push(take_utf8_char(bytes, i)?),
        }
    }
    Err(Error::new("not JSON"))
}

fn take_utf8_char(bytes: &[u8], i: &mut usize) -> Result<char, Error> {
    let width = match bytes.get(*i).copied() {
        Some(0xc2..=0xdf) => 2,
        Some(0xe0..=0xef) => 3,
        Some(0xf0..=0xf4) => 4,
        _ => return Err(Error::new("not JSON")),
    };
    let Some(end) = (*i).checked_add(width) else {
        return Err(Error::new("not JSON"));
    };
    if end > bytes.len() {
        return Err(Error::new("not JSON"));
    }
    let text = std::str::from_utf8(&bytes[*i..end]).map_err(|_| Error::new("not JSON"))?;
    let ch = text.chars().next().ok_or_else(|| Error::new("not JSON"))?;
    *i = end;
    Ok(ch)
}

fn read_hex4(bytes: &[u8], i: &mut usize) -> Result<u32, Error> {
    let end = i.checked_add(4).ok_or_else(|| Error::new("not JSON"))?;
    let digits = bytes.get(*i..end).ok_or_else(|| Error::new("not JSON"))?;
    if !digits.iter().all(u8::is_ascii_hexdigit) {
        return Err(Error::new("not JSON"));
    }
    let hex = std::str::from_utf8(digits).map_err(|_| Error::new("not JSON"))?;
    let code = u32::from_str_radix(hex, 16).map_err(|_| Error::new("not JSON"))?;
    *i = end;
    Ok(code)
}

/// A `\uXXXX` escape. A high surrogate must be followed by `\uDC00..DFFF`; a lone surrogate is invalid.
fn decode_hex(bytes: &[u8], i: &mut usize) -> Result<char, Error> {
    let code = read_hex4(bytes, i)?;
    let scalar = match code {
        0xd800..=0xdbff => {
            if bytes.get(*i) != Some(&b'\\') || bytes.get(*i + 1) != Some(&b'u') {
                return Err(Error::new("not JSON"));
            }
            *i += 2;
            let low = read_hex4(bytes, i)?;
            if !(0xdc00..=0xdfff).contains(&low) {
                return Err(Error::new("not JSON"));
            }
            0x10000 + ((code - 0xd800) << 10) + (low - 0xdc00)
        }
        _ => code,
    };
    char::from_u32(scalar).ok_or_else(|| Error::new("not JSON"))
}

fn scan_lit(bytes: &[u8], i: &mut usize, lit: &[u8]) -> Result<(), Error> {
    if bytes[*i..].starts_with(lit) {
        *i += lit.len();
        Ok(())
    } else {
        Err(Error::new("not JSON"))
    }
}

fn scan_number(bytes: &[u8], i: &mut usize) -> Result<(), Error> {
    let start = *i;
    if bytes[*i] == b'-' {
        *i += 1;
    }
    while *i < bytes.len() && matches!(bytes[*i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-') {
        *i += 1;
    }
    if *i == start {
        return Err(Error::new("not JSON"));
    }
    Ok(())
}

fn skip_ws(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() && matches!(bytes[*i], b' ' | b'\n' | b'\r' | b'\t') {
        *i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::reject_duplicate_keys;

    #[test]
    fn duplicate_key_is_exact() {
        let err = reject_duplicate_keys(r#"{"a":1,"a":2}"#).unwrap_err();
        assert_eq!(err.message(), "duplicate key");
        let err = reject_duplicate_keys(r#"{"a":{"b":1,"b":2}}"#).unwrap_err();
        assert_eq!(err.message(), "duplicate key");
    }

    #[test]
    fn broken_object_is_not_json() {
        let err = reject_duplicate_keys(r#"{"a":"#).unwrap_err();
        assert_eq!(err.message(), "not JSON");
    }

    #[test]
    fn distinct_keys_and_non_objects_pass_the_scanner() {
        assert!(reject_duplicate_keys(r#"{"a":1,"b":2}"#).is_ok());
        assert!(reject_duplicate_keys("[1,2]").is_ok());
        assert!(reject_duplicate_keys("not-json").is_ok());
    }

    #[test]
    fn raw_utf8_and_u_escape_are_the_same_key() {
        let cell = "{\"é\":\"x\",\"\\u00e9\":\"y\"}";
        let err = reject_duplicate_keys(cell).unwrap_err();
        assert_eq!(err.message(), "duplicate key");
        let err = reject_duplicate_keys("{\"a\":{\"\\u00e9\":1,\"é\":2}}").unwrap_err();
        assert_eq!(err.message(), "duplicate key");
        assert!(reject_duplicate_keys("{\"é\":\"x\",\"à\":\"y\"}").is_ok());
    }
}
