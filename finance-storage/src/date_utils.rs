use chrono::{DateTime, SecondsFormat, Utc};

/// Format a DateTime<Utc> to a fixed-width RFC3339 string suitable for SQLite
/// text-based date comparisons.
///
/// Uses second-level precision and the `Z` suffix to ensure all dates have
/// identical string length and sort lexicographically in correct temporal order.
/// This avoids bugs caused by chrono's default `to_rfc3339()` which produces
/// variable-length subsecond precision (e.g. `.123456789` vs no fraction)
/// and may use `+00:00` vs `Z`, both of which break SQLite string comparisons.
///
/// Output format: `YYYY-MM-DDTHH:MM:SSZ` (exactly 20 characters).
pub fn format_dt(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Format an optional DateTime<Utc> for SQLite storage.
pub fn format_dt_opt(dt: &Option<DateTime<Utc>>) -> Option<String> {
    dt.as_ref().map(format_dt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Timelike};

    #[test]
    fn test_format_dt_consistent_output() {
        let dt = Utc.with_ymd_and_hms(2026, 2, 15, 10, 30, 45).unwrap();
        assert_eq!(format_dt(&dt), "2026-02-15T10:30:45Z");
    }

    #[test]
    fn test_format_dt_strips_nanoseconds() {
        let dt = Utc
            .with_ymd_and_hms(2026, 2, 15, 10, 30, 45)
            .unwrap()
            .with_nanosecond(123_456_789)
            .unwrap();
        // Nanoseconds are dropped — we get second precision only
        assert_eq!(format_dt(&dt), "2026-02-15T10:30:45Z");
    }

    #[test]
    fn test_format_dt_uses_z_suffix() {
        let dt = Utc::now();
        let formatted = format_dt(&dt);
        assert!(formatted.ends_with('Z'));
        assert!(!formatted.contains("+00:00"));
    }

    #[test]
    fn test_format_dt_fixed_length() {
        let dt1 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let dt2 = Utc.with_ymd_and_hms(2026, 12, 31, 23, 59, 59).unwrap();
        assert_eq!(format_dt(&dt1).len(), format_dt(&dt2).len());
    }

    #[test]
    fn test_format_dt_lexicographic_order() {
        let earlier = Utc.with_ymd_and_hms(2026, 2, 15, 10, 0, 0).unwrap();
        let later = Utc.with_ymd_and_hms(2026, 2, 15, 10, 0, 1).unwrap();
        assert!(format_dt(&earlier) < format_dt(&later));
    }

    #[test]
    fn test_format_dt_opt_none() {
        assert_eq!(format_dt_opt(&None), None);
    }

    #[test]
    fn test_format_dt_opt_some() {
        let dt = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        assert_eq!(format_dt_opt(&Some(dt)), Some("2026-06-01T12:00:00Z".to_string()));
    }
}
