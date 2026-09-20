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
    /// Retains only baselines for interfaces that match the predicate.
    pub fn retain_active(&mut self, mut is_active: impl FnMut(&str) -> bool) {
        self.by_interface
            .retain(|interface, _| is_active(interface.as_str()));
    }

    /// Updates the baseline for `interface` and returns the transfer rates in `(rx, tx)` KiB/s.
    pub fn update(&mut self, interface: &str, rx_total: u64, tx_total: u64) -> (u64, u64) {
        if let Some(baseline) = self.by_interface.get_mut(interface) {
            let elapsed = baseline.at.elapsed().as_secs_f64();
            let rates = compute_rates(
                Some(baseline.rx_total),
                Some(baseline.tx_total),
                rx_total,
                tx_total,
                elapsed,
            );
            baseline.rx_total = rx_total;
            baseline.tx_total = tx_total;
            baseline.at = Instant::now();
            rates
        } else {
            self.by_interface.insert(
                interface.to_owned(),
                InterfaceBaseline {
                    rx_total,
                    tx_total,
                    at: Instant::now(),
                },
            );
            (0, 0)
        }
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
    if elapsed_secs <= 0.0 || !elapsed_secs.is_finite() {
        return 0;
    }
    ((delta_bytes as f64 / BYTES_PER_KB as f64) / elapsed_secs).round() as u64
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
    baselines.retain_active(|iface| n.get(iface).is_some());

    let unit = unit(no_units, "KiB/s");

    let mut emit_stat = |interface: &str, data: &sysinfo::NetworkData| {
        let key_suffix = network_key_suffix(interface);
        let rx_total = data.total_received();
        let tx_total = data.total_transmitted();
        let (rx_rate, tx_rate) = baselines.update(interface, rx_total, tx_total);

        let _ = write!(
            buf,
            "NETWORK_RX_{}=\"{rx_rate}{unit}\" NETWORK_TX_{}=\"{tx_rate}{unit}\" ",
            key_suffix, key_suffix
        );
    };

    match interfaces {
        Some(ifaces) => {
            for interface in ifaces {
                if let Some(data) = n.get(interface.as_str()) {
                    emit_stat(interface, data);
                }
            }
        }
        None => {
            for (interface, data) in n {
                emit_stat(interface, data);
            }
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
    fn test_rate_kib_per_sec_fractional_sub_kib_rate() {
        // 1000 bytes over 0.5s = 2000 B/s = 1.953 KiB/s -> rounds to 2 (would fail with integer div)
        assert_eq!(rate_kib_per_sec(1000, 0.5), 2);
    }

    #[test]
    fn test_network_baselines_retain_active() {
        let mut baselines = NetworkRateBaselines::default();
        baselines.update("en0", 100, 100);
        baselines.update("utun0", 200, 200);

        baselines.retain_active(|iface| iface == "en0");

        assert!(baselines.by_interface.contains_key("en0"));
        assert!(!baselines.by_interface.contains_key("utun0"));
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
    fn test_rate_kib_per_sec_nan_or_infinite_elapsed() {
        assert_eq!(rate_kib_per_sec(1024, f64::NAN), 0);
        assert_eq!(rate_kib_per_sec(1024, f64::INFINITY), 0);
    }

    #[test]
    fn test_network_baselines_update_in_place() {
        let mut baselines = NetworkRateBaselines::default();
        let (rx, tx) = baselines.update("en0", 1000, 2000);
        assert_eq!((rx, tx), (0, 0));
        assert!(baselines.by_interface.contains_key("en0"));

        if let Some(b) = baselines.by_interface.get_mut("en0") {
            b.at = std::time::Instant::now() - std::time::Duration::from_secs(1);
        }
        let (rx2, tx2) = baselines.update("en0", 1000 + 2048, 2000 + 4096);
        assert_eq!((rx2, tx2), (2, 4));
    }

    #[test]
    fn test_compute_rates_normal_delta() {
        assert_eq!(
            compute_rates(Some(2048), Some(4096), 4096, 6144, 1.0),
            (2, 2)
        );
    }
}
