use std::fmt::Write;
use sysinfo::Gpus;

use super::unit;

/// Returns the highest usage reported across GPUs.
///
/// Macs with both an integrated and a discrete GPU report one of them idle, so
/// the busiest GPU is the meaningful figure rather than an average.
fn highest_usage(usages: impl IntoIterator<Item = Option<f32>>) -> Option<f32> {
    usages.into_iter().flatten().reduce(f32::max)
}

pub fn get_gpu_stats(gpus: &Gpus, flags: &[&str], no_units: bool, buf: &mut String) {
    for &flag in flags {
        if flag != "usage" {
            continue;
        }

        if let Some(usage) = highest_usage(gpus.iter().map(|gpu| gpu.usage())) {
            let unit = unit(no_units, "%");
            let _ = write!(buf, "GPU_USAGE=\"{:.0}{unit}\" ", usage.round());
        } else {
            let _ = write!(buf, "GPU_USAGE=\"N/A\" ");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highest_usage_picks_busiest_gpu() {
        assert_eq!(highest_usage([Some(3.0), Some(45.0)]), Some(45.0));
    }

    #[test]
    fn test_highest_usage_skips_unavailable_gpus() {
        assert_eq!(highest_usage([None, Some(12.0)]), Some(12.0));
    }

    #[test]
    fn test_highest_usage_none_when_unavailable() {
        assert_eq!(highest_usage([None]), None);
        assert_eq!(highest_usage([]), None);
    }

    #[test]
    fn test_get_gpu_stats_without_gpus_emits_na_without_unit() {
        let gpus = Gpus::new().unwrap();
        let mut buf = String::new();

        get_gpu_stats(&gpus, &["usage"], false, &mut buf);

        assert_eq!(buf, "GPU_USAGE=\"N/A\" ");
    }

    #[test]
    fn test_get_gpu_stats_unknown_flag_ignored() {
        let gpus = Gpus::new().unwrap();
        let mut buf = String::new();

        get_gpu_stats(&gpus, &["bogus"], false, &mut buf);

        assert!(buf.is_empty());
    }
}
