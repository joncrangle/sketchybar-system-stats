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
fn format_uptime(uptime_secs: u64, flags: &[&str], buf: &mut String) {
    let mut uptime_secs = uptime_secs;

    let sorted_flags: Vec<&str> = if flags.is_empty() {
        TIME_UNITS.iter().map(|u| u.name).collect()
    } else {
        let mut flags_vec: Vec<&str> = flags
            .iter()
            .copied()
            .filter(|&flag| TIME_UNITS.iter().any(|u| u.name == flag))
            .collect();

        flags_vec.sort_by_key(|&flag| {
            TIME_UNITS
                .iter()
                .position(|u| u.name == flag)
                .unwrap_or(usize::MAX)
        });
        flags_vec
    };

    let _ = write!(buf, "UPTIME=\"");
    let mut has_value = false;

    for &flag in &sorted_flags {
        if let Some(unit) = TIME_UNITS.iter().find(|u| u.name == flag)
            && uptime_secs >= unit.seconds
        {
            let value = uptime_secs / unit.seconds;
            uptime_secs %= unit.seconds;
            if has_value {
                let _ = write!(buf, " ");
            }
            let _ = write!(buf, "{}{}", value, unit.suffix);
            has_value = true;
        }
    }

    if !has_value {
        let min_suffix = sorted_flags
            .last()
            .and_then(|flag| TIME_UNITS.iter().find(|u| u.name == *flag))
            .map(|unit| unit.suffix)
            .unwrap_or("s");
        let _ = write!(buf, "0{}", min_suffix);
    }

    let _ = write!(buf, "\" ");
}

pub fn get_uptime_stats(flags: &[&str], buf: &mut String) {
    format_uptime(System::uptime(), flags, buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format_uptime_helper(uptime_secs: u64, flags: &[&str]) -> String {
        let mut buf = String::new();
        format_uptime(uptime_secs, flags, &mut buf);
        buf
    }

    #[test]
    fn test_format_uptime_zero_seconds() {
        assert_eq!(format_uptime_helper(0, &[]), "UPTIME=\"0s\" ");
    }

    #[test]
    fn test_format_uptime_seconds_and_minutes() {
        assert_eq!(
            format_uptime_helper(61, &["min", "sec"]),
            "UPTIME=\"1m 1s\" "
        );
    }

    #[test]
    fn test_format_uptime_hours_and_minutes() {
        assert_eq!(
            format_uptime_helper(5400, &["hour", "min"]),
            "UPTIME=\"1h 30m\" "
        );
    }

    #[test]
    fn test_format_uptime_all_units() {
        let secs = 7 * 24 * 3600 + 24 * 3600 + 3600 + 60 + 1;
        assert_eq!(
            format_uptime_helper(secs, &["week", "day", "hour", "min", "sec"]),
            "UPTIME=\"1w 1d 1h 1m 1s\" "
        );
    }

    #[test]
    fn test_format_uptime_flags_reordered() {
        assert_eq!(
            format_uptime_helper(3660, &["sec", "hour", "min"]),
            "UPTIME=\"1h 1m\" "
        );
    }

    #[test]
    fn test_format_uptime_empty_flags_fallback() {
        assert_eq!(format_uptime_helper(5400, &["bogus"]), "UPTIME=\"0s\" ");
    }

    #[test]
    fn test_get_uptime_stats_invalid_flag() {
        let mut buf = String::new();
        get_uptime_stats(&["invalid"], &mut buf);

        assert_eq!(buf, "UPTIME=\"0s\" ");
    }
}
