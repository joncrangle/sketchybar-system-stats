use std::fmt::Write;
use sysinfo::{Components, System};

use super::unit;

/// Returns whether a temperature component represents a CPU sensor.
///
/// Intel sensor labels include `CPU` (for example `CPU Proximity`). Apple
/// Silicon exposes performance and efficiency core temperatures through the
/// `pACC` and `eACC` MTR sensors. PMU and SoC readings are not CPU core
/// temperatures and must not be averaged into the result.
fn is_cpu_temperature_sensor(label: &str) -> bool {
    label.starts_with("CPU") || label.starts_with("pACC") || label.starts_with("eACC")
}

pub fn get_cpu_stats(
    s: &System,
    components: &Components,
    flags: &[&str],
    no_units: bool,
    buf: &mut String,
) {
    let cpu_count = s.cpus().len();

    if cpu_count == 0 {
        return;
    }

    for &flag in flags {
        match flag {
            "count" => {
                let _ = write!(buf, "CPU_COUNT=\"{cpu_count}\" ");
            }
            "frequency" => {
                let total_frequency: u64 = s.cpus().iter().map(|cpu| cpu.frequency()).sum();
                let avg_freq = total_frequency / cpu_count as u64;
                let unit = unit(no_units, "MHz");
                let _ = write!(buf, "CPU_FREQUENCY=\"{avg_freq}{unit}\" ");
            }
            "temperature" => {
                let mut total_temp: f32 = 0.0;
                let mut count: u32 = 0;

                for component in components {
                    if is_cpu_temperature_sensor(component.label())
                        && let Some(temperature) = component.temperature()
                    {
                        total_temp += temperature;
                        count += 1;
                    }
                }

                let average_temp = if count > 0 {
                    Some(total_temp / count as f32)
                } else {
                    None
                };

                if let Some(average_temp) = average_temp {
                    let unit = unit(no_units, "°C");
                    let _ = write!(buf, "CPU_TEMP=\"{average_temp:.1}{unit}\" ");
                } else {
                    let _ = write!(buf, "CPU_TEMP=\"N/A\" ");
                }
            }
            "usage" => {
                let unit = unit(no_units, "%");
                let _ = write!(
                    buf,
                    "CPU_USAGE=\"{:.0}{unit}\" ",
                    s.global_cpu_usage().round()
                );
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cpu_temperature_sensor_selects_cpu_core_labels() {
        assert!(is_cpu_temperature_sensor("CPU Proximity"));
        assert!(is_cpu_temperature_sensor("pACC MTR Temp Sensor"));
        assert!(is_cpu_temperature_sensor("eACC MTR Temp Sensor"));
    }

    #[test]
    fn test_is_cpu_temperature_sensor_excludes_pmu_and_soc_labels() {
        assert!(!is_cpu_temperature_sensor("PMU MTR Temp Sensor"));
        assert!(!is_cpu_temperature_sensor("SOC MTR Temp Sensor"));
        assert!(!is_cpu_temperature_sensor("GPU Proximity"));
    }

    #[test]
    fn test_get_cpu_stats_all_flags_emit_expected_keys() {
        use crate::cli;

        let mut s = System::new_all();
        s.refresh_all();
        let components = Components::new_with_refreshed_list();
        let mut buf = String::new();

        get_cpu_stats(&s, &components, cli::ALL_CPU_FLAGS, false, &mut buf);

        assert!(buf.contains("CPU_COUNT="));
        assert!(buf.contains("CPU_FREQUENCY="));
        assert!(buf.contains("CPU_TEMP="));
        assert!(buf.contains("CPU_USAGE="));
        assert!(buf.contains("%"));

        let mut no_units_buf = String::new();
        get_cpu_stats(&s, &components, cli::ALL_CPU_FLAGS, true, &mut no_units_buf);

        assert!(no_units_buf.contains("CPU_USAGE="));
        assert!(!no_units_buf.contains("%"));
        assert!(!no_units_buf.contains("MHz"));
        assert!(!no_units_buf.contains("°C"));
    }

    #[test]
    fn test_get_cpu_stats_empty_flags() {
        let mut s = System::new_all();
        s.refresh_all();
        let components = Components::new_with_refreshed_list();
        let mut buf = String::new();

        get_cpu_stats(&s, &components, &[], false, &mut buf);

        assert_eq!(buf, "");
    }

    #[test]
    fn test_get_cpu_stats_temperature_na_has_no_unit() {
        let s = System::new_all();
        let components = Components::new();
        let mut buf = String::new();

        get_cpu_stats(&s, &components, &["temperature"], false, &mut buf);

        assert_eq!(buf, "CPU_TEMP=\"N/A\" ");
    }

    #[test]
    fn test_get_cpu_stats_invalid_flag() {
        let mut s = System::new_all();
        s.refresh_all();
        let components = Components::new_with_refreshed_list();
        let mut buf = String::new();

        get_cpu_stats(&s, &components, &["invalid_flag"], false, &mut buf);

        assert_eq!(buf, "");
    }
}
