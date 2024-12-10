use sysinfo::System;

pub fn get_system_stats(flags: &[&str]) -> Vec<String> {
    let mut result = Vec::new();
    for &flag in flags {
        match flag {
            "arch" => {
                result.push(format!("ARCH=\"{}\" ", System::cpu_arch()));
            }
            "distro" => {
                result.push(format!("DISTRO=\"{}\" ", System::distribution_id()));
            }

            "host_name" => {
                result.push(format!("HOST_NAME=\"{}\" ", System::host_name().unwrap()));
            }
            "kernel_version" => {
                result.push(format!(
                    "KERNEL_VERSION=\"{}\" ",
                    System::kernel_version().unwrap()
                ));
            }
            "name" => {
                result.push(format!("SYSTEM_NAME=\"{}\" ", System::name().unwrap()));
            }
            "os_version" => {
                result.push(format!("OS_VERSION=\"{}\" ", System::os_version().unwrap()));
            }
            "long_os_version" => {
                result.push(format!(
                    "LONG_OS_VERSION=\"{}\" ",
                    System::long_os_version().unwrap()
                ));
            }
            "uptime" => {
                result.push(format!("UPTIME=\"{} mins\" ", System::uptime() / 60));
            }
            _ => {}
        }
    }
    result
}
