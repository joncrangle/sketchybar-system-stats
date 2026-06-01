use std::fmt::Write;
use sysinfo::System;

fn system_value(value: Option<String>) -> String {
    value.unwrap_or_else(|| "N/A".to_string())
}

pub fn get_system_stats(flags: &[&str], buf: &mut String) {
    for &flag in flags {
        match flag {
            "arch" => {
                let _ = write!(buf, "ARCH=\"{}\" ", System::cpu_arch());
            }
            "distro" => {
                let _ = write!(buf, "DISTRO=\"{}\" ", System::distribution_id());
            }

            "host_name" => {
                let _ = write!(buf, "HOST_NAME=\"{}\" ", system_value(System::host_name()));
            }
            "kernel_version" => {
                let _ = write!(
                    buf,
                    "KERNEL_VERSION=\"{}\" ",
                    system_value(System::kernel_version())
                );
            }
            "name" => {
                let _ = write!(buf, "SYSTEM_NAME=\"{}\" ", system_value(System::name()));
            }
            "os_version" => {
                let _ = write!(
                    buf,
                    "OS_VERSION=\"{}\" ",
                    system_value(System::os_version())
                );
            }
            "long_os_version" => {
                let _ = write!(
                    buf,
                    "LONG_OS_VERSION=\"{}\" ",
                    system_value(System::long_os_version())
                );
            }
            _ => {}
        }
    }
}
