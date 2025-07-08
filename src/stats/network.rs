use sysinfo::Networks;

pub fn get_network_stats(
    n: &Networks,
    interfaces: Option<&[String]>,
    interval: u32,
) -> Vec<String> {
    let mut result = Vec::new();

    let interfaces_to_check: Vec<&str> = match interfaces {
        Some(ifaces) => ifaces.iter().map(String::as_str).collect(),
        None => n
            .keys()
            .map(|interface_name| interface_name.as_str())
            .collect(),
    };

    for interface in interfaces_to_check {
        if let Some(data) = n.get(interface) {
            result.push(format!(
                "NETWORK_RX_{}=\"{}KB/s\" NETWORK_TX_{}=\"{}KB/s\" ",
                interface,
                (data.received() / 1024) / interval as u64,
                interface,
                (data.transmitted() / 1024) / interval as u64
            ));
        }
    }
    result
}
