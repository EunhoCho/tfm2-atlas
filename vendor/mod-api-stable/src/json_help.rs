//! Tiny JSON scalar helpers for the mod-side wrappers.
//!
//! The contract crate has zero dependencies, so typed convenience accessors
//! over the JSON slots parse/format scalars manually. Mods that need full
//! JSON manipulation can pull in their own serde — these helpers only cover
//! the common "read/write one number/flag/string" cases.

pub(crate) fn parse_i64(json: &str) -> Option<i64> {
    json.trim().parse().ok()
}

pub(crate) fn parse_f64(json: &str) -> Option<f64> {
    json.trim().parse().ok()
}

pub(crate) fn parse_bool(json: &str) -> Option<bool> {
    match json.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// Unquotes a JSON string literal (handles the standard escapes including
/// `\uXXXX` with surrogate pairs). Returns None for non-string JSON.
pub(crate) fn parse_string(json: &str) -> Option<String> {
    let trimmed = json.trim();
    let inner = trimmed.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next()? {
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            '/' => out.push('/'),
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            'b' => out.push('\u{0008}'),
            'f' => out.push('\u{000C}'),
            'u' => {
                let unit = read_hex4(&mut chars)?;
                let code = if (0xD800..0xDC00).contains(&unit) {
                    // High surrogate — must be followed by \uXXXX low surrogate.
                    if chars.next()? != '\\' || chars.next()? != 'u' {
                        return None;
                    }
                    let low = read_hex4(&mut chars)?;
                    if !(0xDC00..0xE000).contains(&low) {
                        return None;
                    }
                    0x10000 + ((unit as u32 - 0xD800) << 10) + (low as u32 - 0xDC00)
                } else {
                    unit as u32
                };
                out.push(char::from_u32(code)?);
            }
            _ => return None,
        }
    }
    Some(out)
}

fn read_hex4(chars: &mut std::str::Chars<'_>) -> Option<u16> {
    let mut value = 0u16;
    for _ in 0..4 {
        value = value
            .checked_mul(16)?
            .checked_add(chars.next()?.to_digit(16)? as u16)?;
    }
    Some(value)
}

/// Quotes a string as a JSON string literal.
pub(crate) fn quote_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Extracts a top-level field's raw JSON value from a JSON object, e.g.
/// `object_field("{\"a\":1,\"b\":\"x\"}", "b")` → `Some("\"x\"")`. Depth-1
/// only (nested objects/arrays come back as their raw slice); None when the
/// input is not an object or the key is absent.
pub(crate) fn object_field(json: &str, key: &str) -> Option<String> {
    let trimmed = json.trim();
    let inner = trimmed.strip_prefix('{')?.strip_suffix('}')?;
    let bytes = inner.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        // Key string.
        if bytes[i] != b'"' {
            return None;
        }
        let key_start = i + 1;
        let key_end = scan_string_end(bytes, key_start)?;
        let raw_key = &inner[key_start - 1..key_end + 1];
        i = key_end + 1;
        while i < bytes.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b':' {
            return None;
        }
        i += 1;
        while i < bytes.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        // Value: scan to the matching end at depth 0.
        let value_start = i;
        let mut depth = 0usize;
        while i < bytes.len() {
            match bytes[i] {
                b'"' => i = scan_string_end(bytes, i + 1)?,
                b'{' | b'[' => depth += 1,
                b'}' | b']' => depth = depth.checked_sub(1)?,
                b',' if depth == 0 => break,
                _ => {}
            }
            i += 1;
        }
        if parse_string(raw_key).as_deref() == Some(key) {
            return Some(inner[value_start..i].trim().to_string());
        }
        i += 1; // past the comma
    }
    None
}

/// Index of the closing quote of a JSON string whose content starts at
/// `start` (the byte after the opening quote).
fn scan_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return Some(i),
            _ => i += 1,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_round_trips() {
        assert_eq!(parse_i64(" 42 "), Some(42));
        assert_eq!(parse_f64("2.5"), Some(2.5));
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(
            parse_string("\"a\\\"b\\n\\u00e9\""),
            Some("a\"b\né".to_string())
        );
        assert_eq!(
            parse_string(&quote_string("한글 \"x\"\n")),
            Some("한글 \"x\"\n".to_string())
        );
        assert_eq!(parse_string("\"\\ud83d\\ude00\""), Some("😀".to_string()));
        assert_eq!(parse_string("123"), None);
    }

    #[test]
    fn object_field_extracts_depth_one_values() {
        let json = "{\"selected\": true, \"text\": \"a,\\\"b}\", \"n\": {\"x\": 1}}";
        assert_eq!(object_field(json, "selected"), Some("true".to_string()));
        assert_eq!(object_field(json, "text"), Some("\"a,\\\"b}\"".to_string()));
        assert_eq!(object_field(json, "n"), Some("{\"x\": 1}".to_string()));
        assert_eq!(object_field(json, "missing"), None);
        assert_eq!(object_field("[1]", "x"), None);
    }
}
