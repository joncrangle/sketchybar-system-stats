use sysinfo::{Components, System};

pub fn get_cpu_stats(s: &System, flags: &[&str]) -> String {
    let cpu_count = s.cpus().len() as f32;
    let total_usage: f32 = s.cpus().iter().map(|cpu| cpu.cpu_usage()).sum();

    let mut result = String::new();

    for &flag in flags {
        match flag {
            "count" => {
                result.push_str(&format!("CPU_COUNT=\"{}\" ", cpu_count));
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
                        total_temp += component.temperature();
                        count += 1;
                    }
                }

                let average_temp = if count > 0 {
                    total_temp / count as f32
                } else {
                    -1.0
                };

                let formatted_temp = if average_temp != -1.0 {
                    format!("{:.1}", average_temp)
                } else {
                    "N/A".to_string()
                };

                result.push_str(&format!("CPU_TEMP=\"{}°C\" ", formatted_temp));
            }
            "usage" => {
                let avg_cpu_usage: f32 = (total_usage / cpu_count).round();
                result.push_str(&format!("CPU_USAGE=\"{:.0}%\" ", avg_cpu_usage));
            }
            _ => {}
        }
    }

    result.trim_end().to_string()
}
