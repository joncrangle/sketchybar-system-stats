use std::fmt::Write;
use sysinfo::System;

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
                let _ = write!(buf, "HOST_NAME=\"{}\" ", System::host_name().unwrap());
            }
            "kernel_version" => {
                let _ = write!(
                    buf,
                    "KERNEL_VERSION=\"{}\" ",
                    System::kernel_version().unwrap()
                );
            }
            "name" => {
                let _ = write!(buf, "SYSTEM_NAME=\"{}\" ", System::name().unwrap());
            }
            "os_version" => {
                let _ = write!(buf, "OS_VERSION=\"{}\" ", System::os_version().unwrap());
            }
            "long_os_version" => {
                let _ = write!(
                    buf,
                    "LONG_OS_VERSION=\"{}\" ",
                    System::long_os_version().unwrap()
                );
            }
            _ => {}
        }
    }
}
