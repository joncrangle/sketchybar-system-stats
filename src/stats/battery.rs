use starship_battery::{Manager, State};
use std::fmt::Write;

pub fn get_battery_stats(flags: &[&str], no_units: bool, buf: &mut String) {
    let manager = match Manager::new() {
        Ok(m) => m,
        Err(_) => return,
    };

    let batteries: Vec<_> = match manager.batteries() {
        Ok(batteries) => batteries.filter_map(Result::ok).collect(),
        Err(_) => return,
    };

    if batteries.is_empty() {
        return;
    }

    let battery = &batteries[0];

    for &flag in flags {
        match flag {
            "percentage" => {
                let percentage = (battery.state_of_charge().value * 100.0).round() as u32;
                let unit = if no_units { "" } else { "%" };
                let _ = write!(buf, "BATTERY_PERCENTAGE=\"{percentage}{unit}\" ");
            }
            "state" => {
                let state_str = match battery.state() {
                    State::Charging => "charging",
                    State::Discharging => "discharging",
                    State::Full => "full",
                    State::Empty => "empty",
                    _ => "unknown",
                };
                let _ = write!(buf, "BATTERY_STATE=\"{state_str}\" ");
            }
            "remaining" => {
                if let Some(time) = battery.time_to_empty() {
                    let mins = time.value as u32 / 60;
                    let unit = if no_units { "" } else { "min" };
                    let _ = write!(buf, "BATTERY_REMAINING=\"{mins}{unit}\" ");
                }
            }
            "time_to_full" => {
                if let Some(time) = battery.time_to_full() {
                    let mins = time.value as u32 / 60;
                    let unit = if no_units { "" } else { "min" };
                    let _ = write!(buf, "BATTERY_TIME_TO_FULL=\"{mins}{unit}\" ");
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_battery_stats_with_units() {
        let mut buf = String::new();
        get_battery_stats(&["percentage", "state"], false, &mut buf);

        if !buf.is_empty() {
            assert!(buf.contains("BATTERY_PERCENTAGE=") || buf.contains("BATTERY_STATE="));
        }
    }

    #[test]
    fn test_get_battery_stats_without_units() {
        let mut buf = String::new();
        get_battery_stats(&["percentage"], true, &mut buf);

        if buf.contains("BATTERY_PERCENTAGE=") {
            assert!(!buf.contains("%"));
        }
    }

    #[test]
    fn test_get_battery_stats_empty_flags() {
        let mut buf = String::new();
        get_battery_stats(&[], false, &mut buf);

        assert_eq!(buf, "");
    }

    #[test]
    fn test_get_battery_stats_handles_no_battery() {
        let mut buf = String::new();
        get_battery_stats(&["percentage"], false, &mut buf);
    }
}
