use sysinfo::System;

pub fn get_system_stats(flags: &[&str]) -> String {
    let mut result = String::new();
    for &flag in flags {
        match flag {
            "arch" => {
                result.push_str(&format!("ARCH=\"{}\" ", System::cpu_arch().unwrap()));
            }
            "distro" => {
                result.push_str(&format!("DISTRO=\"{}\" ", System::distribution_id()));
            }

            "host_name" => {
                result.push_str(&format!("HOST_NAME=\"{}\" ", System::host_name().unwrap()));
            }
            "kernel_version" => {
                result.push_str(&format!(
                    "KERNEL_VERSION=\"{}\" ",
                    System::kernel_version().unwrap()
                ));
            }
            "name" => {
                result.push_str(&format!("SYSTEM_NAME=\"{}\" ", System::name().unwrap()));
            }
            "os_version" => {
                result.push_str(&format!(
                    "OS_VERSION=\"{}\" ",
                    System::os_version().unwrap()
                ));
            }
            "long_os_version" => {
                result.push_str(&format!(
                    "LONG_OS_VERSION=\"{}\" ",
                    System::long_os_version().unwrap()
                ));
            }
            "uptime" => {
                result.push_str(&format!("UPTIME=\"{} mins\" ", System::uptime() / 60));
            }
            _ => {}
        }
    }
    result.trim_end().to_string()
}
