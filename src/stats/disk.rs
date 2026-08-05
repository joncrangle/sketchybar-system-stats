use super::{BYTES_PER_GB, PERCENT, unit};
use std::fmt::Write;
use sysinfo::Disks;

pub fn get_disk_stats(disks: &Disks, flags: &[&str], no_units: bool, buf: &mut String) {
    let (total_space, used_space) = disks.list().iter().fold((0, 0), |(total, used), disk| {
        (
            total + disk.total_space(),
            used + disk.total_space() - disk.available_space(),
        )
    });
    let disk_usage_percentage = if total_space > 0 {
        ((used_space as f32 / total_space as f32) * PERCENT).round() as u32
    } else {
        0
    };

    for &flag in flags {
        match flag {
            "count" => {
                let _ = write!(buf, "DISK_COUNT=\"{}\" ", disks.list().len());
            }
            "free" => {
                let unit = unit(no_units, "GB");
                let _ = write!(
                    buf,
                    "DISK_FREE=\"{:.1}{unit}\" ",
                    (total_space as f32 - used_space as f32) / BYTES_PER_GB
                );
            }
            "total" => {
                let unit = unit(no_units, "GB");
                let _ = write!(
                    buf,
                    "DISK_TOTAL=\"{:.1}{unit}\" ",
                    total_space as f32 / BYTES_PER_GB
                );
            }
            "used" => {
                let unit = unit(no_units, "GB");
                let _ = write!(
                    buf,
                    "DISK_USED=\"{:.1}{unit}\" ",
                    used_space as f32 / BYTES_PER_GB
                );
            }
            "usage" => {
                let unit = unit(no_units, "%");
                let _ = write!(buf, "DISK_USAGE=\"{disk_usage_percentage}{unit}\" ");
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_disk_stats_with_units() {
        let disks = Disks::new_with_refreshed_list();
        let mut buf = String::new();

        get_disk_stats(&disks, &["count", "total"], false, &mut buf);

        assert!(buf.contains("DISK_COUNT="));
    }

    #[test]
    fn test_get_disk_stats_without_units() {
        let disks = Disks::new_with_refreshed_list();
        let mut buf = String::new();

        get_disk_stats(&disks, &["total"], true, &mut buf);

        if !buf.is_empty() {
            assert!(!buf.contains("GB"));
        }
    }

    #[test]
    fn test_get_disk_stats_empty_flags() {
        let disks = Disks::new_with_refreshed_list();
        let mut buf = String::new();

        get_disk_stats(&disks, &[], false, &mut buf);

        assert_eq!(buf, "");
    }
}
