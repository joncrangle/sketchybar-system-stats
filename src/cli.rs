use anyhow::{bail, Result};
use clap::Parser;

// Default values as constants
pub const DEFAULT_INTERVAL: u32 = 5;
pub const DEFAULT_NETWORK_REFRESH_RATE: u32 = 5;
pub const MIN_INTERVAL: u32 = 1;
pub const MAX_INTERVAL: u32 = 3600; // 1 hour max
pub const MIN_NETWORK_REFRESH_RATE: u32 = 1;
pub const MAX_NETWORK_REFRESH_RATE: u32 = 100;

#[derive(Parser, Debug)]
#[command(name = "stats_provider", version, about, long_about = None, arg_required_else_help = true)]
pub struct Cli {
    #[arg(short = 'a', long, num_args = 0, help = "Get all stats")]
    pub all: bool,

    #[arg(short = 'b', long, num_args = 1.., value_parser = all_battery_flags(), help = "Get battery stats")]
    pub battery: Option<Vec<String>>,

    #[arg(short = 'c', long, num_args = 1.., value_parser = all_cpu_flags(), help = "Get CPU stats")]
    pub cpu: Option<Vec<String>>,

    #[arg(short = 'd', long, num_args = 1.., value_parser = all_disk_flags(), help = "Get disk stats")]
    pub disk: Option<Vec<String>>,

    #[arg(short = 'm', long, num_args = 1.., value_parser = all_memory_flags(), help = "Get memory stats")]
    pub memory: Option<Vec<String>>,

    #[arg(short = 'n', long, num_args = 1.., help = "Network rx/tx in KB/s. Specify network interfaces (e.g., -n eth0 en0 lo0). At least one is required.")]
    pub network: Option<Vec<String>>,

    #[arg(short = 's', long, num_args = 1.., value_parser = all_system_flags(), help = "Get system stats")]
    pub system: Option<Vec<String>>,

    #[arg(short = 'u', long, num_args = 1.., value_parser = all_uptime_flags(), help = "Get uptime stats")]
    pub uptime: Option<Vec<String>>,

    #[arg(
        short = 'i',
        long,
        default_value_t = DEFAULT_INTERVAL,
        help = "Refresh interval in seconds"
    )]
    pub interval: u32,

    #[arg(
        long,
        default_value_t = DEFAULT_NETWORK_REFRESH_RATE,
        help = "Network refresh rate (how often to refresh network interface list, in stat intervals)"
    )]
    pub network_refresh_rate: u32,

    #[arg(long, help = "Bar name (optional)")]
    pub bar: Option<String>,

    #[arg(long, default_value_t = false, help = "Enable verbose output")]
    pub verbose: bool,

    #[arg(long, default_value_t = false, help = "Output values without units")]
    pub no_units: bool,
}

pub fn parse_args() -> Cli {
    Cli::parse()
}

pub fn validate_cli(cli: &Cli) -> Result<()> {
    // Validate interval
    if cli.interval < MIN_INTERVAL || cli.interval > MAX_INTERVAL {
        bail!(
            "Interval must be between {} and {} seconds, got {}",
            MIN_INTERVAL,
            MAX_INTERVAL,
            cli.interval
        );
    }

    // Validate network refresh rate
    if cli.network_refresh_rate < MIN_NETWORK_REFRESH_RATE
        || cli.network_refresh_rate > MAX_NETWORK_REFRESH_RATE
    {
        bail!(
            "Network refresh rate must be between {} and {}, got {}",
            MIN_NETWORK_REFRESH_RATE,
            MAX_NETWORK_REFRESH_RATE,
            cli.network_refresh_rate
        );
    }

    // Validate that at least one stat type is requested if not using --all
    if !cli.all
        && cli.battery.is_none()
        && cli.cpu.is_none()
        && cli.disk.is_none()
        && cli.memory.is_none()
        && cli.network.is_none()
        && cli.system.is_none()
        && cli.uptime.is_none()
    {
        bail!("At least one stat type must be specified, or use --all");
    }

    Ok(())
}

pub fn all_battery_flags() -> Vec<&'static str> {
    vec!["percentage", "remaining", "state", "time_to_full"]
}

pub fn all_cpu_flags() -> Vec<&'static str> {
    vec!["count", "frequency", "temperature", "usage"]
}

pub fn all_disk_flags() -> Vec<&'static str> {
    vec!["count", "free", "total", "usage", "used"]
}

pub fn all_ram_flags() -> Vec<&'static str> {
    vec!["ram_available", "ram_total", "ram_usage", "ram_used"]
}

pub fn all_swp_flags() -> Vec<&'static str> {
    vec!["swp_free", "swp_total", "swp_usage", "swp_used"]
}

pub fn all_memory_flags() -> Vec<&'static str> {
    let mut flags = Vec::new();
    flags.extend(all_ram_flags());
    flags.extend(all_swp_flags());
    flags
}

pub fn all_system_flags() -> Vec<&'static str> {
    vec![
        "arch",
        "distro",
        "host_name",
        "kernel_version",
        "name",
        "os_version",
        "long_os_version",
    ]
}

pub fn all_uptime_flags() -> Vec<&'static str> {
    vec!["week", "day", "hour", "min", "sec"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_cli_with_all_flag() {
        let cli = Cli {
            all: true,
            battery: None,
            cpu: None,
            disk: None,
            memory: None,
            network: None,
            system: None,
            uptime: None,
            interval: DEFAULT_INTERVAL,
            network_refresh_rate: DEFAULT_NETWORK_REFRESH_RATE,
            bar: None,
            verbose: false,
            no_units: false,
        };
        assert!(validate_cli(&cli).is_ok());
    }

    #[test]
    fn test_validate_cli_with_cpu_flag() {
        let cli = Cli {
            all: false,
            battery: None,
            cpu: Some(vec!["usage".to_string()]),
            disk: None,
            memory: None,
            network: None,
            system: None,
            uptime: None,
            interval: DEFAULT_INTERVAL,
            network_refresh_rate: DEFAULT_NETWORK_REFRESH_RATE,
            bar: None,
            verbose: false,
            no_units: false,
        };
        assert!(validate_cli(&cli).is_ok());
    }

    #[test]
    fn test_validate_cli_no_flags() {
        let cli = Cli {
            all: false,
            battery: None,
            cpu: None,
            disk: None,
            memory: None,
            network: None,
            system: None,
            uptime: None,
            interval: DEFAULT_INTERVAL,
            network_refresh_rate: DEFAULT_NETWORK_REFRESH_RATE,
            bar: None,
            verbose: false,
            no_units: false,
        };
        assert!(validate_cli(&cli).is_err());
    }

    #[test]
    fn test_validate_cli_interval_too_low() {
        let cli = Cli {
            all: true,
            battery: None,
            cpu: None,
            disk: None,
            memory: None,
            network: None,
            system: None,
            uptime: None,
            interval: 0,
            network_refresh_rate: DEFAULT_NETWORK_REFRESH_RATE,
            bar: None,
            verbose: false,
            no_units: false,
        };
        assert!(validate_cli(&cli).is_err());
    }

    #[test]
    fn test_validate_cli_interval_too_high() {
        let cli = Cli {
            all: true,
            battery: None,
            cpu: None,
            disk: None,
            memory: None,
            network: None,
            system: None,
            uptime: None,
            interval: MAX_INTERVAL + 1,
            network_refresh_rate: DEFAULT_NETWORK_REFRESH_RATE,
            bar: None,
            verbose: false,
            no_units: false,
        };
        assert!(validate_cli(&cli).is_err());
    }

    #[test]
    fn test_validate_cli_network_refresh_rate_too_low() {
        let cli = Cli {
            all: true,
            battery: None,
            cpu: None,
            disk: None,
            memory: None,
            network: None,
            system: None,
            uptime: None,
            interval: DEFAULT_INTERVAL,
            network_refresh_rate: 0,
            bar: None,
            verbose: false,
            no_units: false,
        };
        assert!(validate_cli(&cli).is_err());
    }

    #[test]
    fn test_validate_cli_network_refresh_rate_too_high() {
        let cli = Cli {
            all: true,
            battery: None,
            cpu: None,
            disk: None,
            memory: None,
            network: None,
            system: None,
            uptime: None,
            interval: DEFAULT_INTERVAL,
            network_refresh_rate: MAX_NETWORK_REFRESH_RATE + 1,
            bar: None,
            verbose: false,
            no_units: false,
        };
        assert!(validate_cli(&cli).is_err());
    }

    #[test]
    fn test_all_cpu_flags_returns_correct_flags() {
        let flags = all_cpu_flags();
        assert_eq!(flags, vec!["count", "frequency", "temperature", "usage"]);
    }

    #[test]
    fn test_all_disk_flags_returns_correct_flags() {
        let flags = all_disk_flags();
        assert_eq!(flags, vec!["count", "free", "total", "usage", "used"]);
    }

    #[test]
    fn test_all_memory_flags_contains_ram_and_swap() {
        let flags = all_memory_flags();
        assert!(flags.contains(&"ram_available"));
        assert!(flags.contains(&"swp_total"));
        assert_eq!(flags.len(), 8);
    }

    #[test]
    fn test_all_system_flags_returns_correct_flags() {
        let flags = all_system_flags();
        assert!(flags.contains(&"arch"));
        assert!(flags.contains(&"distro"));
        assert_eq!(flags.len(), 7);
    }

    #[test]
    fn test_all_uptime_flags_returns_correct_flags() {
        let flags = all_uptime_flags();
        assert_eq!(flags, vec!["week", "day", "hour", "min", "sec"]);
    }
}
