extern crate starship_battery as battery;

use battery::units::time::second;
use std::io;

fn new_battery() -> battery::Result<(battery::Battery, battery::Manager)> {
    let manager = battery::Manager::new()?;
    let battery = match manager.batteries()?.next() {
        Some(Ok(battery)) => battery,
        Some(Err(e)) => {
            eprintln!("Unable to access battery information");
            return Err(e);
        }
        None => {
            eprintln!("Unable to find any batteries");
            return Err(io::Error::from(io::ErrorKind::NotFound).into());
        }
    };
    Ok((battery, manager))
}

pub fn get_battery_stats(flags: &[&str]) -> String {
    let mut result = String::new();

    match new_battery() {
        Ok((mut b, manager)) => {
            let _ = manager.refresh(&mut b);
            for &flag in flags {
                match flag {
                    "count" => {
                        if let Ok(battery_count) = manager.batteries() {
                            result
                                .push_str(&format!("BATTERY_COUNT=\"{}\" ", battery_count.count()));
                        }
                    }
                    "percentage" => {
                        result.push_str(&format!(
                            "BATTERY_PERCENTAGE=\"{:.2}%\" ",
                            b.state_of_charge().get::<battery::units::ratio::percent>()
                        ));
                    }
                    "state" => {
                        result.push_str(&format!("BATTERY_STATE=\"{}\" ", b.state()));
                    }
                    "time_to_empty" => {
                        if let Some(time) = b.time_to_empty() {
                            let time_in_minutes = time.get::<second>() / 60.0;
                            result.push_str(&format!(
                                "BATTERY_TIME_TO_EMPTY=\"{} mins\" ",
                                time_in_minutes
                            ));
                        } else {
                            result.push_str("BATTERY_TIME_TO_EMPTY=\"N/A\" ");
                        }
                    }
                    "time_to_full" => {
                        if let Some(time) = b.time_to_full() {
                            let time_in_minutes = time.get::<second>() / 60.0;
                            result.push_str(&format!(
                                "BATTERY_TIME_TO_FULL=\"{} mins\" ",
                                time_in_minutes
                            ));
                        } else {
                            result.push_str("BATTERY_TIME_TO_FULL=\"N/A\" ");
                        }
                    }
                    _ => {}
                }
            }
        }
        Err(_e) => {
            eprintln!("Unable to access battery information");
        }
    }
    result
}
