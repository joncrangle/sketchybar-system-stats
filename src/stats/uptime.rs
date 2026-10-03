use std::fmt::Write;
use sysinfo::System;

struct TimeUnit {
    name: &'static str,
    seconds: u64,
    suffix: &'static str,
}

const TIME_UNITS: &[TimeUnit] = &[
    TimeUnit {
        name: "week",
        seconds: 7 * 24 * 3600,
        suffix: "w",
    },
    TimeUnit {
        name: "day",
        seconds: 24 * 3600,
        suffix: "d",
    },
    TimeUnit {
        name: "hour",
        seconds: 3600,
        suffix: "h",
    },
    TimeUnit {
        name: "min",
        seconds: 60,
        suffix: "m",
    },
    TimeUnit {
        name: "sec",
        seconds: 1,
        suffix: "s",
    },
];

/// Formats an uptime duration as a sketchybar key/value pair into `buf`.
///
/// Units are emitted in descending order of size (week to sec). An empty
/// `flags` slice selects every unit. When no unit qualifies (for example all
/// requested flags are unknown, or the duration is zero seconds), the smallest
/// qualifying unit falls back to a zero value.
fn format_uptime(mut uptime_secs: u64, flags: &[&str], no_units: bool, buf: &mut String) {
    let _ = write!(buf, "UPTIME=\"");
    let mut has_value = false;
    let mut smallest_suffix = "s";

    for unit in TIME_UNITS {
        if flags.is_empty() || flags.contains(&unit.name) {
            smallest_suffix = unit.suffix;
            if uptime_secs >= unit.seconds {
                let value = uptime_secs / unit.seconds;
                uptime_secs %= unit.seconds;
                if has_value {
                    let _ = write!(buf, " ");
                }
                if no_units {
                    let _ = write!(buf, "{value}");
                } else {
                    let _ = write!(buf, "{}{}", value, unit.suffix);
                }
                has_value = true;
            }
        }
    }

    if !has_value {
        if no_units {
            let _ = write!(buf, "0");
        } else {
            let _ = write!(buf, "0{smallest_suffix}");
        }
    }

    let _ = write!(buf, "\" ");
}

pub fn get_uptime_stats(flags: &[&str], no_units: bool, buf: &mut String) {
    format_uptime(System::uptime(), flags, no_units, buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format_uptime_helper(uptime_secs: u64, flags: &[&str], no_units: bool) -> String {
        let mut buf = String::new();
        format_uptime(uptime_secs, flags, no_units, &mut buf);
        buf
    }

    #[test]
    fn test_format_uptime_zero_seconds() {
        assert_eq!(format_uptime_helper(0, &[], false), "UPTIME=\"0s\" ");
    }

    #[test]
    fn test_format_uptime_seconds_and_minutes() {
        assert_eq!(
            format_uptime_helper(61, &["min", "sec"], false),
            "UPTIME=\"1m 1s\" "
        );
    }

    #[test]
    fn test_format_uptime_hours_and_minutes() {
        assert_eq!(
            format_uptime_helper(5400, &["hour", "min"], false),
            "UPTIME=\"1h 30m\" "
        );
    }

    #[test]
    fn test_format_uptime_all_units() {
        let secs = 7 * 24 * 3600 + 24 * 3600 + 3600 + 60 + 1;
        assert_eq!(
            format_uptime_helper(secs, &["week", "day", "hour", "min", "sec"], false),
            "UPTIME=\"1w 1d 1h 1m 1s\" "
        );
    }

    #[test]
    fn test_format_uptime_empty_flags_select_all_units() {
        assert_eq!(
            format_uptime_helper(3661, &[], false),
            "UPTIME=\"1h 1m 1s\" "
        );
    }

    #[test]
    fn test_format_uptime_flags_reordered() {
        assert_eq!(
            format_uptime_helper(3660, &["sec", "hour", "min"], false),
            "UPTIME=\"1h 1m\" "
        );
    }

    #[test]
    fn test_format_uptime_empty_flags_fallback() {
        assert_eq!(
            format_uptime_helper(5400, &["bogus"], false),
            "UPTIME=\"0s\" "
        );
    }

    #[test]
    fn test_format_uptime_zero_with_custom_smallest_unit() {
        assert_eq!(
            format_uptime_helper(0, &["day", "hour"], false),
            "UPTIME=\"0h\" "
        );
    }

    #[test]
    fn test_get_uptime_stats_invalid_flag() {
        let mut buf = String::new();
        get_uptime_stats(&["invalid"], false, &mut buf);

        assert_eq!(buf, "UPTIME=\"0s\" ");
    }

    #[test]
    fn test_format_uptime_without_units_for_seconds() {
        assert_eq!(format_uptime_helper(61, &["sec"], true), "UPTIME=\"61\" ");
    }

    #[test]
    fn test_format_uptime_without_units_for_multiple_units() {
        assert_eq!(
            format_uptime_helper(5400, &["hour", "min"], true),
            "UPTIME=\"1 30\" "
        );
    }

    #[test]
    fn test_format_uptime_without_units_empty_flags_select_all_units() {
        assert_eq!(format_uptime_helper(3661, &[], true), "UPTIME=\"1 1 1\" ");
    }

    #[test]
    fn test_format_uptime_without_units_for_zero_seconds() {
        assert_eq!(format_uptime_helper(0, &["sec"], true), "UPTIME=\"0\" ");
    }
}
