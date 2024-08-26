use sysinfo::Disks;

const BYTES_PER_GB: f32 = 1_073_741_824.0;

pub fn get_disk_stats(disks: &Disks, flags: &[&str]) -> String {
    let (total_space, used_space) = disks.list().iter().fold((0, 0), |(total, used), disk| {
        (
            total + disk.total_space(),
            used + disk.total_space() - disk.available_space(),
        )
    });
    let disk_usage_percentage = if total_space > 0 {
        ((used_space as f32 / total_space as f32) * 100.0).round() as u32
    } else {
        0
    };

    let mut result = String::new();

    for &flag in flags {
        match flag {
            "count" => {
                result.push_str(&format!("DISK_COUNT=\"{}\" ", disks.list().len()));
            }
            "free" => {
                result.push_str(&format!(
                    "DISK_FREE=\"{:.1}GB\" ",
                    (total_space as f32 - used_space as f32) / BYTES_PER_GB
                ));
            }
            "total" => {
                result.push_str(&format!(
                    "DISK_TOTAL=\"{:.1}GB\" ",
                    total_space as f32 / BYTES_PER_GB
                ));
            }
            "used" => {
                result.push_str(&format!(
                    "DISK_USED=\"{:.1}GB\" ",
                    used_space as f32 / BYTES_PER_GB
                ));
            }
            "usage" => {
                result.push_str(&format!("DISK_USAGE=\"{}%\" ", disk_usage_percentage));
            }
            _ => {}
        }
    }

    result.trim_end().to_string()
}
