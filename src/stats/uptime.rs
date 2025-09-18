use sysinfo::System;

struct TimeUnit {
    name: &'static str,
    seconds: u64,
    suffix: &'static str,
}

const TIME_UNITS: &[TimeUnit] = &[
    TimeUnit {
        name: "week",
        seconds: 7 * 24 * 3600,
        suffix: "w",
    },
    TimeUnit {
        name: "day",
        seconds: 24 * 3600,
        suffix: "d",
    },
    TimeUnit {
        name: "hour",
        seconds: 3600,
        suffix: "h",
    },
    TimeUnit {
        name: "min",
        seconds: 60,
        suffix: "m",
    },
    TimeUnit {
        name: "sec",
        seconds: 1,
        suffix: "s",
    },
];

pub fn get_uptime_stats(flags: &[&str]) -> String {
    let mut uptime_secs = System::uptime();
    let mut result = Vec::new();

    let sorted_flags: Vec<&str> = if flags.is_empty() {
        TIME_UNITS.iter().map(|u| u.name).collect()
    } else {
        let mut flags_vec: Vec<&str> = flags
            .iter()
            .copied()
            .filter(|&flag| TIME_UNITS.iter().any(|u| u.name == flag))
            .collect();

        flags_vec.sort_by_key(|&flag| TIME_UNITS.iter().position(|u| u.name == flag).unwrap());
        flags_vec
    };

    for &flag in &sorted_flags {
        if let Some(unit) = TIME_UNITS.iter().find(|u| u.name == flag) {
            if uptime_secs >= unit.seconds {
                let value = uptime_secs / unit.seconds;
                uptime_secs %= unit.seconds;
                result.push(format!("{}{}", value, unit.suffix));
            }
        }
    }

    if result.is_empty() {
        let min_suffix = sorted_flags
            .last()
            .and_then(|flag| TIME_UNITS.iter().find(|u| u.name == *flag))
            .map(|unit| unit.suffix)
            .unwrap_or("s");
        format!("UPTIME=\"0{}\" ", min_suffix)
    } else {
        format!("UPTIME=\"{}\" ", result.join(" "))
    }
}
