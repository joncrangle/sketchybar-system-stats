/// Bytes in one gibibyte (1024^3), used to convert byte counts to gigabytes.
pub const BYTES_PER_GB: f32 = 1_073_741_824.0;

/// Bytes in one kibibyte (1024), used to convert byte counts to kibibytes.
pub const BYTES_PER_KB: u64 = 1024;

/// Factor used to scale a ratio in 0 to 1 into a percentage.
pub const PERCENT: f32 = 100.0;

/// Seconds in one minute, used for battery time conversions.
pub const SECONDS_PER_MINUTE: u64 = 60;

/// Temperature sentinel returned when no CPU temperature component is found.
pub const NO_TEMP_SENTINEL: f32 = -1.0;

/// Returns the unit string, or an empty string when units are disabled.
pub fn unit(no_units: bool, unit: &'static str) -> &'static str {
    if no_units { "" } else { unit }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_with_units() {
        assert_eq!(unit(false, "GB"), "GB");
        assert_eq!(unit(false, "%"), "%");
        assert_eq!(unit(false, "KiB/s"), "KiB/s");
    }

    #[test]
    fn test_unit_without_units() {
        assert_eq!(unit(true, "GB"), "");
        assert_eq!(unit(true, "%"), "");
        assert_eq!(unit(true, "KiB/s"), "");
    }
}
