use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

/// Store UTC timestamps as `YYYY-MM-DD HH:MM:SS.ffffff`, which sorts and matches
/// the strings SQLite already holds from the Python build.
pub fn format_db(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S%.6f").to_string()
}

pub fn format_api(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}

pub fn now() -> DateTime<Utc> {
    Utc::now()
}

pub fn parse_db(value: &str) -> Option<DateTime<Utc>> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc));
    }
    let formats = [
        "%Y-%m-%d %H:%M:%S%.f%:z",
        "%Y-%m-%d %H:%M:%S%:z",
        "%Y-%m-%dT%H:%M:%S%.f%:z",
        "%Y-%m-%dT%H:%M:%S%:z",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
    ];
    for format in formats {
        if let Ok(dt) = DateTime::parse_from_str(value, format) {
            return Some(dt.with_timezone(&Utc));
        }
        if let Ok(naive) = NaiveDateTime::parse_from_str(value, format) {
            return Some(Utc.from_utc_datetime(&naive));
        }
    }
    None
}

pub fn parse_db_required(value: &str) -> DateTime<Utc> {
    parse_db(value).unwrap_or_else(now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_legacy_shapes() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 5, 19, 17, 18).unwrap();
        let stored = format_db(dt);
        let parsed = parse_db(&stored).unwrap();
        assert_eq!(parsed.timestamp(), dt.timestamp());
        assert!(parse_db("2026-05-05 19:17:18+00:00").is_some());
        assert!(parse_db("2026-05-05T19:17:18Z").is_some());
        assert!(parse_db("").is_none());
    }
}
