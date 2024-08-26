use crate::cli;
use sysinfo::System;

const BYTES_PER_GB: f32 = 1_073_741_824.0;

pub fn get_memory_stats(s: &System, flags: &[&str]) -> String {
    let mut result = String::new();

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
                result.push_str(&format!(
                    "RAM_AVAILABLE=\"{:.1}GB\" ",
                    s.available_memory() as f32 / BYTES_PER_GB
                ));
            }
            "ram_total" => {
                result.push_str(&format!(
                    "RAM_TOTAL=\"{:.1}GB\" ",
                    ram_total as f32 / BYTES_PER_GB
                ));
            }
            "ram_used" => {
                result.push_str(&format!(
                    "RAM_USED=\"{:.1}GB\" ",
                    ram_used as f32 / BYTES_PER_GB
                ));
            }
            "ram_usage" => {
                result.push_str(&format!("RAM_USAGE=\"{}%\" ", ram_usage_percentage));
            }
            "swp_free" => {
                result.push_str(&format!(
                    "SWP_FREE=\"{:.1}GB\" ",
                    s.free_swap() as f32 / BYTES_PER_GB
                ));
            }
            "swp_total" => {
                result.push_str(&format!(
                    "SWP_TOTAL=\"{:.1}GB\" ",
                    swp_total as f32 / BYTES_PER_GB
                ));
            }
            "swp_used" => {
                result.push_str(&format!(
                    "SWP_USED=\"{:.1}GB\" ",
                    swp_used as f32 / BYTES_PER_GB
                ));
            }
            "swp_usage" => {
                result.push_str(&format!("SWP_USAGE=\"{}%\" ", swp_usage_percentage));
            }
            _ => {}
        }
    }

    result.trim_end().to_string()
}
