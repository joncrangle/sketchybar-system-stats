use sysinfo::{Components, System};

pub fn get_cpu_stats(s: &System, flags: &[&str], no_units: bool) -> Vec<String> {
    let cpu_count = s.cpus().len() as f32;

    let mut result = Vec::new();

    for &flag in flags {
        match flag {
            "count" => {
                result.push(format!("CPU_COUNT=\"{cpu_count}\" "));
            }
            "frequency" => {
                let total_frequency: u64 = s.cpus().iter().map(|cpu| cpu.frequency()).sum();
                let avg_freq = total_frequency / cpu_count as u64;
                let unit = if no_units { "" } else { "MHz" };
                result.push(format!("CPU_FREQUENCY=\"{avg_freq}{unit}\" "));
            }
            "temperature" => {
                let components = Components::new_with_refreshed_list();
                let mut total_temp: f32 = 0.0;
                let mut count: u32 = 0;

                let cpu_labels = ["CPU", "PMU", "SOC"];

                for component in &components {
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

                let formatted_temp = if average_temp != -1.0 {
                    format!("{average_temp:.1}")
                } else {
                    "N/A".to_string()
                };

                let unit = if no_units { "" } else { "°C" };
                result.push(format!("CPU_TEMP=\"{formatted_temp}{unit}\" "));
            }
            "usage" => {
                let unit = if no_units { "" } else { "%" };
                result.push(format!(
                    "CPU_USAGE=\"{:.0}{unit}\" ",
                    s.global_cpu_usage().round()
                ));
            }
            _ => {}
        }
    }

    result
}
