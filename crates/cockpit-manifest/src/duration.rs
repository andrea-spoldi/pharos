use std::time::Duration;

/// Parses `<n>s`, `<n>m` or `<n>h` with `n > 0` (e.g. `30s`, `5m`).
pub fn parse_duration(s: &str) -> Option<Duration> {
    let unit = s.chars().last()?;
    let digits = &s[..s.len() - unit.len_utf8()];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: u64 = digits.parse().ok().filter(|n| *n > 0)?;
    let secs = match unit {
        's' => n,
        'm' => n.checked_mul(60)?,
        'h' => n.checked_mul(3600)?,
        _ => return None,
    };
    Some(Duration::from_secs(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_units() {
        assert_eq!(parse_duration("30s"), Some(Duration::from_secs(30)));
        assert_eq!(parse_duration("5m"), Some(Duration::from_secs(300)));
        assert_eq!(parse_duration("2h"), Some(Duration::from_secs(7200)));
    }

    #[test]
    fn rejects_malformed() {
        for s in ["", "m", "0s", "5", "5 m", "-5m", "5d", "1.5m", "fast"] {
            assert_eq!(parse_duration(s), None, "{s:?}");
        }
    }
}
