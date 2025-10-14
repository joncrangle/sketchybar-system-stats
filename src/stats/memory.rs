use crate::cli;
use sysinfo::System;

const BYTES_PER_GB: f32 = 1_073_741_824.0;

pub fn get_memory_stats(s: &System, flags: &[&str], no_units: bool) -> Vec<String> {
    let mut result = Vec::new();

    let ram_flag_present = flags
        .iter()
        .any(|&flag| cli::all_ram_flags().contains(&flag));
    let swp_flag_present = flags
        .iter()
        .any(|&flag| cli::all_swp_flags().contains(&flag));

    let (ram_total, ram_used, ram_usage_percentage) = if ram_flag_present {
        let ram_total = s.total_memory();
        let ram_used = s.used_memory();
        let ram_usage_percentage = if ram_total > 0 {
            ((ram_used as f32 / ram_total as f32) * 100.0).round() as u32
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
            ((swp_used as f32 / swp_total as f32) * 100.0).round() as u32
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
                let unit = if no_units { "" } else { "GB" };
                result.push(format!(
                    "RAM_AVAILABLE=\"{:.1}{unit}\" ",
                    s.available_memory() as f32 / BYTES_PER_GB
                ));
            }
            "ram_total" => {
                let unit = if no_units { "" } else { "GB" };
                result.push(format!(
                    "RAM_TOTAL=\"{:.1}{unit}\" ",
                    ram_total as f32 / BYTES_PER_GB
                ));
            }
            "ram_used" => {
                let unit = if no_units { "" } else { "GB" };
                result.push(format!(
                    "RAM_USED=\"{:.1}{unit}\" ",
                    ram_used as f32 / BYTES_PER_GB
                ));
            }
            "ram_usage" => {
                let unit = if no_units { "" } else { "%" };
                result.push(format!("RAM_USAGE=\"{ram_usage_percentage}{unit}\" "));
            }
            "swp_free" => {
                let unit = if no_units { "" } else { "GB" };
                result.push(format!(
                    "SWP_FREE=\"{:.1}{unit}\" ",
                    s.free_swap() as f32 / BYTES_PER_GB
                ));
            }
            "swp_total" => {
                let unit = if no_units { "" } else { "GB" };
                result.push(format!(
                    "SWP_TOTAL=\"{:.1}{unit}\" ",
                    swp_total as f32 / BYTES_PER_GB
                ));
            }
            "swp_used" => {
                let unit = if no_units { "" } else { "GB" };
                result.push(format!(
                    "SWP_USED=\"{:.1}{unit}\" ",
                    swp_used as f32 / BYTES_PER_GB
                ));
            }
            "swp_usage" => {
                let unit = if no_units { "" } else { "%" };
                result.push(format!("SWP_USAGE=\"{swp_usage_percentage}{unit}\" "));
            }
            _ => {}
        }
    }

    result
}
