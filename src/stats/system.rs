use std::fmt::Write;
use sysinfo::System;

use crate::sketchybar::escape_quoted_value;

fn system_value(value: Option<&str>) -> &str {
    value.unwrap_or("N/A")
}

fn write_system_stat(key: &str, value: &str, buf: &mut String) {
    let _ = write!(buf, "{key}=\"{}\" ", escape_quoted_value(value));
}

pub fn get_system_stats(flags: &[&str], buf: &mut String) {
    for &flag in flags {
        match flag {
            "arch" => {
                write_system_stat("ARCH", &System::cpu_arch(), buf);
            }
            "distro" => {
                write_system_stat("DISTRO", &System::distribution_id(), buf);
            }

            "host_name" => {
                write_system_stat(
                    "HOST_NAME",
                    system_value(System::host_name().as_deref()),
                    buf,
                );
            }
            "kernel_version" => {
                write_system_stat(
                    "KERNEL_VERSION",
                    system_value(System::kernel_version().as_deref()),
                    buf,
                );
            }
            "name" => {
                write_system_stat("SYSTEM_NAME", system_value(System::name().as_deref()), buf);
            }
            "os_version" => {
                write_system_stat(
                    "OS_VERSION",
                    system_value(System::os_version().as_deref()),
                    buf,
                );
            }
            "long_os_version" => {
                write_system_stat(
                    "LONG_OS_VERSION",
                    system_value(System::long_os_version().as_deref()),
                    buf,
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
    fn test_get_system_stats_all_flags_emit_expected_keys() {
        use crate::cli;

        let mut buf = String::new();

        get_system_stats(cli::ALL_SYSTEM_FLAGS, &mut buf);

        assert!(buf.contains("ARCH="));
        assert!(buf.contains("DISTRO="));
        assert!(buf.contains("HOST_NAME="));
        assert!(buf.contains("KERNEL_VERSION="));
        assert!(buf.contains("SYSTEM_NAME="));
        assert!(buf.contains(" OS_VERSION=\""));
        assert!(buf.contains("LONG_OS_VERSION="));
    }

    #[test]
    fn test_system_value_some_and_none() {
        assert_eq!(system_value(Some("darwin")), "darwin");
        assert_eq!(system_value(None), "N/A");
    }

    #[test]
    fn test_write_system_stat_escapes_quotes_and_backslashes() {
        let mut buf = String::new();
        write_system_stat("HOST_NAME", "Alice's \"Mac\"\\work", &mut buf);

        assert_eq!(buf, "HOST_NAME=\"Alice's \\\"Mac\\\"\\\\work\" ");
    }
}
