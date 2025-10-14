use std::fmt::Write;
use sysinfo::Networks;

pub fn get_network_stats(
    n: &Networks,
    interfaces: Option<&[String]>,
    interval: u32,
    no_units: bool,
    buf: &mut String,
) {
    if interval == 0 {
        return;
    }

    let interfaces_to_check: Vec<&str> = match interfaces {
        Some(ifaces) => ifaces.iter().map(String::as_str).collect(),
        None => n
            .keys()
            .map(|interface_name| interface_name.as_str())
            .collect(),
    };

    let unit = if no_units { "" } else { "KB/s" };

    for interface in interfaces_to_check {
        if let Some(data) = n.get(interface) {
            let _ = write!(
                buf,
                "NETWORK_RX_{}=\"{}{unit}\" NETWORK_TX_{}=\"{}{unit}\" ",
                interface,
                (data.received() / 1024) / interval as u64,
                interface,
                (data.transmitted() / 1024) / interval as u64
            );
        }
    }
}
