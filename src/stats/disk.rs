use std::collections::HashSet;
use std::fmt::Write;
use sysinfo::Disks;

use super::{BYTES_PER_GB, PERCENT, unit};

fn apfs_container_id(disk_name: &str) -> &str {
    if let Some(idx) = disk_name.find("disk") {
        let after_disk = &disk_name[idx + 4..];
        let digit_len = after_disk
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .count();
        if digit_len > 0 {
            let end_of_base = idx + 4 + digit_len;
            if disk_name[end_of_base..].starts_with('s') {
                return &disk_name[..end_of_base];
            }
        }
    }
    disk_name
}

#[cfg(target_os = "macos")]
fn get_mount_device_name(mount_point: &std::path::Path) -> Option<String> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    // Mount-device lookup is best-effort: callers can still deduplicate by the
    // sysinfo disk name when a path cannot be represented for `statfs`.
    let path = CString::new(mount_point.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(path.as_ptr(), &mut stat) } == 0 {
        let name_bytes: Vec<u8> = stat
            .f_mntfromname
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as u8)
            .collect();
        String::from_utf8(name_bytes).ok()
    } else {
        None
    }
}

#[cfg(not(target_os = "macos"))]
fn get_mount_device_name(_mount_point: &std::path::Path) -> Option<String> {
    None
}

pub fn get_disk_stats(disks: &Disks, flags: &[&str], no_units: bool, buf: &mut String) {
    let mut seen_containers = HashSet::new();
    let unique_disks: Vec<_> = disks
        .list()
        .iter()
        .filter(|disk| {
            let is_apfs = disk
                .file_system()
                .to_string_lossy()
                .eq_ignore_ascii_case("apfs");
            let device_name = get_mount_device_name(disk.mount_point())
                .unwrap_or_else(|| disk.name().to_string_lossy().into_owned());

            let key = if is_apfs {
                apfs_container_id(&device_name).to_string()
            } else {
                device_name
            };

            seen_containers.insert(key)
        })
        .collect();

    let disk_count = unique_disks.len();
    let (total_space, used_space) = unique_disks.iter().fold((0, 0), |(total, used), disk| {
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
                let _ = write!(buf, "DISK_COUNT=\"{disk_count}\" ");
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
    fn test_get_disk_stats_all_flags_emit_expected_keys() {
        use crate::cli;

        let disks = Disks::new_with_refreshed_list();
        let mut buf = String::new();

        get_disk_stats(&disks, cli::ALL_DISK_FLAGS, false, &mut buf);

        assert!(buf.contains("DISK_COUNT="));
        assert!(buf.contains("DISK_FREE="));
        assert!(buf.contains("DISK_TOTAL="));
        assert!(buf.contains("DISK_USED="));
        assert!(buf.contains("DISK_USAGE="));
    }

    #[test]
    fn test_get_disk_stats_unknown_flag_ignored() {
        let disks = Disks::new_with_refreshed_list();
        let mut buf = String::new();

        get_disk_stats(&disks, &["bogus"], false, &mut buf);

        assert_eq!(buf, "");
    }

    #[test]
    fn test_get_disk_stats_no_units() {
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

    #[test]
    fn test_apfs_container_id_extraction() {
        assert_eq!(apfs_container_id("/dev/disk3s1s1"), "/dev/disk3");
        assert_eq!(apfs_container_id("/dev/disk3s5"), "/dev/disk3");
        assert_eq!(apfs_container_id("disk1s2"), "disk1");
        assert_eq!(apfs_container_id("/dev/disk4"), "/dev/disk4");
        assert_eq!(apfs_container_id("custom_volume"), "custom_volume");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_mount_device_name_root() {
        let dev = get_mount_device_name(std::path::Path::new("/"));
        assert!(dev.is_some());
        assert!(dev.unwrap().starts_with("/dev/disk"));
    }
}
