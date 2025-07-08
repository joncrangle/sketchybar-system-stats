use sysinfo::Disks;

const BYTES_PER_GB: f32 = 1_073_741_824.0;

pub fn get_disk_stats(disks: &Disks, flags: &[&str]) -> Vec<String> {
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

    let mut result = Vec::new();

    for &flag in flags {
        match flag {
            "count" => {
                result.push(format!("DISK_COUNT=\"{}\" ", disks.list().len()));
            }
            "free" => {
                result.push(format!(
                    "DISK_FREE=\"{:.1}GB\" ",
                    (total_space as f32 - used_space as f32) / BYTES_PER_GB
                ));
            }
            "total" => {
                result.push(format!(
                    "DISK_TOTAL=\"{:.1}GB\" ",
                    total_space as f32 / BYTES_PER_GB
                ));
            }
            "used" => {
                result.push(format!(
                    "DISK_USED=\"{:.1}GB\" ",
                    used_space as f32 / BYTES_PER_GB
                ));
            }
            "usage" => {
                result.push(format!("DISK_USAGE=\"{disk_usage_percentage}%\" "));
            }
            _ => {}
        }
    }

    result
}
