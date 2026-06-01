use std::fmt::Write;
use sysinfo::Networks;

fn network_key_suffix(interface: &str) -> String {
    interface
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

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
            let key_suffix = network_key_suffix(interface);
            let _ = write!(
                buf,
                "NETWORK_RX_{}=\"{}{unit}\" NETWORK_TX_{}=\"{}{unit}\" ",
                key_suffix,
                (data.received() / 1024) / interval as u64,
                key_suffix,
                (data.transmitted() / 1024) / interval as u64
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_key_suffix_normalizes_interface_name() {
        assert_eq!(network_key_suffix("en0"), "en0");
        assert_eq!(network_key_suffix("bridge.100"), "bridge_100");
        assert_eq!(network_key_suffix("utun-1"), "utun_1");
    }
}
