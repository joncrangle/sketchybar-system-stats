use std::fmt::Write;
use sysinfo::Disks;

const BYTES_PER_GB: f32 = 1_073_741_824.0;

pub fn get_disk_stats(disks: &Disks, flags: &[&str], no_units: bool, buf: &mut String) {
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

    for &flag in flags {
        match flag {
            "count" => {
                let _ = write!(buf, "DISK_COUNT=\"{}\" ", disks.list().len());
            }
            "free" => {
                let unit = if no_units { "" } else { "GB" };
                let _ = write!(
                    buf,
                    "DISK_FREE=\"{:.1}{unit}\" ",
                    (total_space as f32 - used_space as f32) / BYTES_PER_GB
                );
            }
            "total" => {
                let unit = if no_units { "" } else { "GB" };
                let _ = write!(
                    buf,
                    "DISK_TOTAL=\"{:.1}{unit}\" ",
                    total_space as f32 / BYTES_PER_GB
                );
            }
            "used" => {
                let unit = if no_units { "" } else { "GB" };
                let _ = write!(
                    buf,
                    "DISK_USED=\"{:.1}{unit}\" ",
                    used_space as f32 / BYTES_PER_GB
                );
            }
            "usage" => {
                let unit = if no_units { "" } else { "%" };
                let _ = write!(buf, "DISK_USAGE=\"{disk_usage_percentage}{unit}\" ");
            }
            _ => {}
        }
    }
}
