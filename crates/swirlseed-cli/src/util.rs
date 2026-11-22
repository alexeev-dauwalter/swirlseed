//! Small dependency-free helpers for the CLI (unit-tested).

/// Smallest and largest accepted image side, in pixels.
pub const MIN_SIDE: u32 = 16;
pub const MAX_SIDE: u32 = 8192;

/// Parses `"1024"` (square) or `"1920x1080"` (also `X` and `×` as the separator).
pub fn parse_size(s: &str) -> Result<(u32, u32), String> {
    let s = s.trim();
    let (w, h) = match s.find(['x', 'X', '×']) {
        Some(i) => {
            let sep_len = s[i..].chars().next().map_or(1, char::len_utf8);
            (&s[..i], &s[i + sep_len..])
        }
        None => (s, s),
    };
    let side = |part: &str| -> Result<u32, String> {
        let v: u32 = part
            .trim()
            .parse()
            .map_err(|_| format!("`{s}` is not a size; use e.g. 1024 or 1920x1080"))?;
        if !(MIN_SIDE..=MAX_SIDE).contains(&v) {
            return Err(format!(
                "each side must be between {MIN_SIDE} and {MAX_SIDE} px, got {v}"
            ));
        }
        Ok(v)
    };
    Ok((side(w)?, side(h)?))
}

/// A filesystem-friendly file stem for a seed: letters and digits are kept, runs of anything else
/// become a single `_`, at most 40 characters, never empty.
pub fn slug(seed: &str) -> String {
    let mut out = String::new();
    let mut pending_sep = false;
    for ch in seed.chars() {
        if ch.is_alphanumeric() {
            if pending_sep && !out.is_empty() {
                out.push('_');
            }
            pending_sep = false;
            out.push(ch);
        } else {
            pending_sep = true;
        }
    }
    let out: String = out.chars().take(40).collect();
    if out.is_empty() {
        "swirlseed".to_owned()
    } else {
        out
    }
}

/// Removes one trailing `\n` or `\r\n` (what `echo` and editors add), nothing else.
pub fn strip_trailing_newline(mut s: String) -> String {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(parse_size("1024"), Ok((1024, 1024)));
        assert_eq!(parse_size("1920x1080"), Ok((1920, 1080)));
        assert_eq!(parse_size(" 800X600 "), Ok((800, 600)));
        assert_eq!(parse_size("640×480"), Ok((640, 480)));
        assert!(parse_size("8x8").is_err());
        assert!(parse_size("9000").is_err());
        assert!(parse_size("abc").is_err());
        assert!(parse_size("1024x").is_err());
        assert!(parse_size("").is_err());
    }

    #[test]
    fn slugs() {
        assert_eq!(slug("Hello, world!"), "Hello_world");
        assert_eq!(slug("  --a//b  "), "a_b");
        assert_eq!(slug("Пётр"), "Пётр");
        assert_eq!(slug("???"), "swirlseed");
        assert_eq!(slug(""), "swirlseed");
        assert_eq!(slug(&"x".repeat(100)).chars().count(), 40);
    }

    #[test]
    fn newlines() {
        assert_eq!(strip_trailing_newline("abc\n".into()), "abc");
        assert_eq!(strip_trailing_newline("abc\r\n".into()), "abc");
        assert_eq!(strip_trailing_newline("abc\n\n".into()), "abc\n");
        assert_eq!(strip_trailing_newline("abc".into()), "abc");
    }
}
