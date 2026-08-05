use std::collections::HashMap;
use std::fmt::Write;
use std::time::Instant;

use super::{BYTES_PER_KB, unit};
use sysinfo::Networks;

/// Per-interface counters used to compute transfer rates between ticks.
struct InterfaceBaseline {
    rx_total: u64,
    tx_total: u64,
    at: Instant,
}

/// Collector-owned baselines keyed by network interface name.
#[derive(Default)]
pub struct NetworkRateBaselines {
    by_interface: HashMap<String, InterfaceBaseline>,
}

impl NetworkRateBaselines {
    /// Resets the baseline for `interface` to the current cumulative totals.
    fn reset(&mut self, interface: &str, rx_total: u64, tx_total: u64) {
        self.by_interface.insert(
            interface.to_owned(),
            InterfaceBaseline {
                rx_total,
                tx_total,
                at: Instant::now(),
            },
        );
    }
}

fn network_key_suffix(interface: &str) -> String {
    interface
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Converts a byte delta and the elapsed time into a rate in `KiB/s`.
fn rate_kib_per_sec(delta_bytes: u64, elapsed_secs: f64) -> u64 {
    if elapsed_secs <= 0.0 {
        return 0;
    }
    ((delta_bytes / BYTES_PER_KB) as f64 / elapsed_secs).round() as u64
}

/// Computes rx/tx rates in `KiB/s` from the previous and current cumulative totals.
///
/// Returns `(0, 0)` when there is no previous baseline (first sighting of an
/// interface) or when the cumulative counters wrapped around.
fn compute_rates(
    prev_rx: Option<u64>,
    prev_tx: Option<u64>,
    rx_total: u64,
    tx_total: u64,
    elapsed_secs: f64,
) -> (u64, u64) {
    let (Some(prev_rx), Some(prev_tx)) = (prev_rx, prev_tx) else {
        return (0, 0);
    };

    if rx_total < prev_rx || tx_total < prev_tx {
        return (0, 0);
    }

    (
        rate_kib_per_sec(rx_total - prev_rx, elapsed_secs),
        rate_kib_per_sec(tx_total - prev_tx, elapsed_secs),
    )
}

pub fn get_network_stats(
    n: &Networks,
    interfaces: Option<&[String]>,
    baselines: &mut NetworkRateBaselines,
    no_units: bool,
    buf: &mut String,
) {
    let interfaces_to_check: Vec<&str> = match interfaces {
        Some(ifaces) => ifaces.iter().map(String::as_str).collect(),
        None => n
            .keys()
            .map(|interface_name| interface_name.as_str())
            .collect(),
    };

    let unit = unit(no_units, "KiB/s");

    for interface in interfaces_to_check {
        if let Some(data) = n.get(interface) {
            let key_suffix = network_key_suffix(interface);
            let rx_total = data.total_received();
            let tx_total = data.total_transmitted();

            let (rx_rate, tx_rate) = match baselines.by_interface.get(interface) {
                Some(baseline) => {
                    let elapsed = baseline.at.elapsed().as_secs_f64();
                    let rates = compute_rates(
                        Some(baseline.rx_total),
                        Some(baseline.tx_total),
                        rx_total,
                        tx_total,
                        elapsed,
                    );

                    baselines.reset(interface, rx_total, tx_total);

                    rates
                }
                None => {
                    baselines.reset(interface, rx_total, tx_total);
                    (0, 0)
                }
            };

            let _ = write!(
                buf,
                "NETWORK_RX_{}=\"{rx_rate}{unit}\" NETWORK_TX_{}=\"{tx_rate}{unit}\" ",
                key_suffix, key_suffix
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

    #[test]
    fn test_rate_kib_per_sec_typical_rate() {
        assert_eq!(rate_kib_per_sec(2048, 2.0), 1);
    }

    #[test]
    fn test_rate_kib_per_sec_sub_kib_rate_rounds_down() {
        assert_eq!(rate_kib_per_sec(500, 1.0), 0);
    }

    #[test]
    fn test_rate_kib_per_sec_rounds_to_nearest() {
        assert_eq!(rate_kib_per_sec(3072, 1.0), 3);
    }

    #[test]
    fn test_rate_kib_per_sec_zero_elapsed() {
        assert_eq!(rate_kib_per_sec(1024, 0.0), 0);
    }

    #[test]
    fn test_rate_kib_per_sec_pins_1024_divisor() {
        assert_eq!(rate_kib_per_sec(1500, 1.0), 1);
    }

    #[test]
    fn test_rate_kib_per_sec_fractional_elapsed() {
        assert_eq!(rate_kib_per_sec(1024, 0.5), 2);
    }

    #[test]
    fn test_compute_rates_first_sighting_returns_zero() {
        assert_eq!(compute_rates(None, None, 1000, 2000, 1.0), (0, 0));
    }

    #[test]
    fn test_compute_rates_counter_wrap_returns_zero() {
        assert_eq!(
            compute_rates(Some(1000), Some(2000), 500, 2500, 1.0),
            (0, 0)
        );
    }

    #[test]
    fn test_compute_rates_normal_delta() {
        assert_eq!(
            compute_rates(Some(2048), Some(4096), 4096, 6144, 1.0),
            (2, 2)
        );
    }
}
