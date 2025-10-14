use std::fmt::Write;
use sysinfo::{Components, System};

pub fn get_cpu_stats(
    s: &System,
    components: &Components,
    flags: &[&str],
    no_units: bool,
    buf: &mut String,
) {
    let cpu_count = s.cpus().len() as f32;

    for &flag in flags {
        match flag {
            "count" => {
                let _ = write!(buf, "CPU_COUNT=\"{cpu_count}\" ");
            }
            "frequency" => {
                let total_frequency: u64 = s.cpus().iter().map(|cpu| cpu.frequency()).sum();
                let avg_freq = total_frequency / cpu_count as u64;
                let unit = if no_units { "" } else { "MHz" };
                let _ = write!(buf, "CPU_FREQUENCY=\"{avg_freq}{unit}\" ");
            }
            "temperature" => {
                let mut total_temp: f32 = 0.0;
                let mut count: u32 = 0;

                let cpu_labels = ["CPU", "PMU", "SOC"];

                for component in components {
                    if cpu_labels
                        .iter()
                        .any(|&label| component.label().contains(label))
                    {
                        if let Some(temperature) = component.temperature() {
                            total_temp += temperature;
                            count += 1;
                        }
                    }
                }

                let average_temp = if count > 0 {
                    total_temp / count as f32
                } else {
                    -1.0
                };

                let unit = if no_units { "" } else { "°C" };
                if average_temp != -1.0 {
                    let _ = write!(buf, "CPU_TEMP=\"{average_temp:.1}{unit}\" ");
                } else {
                    let _ = write!(buf, "CPU_TEMP=\"N/A{unit}\" ");
                }
            }
            "usage" => {
                let unit = if no_units { "" } else { "%" };
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
