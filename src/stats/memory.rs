use super::{BYTES_PER_GB, PERCENT, unit};
use crate::cli;
use std::fmt::Write;
use sysinfo::System;

pub fn get_memory_stats(s: &System, flags: &[&str], no_units: bool, buf: &mut String) {
    let ram_flag_present = flags.iter().any(|&flag| cli::ALL_RAM_FLAGS.contains(&flag));
    let swp_flag_present = flags.iter().any(|&flag| cli::ALL_SWP_FLAGS.contains(&flag));

    let (ram_total, ram_used, ram_usage_percentage) = if ram_flag_present {
        let ram_total = s.total_memory();
        let ram_used = s.used_memory();
        let ram_usage_percentage = if ram_total > 0 {
            ((ram_used as f32 / ram_total as f32) * PERCENT).round() as u32
        } else {
            0
        };
        (ram_total, ram_used, ram_usage_percentage)
    } else {
        (0, 0, 0)
    };
    let (swp_total, swp_used, swp_usage_percentage) = if swp_flag_present {
        let swp_total = s.total_swap();
        let swp_used = s.used_swap();
        let swp_usage_percentage = if swp_total > 0 {
            ((swp_used as f32 / swp_total as f32) * PERCENT).round() as u32
        } else {
            0
        };
        (swp_total, swp_used, swp_usage_percentage)
    } else {
        (0, 0, 0)
    };

    for &flag in flags {
        match flag {
            "ram_available" => {
                let unit = unit(no_units, "GB");
                let _ = write!(
                    buf,
                    "RAM_AVAILABLE=\"{:.1}{unit}\" ",
                    s.available_memory() as f32 / BYTES_PER_GB
                );
            }
            "ram_total" => {
                let unit = unit(no_units, "GB");
                let _ = write!(
                    buf,
                    "RAM_TOTAL=\"{:.1}{unit}\" ",
                    ram_total as f32 / BYTES_PER_GB
                );
            }
            "ram_used" => {
                let unit = unit(no_units, "GB");
                let _ = write!(
                    buf,
                    "RAM_USED=\"{:.1}{unit}\" ",
                    ram_used as f32 / BYTES_PER_GB
                );
            }
            "ram_usage" => {
                let unit = unit(no_units, "%");
                let _ = write!(buf, "RAM_USAGE=\"{ram_usage_percentage}{unit}\" ");
            }
            "swp_free" => {
                let unit = unit(no_units, "GB");
                let _ = write!(
                    buf,
                    "SWP_FREE=\"{:.1}{unit}\" ",
                    s.free_swap() as f32 / BYTES_PER_GB
                );
            }
            "swp_total" => {
                let unit = unit(no_units, "GB");
                let _ = write!(
                    buf,
                    "SWP_TOTAL=\"{:.1}{unit}\" ",
                    swp_total as f32 / BYTES_PER_GB
                );
            }
            "swp_used" => {
                let unit = unit(no_units, "GB");
                let _ = write!(
                    buf,
                    "SWP_USED=\"{:.1}{unit}\" ",
                    swp_used as f32 / BYTES_PER_GB
                );
            }
            "swp_usage" => {
                let unit = unit(no_units, "%");
                let _ = write!(buf, "SWP_USAGE=\"{swp_usage_percentage}{unit}\" ");
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_memory_stats_all_flags_emit_expected_keys() {
        let mut s = System::new_all();
        s.refresh_all();
        let mut buf = String::new();

        get_memory_stats(&s, cli::ALL_MEMORY_FLAGS, false, &mut buf);

        assert!(buf.contains("RAM_AVAILABLE="));
        assert!(buf.contains("RAM_TOTAL="));
        assert!(buf.contains("RAM_USED="));
        assert!(buf.contains("RAM_USAGE="));
        assert!(buf.contains("SWP_FREE="));
        assert!(buf.contains("SWP_TOTAL="));
        assert!(buf.contains("SWP_USED="));
        assert!(buf.contains("SWP_USAGE="));
    }

    #[test]
    fn test_get_memory_stats_empty_flags() {
        let s = System::new_all();
        let mut buf = String::new();

        get_memory_stats(&s, &[], false, &mut buf);

        assert_eq!(buf, "");
    }
}
