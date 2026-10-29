use std::fmt::Write;
use sysinfo::{LoadAvg, System};

/// Formats the selected load averages as sketchybar key/value pairs into `buf`.
///
/// Averages are emitted in order of their window (1, 5, then 15 minutes). An
/// empty `flags` slice selects all three.
fn format_load(load: &LoadAvg, flags: &[&str], buf: &mut String) {
    let averages = [
        ("one", 1, load.one),
        ("five", 5, load.five),
        ("fifteen", 15, load.fifteen),
    ];

    for (name, minutes, value) in averages {
        if flags.is_empty() || flags.contains(&name) {
            let _ = write!(buf, "LOAD_AVG_{minutes}=\"{value:.2}\" ");
        }
    }
}

pub fn get_load_stats(flags: &[&str], buf: &mut String) {
    if let Ok(load) = System::load_average() {
        format_load(&load, flags, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_load() -> LoadAvg {
        LoadAvg {
            one: 1.5,
            five: 0.25,
            fifteen: 10.0,
        }
    }

    #[test]
    fn test_format_load_empty_flags_selects_all() {
        let mut buf = String::new();

        format_load(&sample_load(), &[], &mut buf);

        assert_eq!(
            buf,
            "LOAD_AVG_1=\"1.50\" LOAD_AVG_5=\"0.25\" LOAD_AVG_15=\"10.00\" "
        );
    }

    #[test]
    fn test_format_load_selected_subset_keeps_window_order() {
        let mut buf = String::new();

        format_load(&sample_load(), &["fifteen", "one"], &mut buf);

        assert_eq!(buf, "LOAD_AVG_1=\"1.50\" LOAD_AVG_15=\"10.00\" ");
    }

    #[test]
    fn test_get_load_stats_emits_every_window() {
        let mut buf = String::new();

        get_load_stats(&[], &mut buf);

        for key in ["LOAD_AVG_1=\"", "LOAD_AVG_5=\"", "LOAD_AVG_15=\""] {
            assert!(buf.contains(key), "missing {key} in: {buf}");
        }
    }
}
