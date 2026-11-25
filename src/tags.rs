use crate::error::Error;
use crate::json_scan::reject_duplicate_keys;

/// `resource_tags` is a CUR 2.0 map. In a CSV cell that map is a JSON object of strings.
pub(crate) fn parse_resource_tags(raw: &str) -> Result<Vec<(String, String)>, Error> {
    let Some(text) = trim_ascii_edges(raw) else {
        return Err(Error::new("invalid resource_tags"));
    };
    if text.is_empty() || text == "{}" {
        return Ok(Vec::new());
    }
    if text.len() > 65_536 {
        return Err(Error::new("invalid resource_tags"));
    }
    match reject_duplicate_keys(text) {
        Ok(()) => {}
        Err(err) if err.message() == "duplicate key" => return Err(err),
        Err(_) => return Err(Error::new("invalid resource_tags")),
    }
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| Error::new("invalid resource_tags"))?;
    let Some(obj) = value.as_object() else {
        return Err(Error::new("invalid resource_tags"));
    };
    if obj.len() > 50 {
        return Err(Error::new("invalid resource_tags"));
    }
    let mut pairs = Vec::with_capacity(obj.len());
    for (key, item) in obj {
        let Some(val) = item.as_str() else {
            return Err(Error::new("invalid resource_tags"));
        };
        if key.is_empty()
            || key.chars().count() > 128
            || val.chars().count() > 256
            || has_control(key)
            || has_control(val)
        {
            return Err(Error::new("invalid resource_tags"));
        }
        pairs.push((key.clone(), val.to_string()));
    }
    Ok(pairs)
}

/// Strips only ASCII space, tab, CR and LF from both ends. Any other Unicode
/// whitespace (NBSP, U+3000, U+2028, ...) left at an edge is an error (`None`).
pub(crate) fn trim_ascii_edges(raw: &str) -> Option<&str> {
    let text = raw.trim_matches([' ', '\t', '\r', '\n']);
    let edge = text
        .chars()
        .next()
        .into_iter()
        .chain(text.chars().next_back());
    if edge.into_iter().any(char::is_whitespace) {
        None
    } else {
        Some(text)
    }
}

fn has_control(text: &str) -> bool {
    text.chars().any(|c| c.is_control())
}

#[cfg(test)]
mod tests {
    use super::parse_resource_tags;

    #[test]
    fn unicode_white_at_the_edge_is_an_error_ascii_white_is_not() {
        for ws in ['\u{a0}', '\u{3000}', '\u{2028}'] {
            for raw in [format!("{ws}{{}}"), format!("{{}}{ws}"), format!("{ws}")] {
                let err = parse_resource_tags(&raw).unwrap_err();
                assert_eq!(err.message(), "invalid resource_tags");
            }
        }
        assert!(parse_resource_tags(" \t\r\n{}\n\r\t ").unwrap().is_empty());
        assert_eq!(
            parse_resource_tags(" \t{\"a\":\"b\"}\r\n").unwrap().len(),
            1
        );
    }

    #[test]
    fn empty_and_object() {
        assert!(parse_resource_tags("").unwrap().is_empty());
        assert!(parse_resource_tags(" {} ").unwrap().is_empty());
        assert!(parse_resource_tags("{}").unwrap().is_empty());
    }

    #[test]
    fn two_string_pairs() {
        let pairs = parse_resource_tags(r#"{"user_env":"prod","user_team":"platform"}"#).unwrap();
        assert_eq!(pairs.len(), 2);
        assert!(pairs.contains(&("user_team".to_string(), "platform".to_string())));
        assert!(pairs.contains(&("user_env".to_string(), "prod".to_string())));
    }

    #[test]
    fn duplicate_key_rejected() {
        let err = parse_resource_tags(r#"{"secret":"a","secret":"b"}"#).unwrap_err();
        assert_eq!(err.message(), "duplicate key");
        assert!(!err.message().contains("secret"));
    }

    #[test]
    fn non_string_hides_the_cell() {
        let err = parse_resource_tags(r#"{"user_team":["secret"]}"#).unwrap_err();
        assert_eq!(err.message(), "invalid resource_tags");
        assert!(!err.message().contains("secret"));
        let err = parse_resource_tags(r#"["secret"]"#).unwrap_err();
        assert_eq!(err.message(), "invalid resource_tags");
        assert!(!err.message().contains("secret"));
    }

    #[test]
    fn too_many_tags() {
        let mut body = String::from('{');
        for i in 0..51 {
            if i > 0 {
                body.push(',');
            }
            body.push_str(&format!(r#""k{i}":"v""#));
        }
        body.push('}');
        let err = parse_resource_tags(&body).unwrap_err();
        assert_eq!(err.message(), "invalid resource_tags");
    }

    #[test]
    fn control_char_rejected() {
        let err = parse_resource_tags("{\"user_team\":\"a\\nb\"}").unwrap_err();
        assert_eq!(err.message(), "invalid resource_tags");
        assert!(parse_resource_tags(r#"{"user_team":""}"#).unwrap().len() == 1);
    }

    #[test]
    fn surrogate_pair_is_one_scalar_and_lone_surrogate_is_invalid() {
        let pairs = parse_resource_tags(r#"{"user_team":"\ud83d\ude00"}"#).unwrap();
        assert_eq!(pairs[0].1, "\u{1F600}");
        let raw = parse_resource_tags("{\"user_team\":\"\u{1F600}\"}").unwrap();
        assert_eq!(raw, pairs);
        for bad in [
            r#"{"user_team":"\ud83d"}"#,
            r#"{"user_team":"\ude00"}"#,
            r#"{"user_team":"\ud83dx"}"#,
            r#"{"user_team":"\ud83d\u0041"}"#,
        ] {
            let err = parse_resource_tags(bad).unwrap_err();
            assert_eq!(err.message(), "invalid resource_tags");
        }
    }

    #[test]
    fn tag_limits_accept_the_edge_and_reject_one_more() {
        let key = |n: usize| "k".repeat(n);
        let val = |n: usize| "v".repeat(n);
        let one = |k: &str, v: &str| format!(r#"{{"{k}":"{v}"}}"#);
        assert_eq!(parse_resource_tags(&one(&key(128), "x")).unwrap().len(), 1);
        assert!(parse_resource_tags(&one(&key(129), "x")).is_err());
        assert_eq!(parse_resource_tags(&one("x", &val(256))).unwrap().len(), 1);
        assert!(parse_resource_tags(&one("x", &val(257))).is_err());
        assert!(parse_resource_tags(&one("", "x")).is_err());
        let fifty = (0..50)
            .map(|i| format!(r#""k{i}":"v""#))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            parse_resource_tags(&format!("{{{fifty}}}")).unwrap().len(),
            50
        );
        let big = format!("{{}}{}", " ".repeat(70_000));
        assert!(parse_resource_tags(&big).unwrap().is_empty());
        let over = format!(r#"{{"a":"b"{}}}"#, " ".repeat(65_536));
        let err = parse_resource_tags(&over).unwrap_err();
        assert_eq!(err.message(), "invalid resource_tags");
    }
}
