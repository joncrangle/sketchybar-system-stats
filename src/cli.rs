use anyhow::{Result, bail};
use clap::Parser;

// Default values as constants
pub const DEFAULT_INTERVAL: u32 = 5;
pub const DEFAULT_NETWORK_REFRESH_RATE: u32 = 5;
pub const MIN_INTERVAL: u32 = 1;
pub const MAX_INTERVAL: u32 = 3600; // 1 hour max
pub const MIN_NETWORK_REFRESH_RATE: u32 = 1;
pub const MAX_NETWORK_REFRESH_RATE: u32 = 100;

pub const ALL_BATTERY_FLAGS: &[&str] = &["percentage", "remaining", "state", "time_to_full"];
pub const ALL_CPU_FLAGS: &[&str] = &["count", "frequency", "temperature", "usage"];
pub const ALL_DISK_FLAGS: &[&str] = &["count", "free", "total", "usage", "used"];
pub const ALL_RAM_FLAGS: &[&str] = &["ram_available", "ram_total", "ram_usage", "ram_used"];
pub const ALL_SWP_FLAGS: &[&str] = &["swp_free", "swp_total", "swp_usage", "swp_used"];
pub const ALL_MEMORY_FLAGS: &[&str] = &[
    "ram_available",
    "ram_total",
    "ram_usage",
    "ram_used",
    "swp_free",
    "swp_total",
    "swp_usage",
    "swp_used",
];
pub const ALL_SYSTEM_FLAGS: &[&str] = &[
    "arch",
    "distro",
    "host_name",
    "kernel_version",
    "name",
    "os_version",
    "long_os_version",
];
pub const ALL_UPTIME_FLAGS: &[&str] = &["week", "day", "hour", "min", "sec"];

#[derive(Parser, Debug)]
#[command(name = "stats_provider", version, about, long_about = None, arg_required_else_help = true)]
pub struct Cli {
    #[arg(short = 'a', long, help = "Get all stats")]
    pub all: bool,

    #[arg(short = 'b', long, num_args = 1.., value_parser = clap::builder::PossibleValuesParser::new(ALL_BATTERY_FLAGS), help = "Get battery stats")]
    pub battery: Option<Vec<String>>,

    #[arg(short = 'c', long, num_args = 1.., value_parser = clap::builder::PossibleValuesParser::new(ALL_CPU_FLAGS), help = "Get CPU stats")]
    pub cpu: Option<Vec<String>>,

    #[arg(short = 'd', long, num_args = 1.., value_parser = clap::builder::PossibleValuesParser::new(ALL_DISK_FLAGS), help = "Get disk stats")]
    pub disk: Option<Vec<String>>,

    #[arg(short = 'm', long, num_args = 1.., value_parser = clap::builder::PossibleValuesParser::new(ALL_MEMORY_FLAGS), help = "Get memory stats")]
    pub memory: Option<Vec<String>>,

    #[arg(short = 'n', long, num_args = 1.., help = "Network rx/tx in KiB/s. Specify network interfaces (e.g., -n eth0 en0 lo0). At least one is required.")]
    pub network: Option<Vec<String>>,

    #[arg(short = 's', long, num_args = 1.., value_parser = clap::builder::PossibleValuesParser::new(ALL_SYSTEM_FLAGS), help = "Get system stats")]
    pub system: Option<Vec<String>>,

    #[arg(short = 'u', long, num_args = 1.., value_parser = clap::builder::PossibleValuesParser::new(ALL_UPTIME_FLAGS), help = "Get uptime stats")]
    pub uptime: Option<Vec<String>>,

    #[arg(
        short = 'i',
        long,
        default_value_t = DEFAULT_INTERVAL,
        value_parser = clap::value_parser!(u32).range((MIN_INTERVAL as i64)..=(MAX_INTERVAL as i64)),
        help = "Refresh interval in seconds (1-3600)"
    )]
    pub interval: u32,

    #[arg(
        long,
        default_value_t = DEFAULT_NETWORK_REFRESH_RATE,
        value_parser = clap::value_parser!(u32).range((MIN_NETWORK_REFRESH_RATE as i64)..=(MAX_NETWORK_REFRESH_RATE as i64)),
        help = "Network refresh rate (how often to refresh the network interface list, in stat intervals) (1-100)"
    )]
    pub network_refresh_rate: u32,

    #[arg(long, help = "Bar name (optional)")]
    pub bar: Option<String>,

    #[arg(long, help = "Enable verbose output")]
    pub verbose: bool,

    #[arg(long, help = "Output values without units")]
    pub no_units: bool,
}

pub fn parse_args() -> Cli {
    Cli::parse()
}

pub fn validate_cli(cli: &Cli) -> Result<()> {
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
    fn test_interval_range_validation_via_clap() {
        let min = MIN_INTERVAL.to_string();
        let max = MAX_INTERVAL.to_string();
        let max_plus_one = (MAX_INTERVAL + 1).to_string();

        assert!(Cli::try_parse_from(["stats_provider", "-i", min.as_str()]).is_ok());
        assert!(Cli::try_parse_from(["stats_provider", "-i", max.as_str()]).is_ok());
        assert!(Cli::try_parse_from(["stats_provider", "-i", "0"]).is_err());
        assert!(Cli::try_parse_from(["stats_provider", "-i", max_plus_one.as_str()]).is_err());
    }

    #[test]
    fn test_network_refresh_rate_range_validation_via_clap() {
        let min = MIN_NETWORK_REFRESH_RATE.to_string();
        let max = MAX_NETWORK_REFRESH_RATE.to_string();
        let max_plus_one = (MAX_NETWORK_REFRESH_RATE + 1).to_string();

        assert!(
            Cli::try_parse_from(["stats_provider", "--network-refresh-rate", min.as_str()]).is_ok()
        );
        assert!(
            Cli::try_parse_from(["stats_provider", "--network-refresh-rate", max.as_str()]).is_ok()
        );
        assert!(Cli::try_parse_from(["stats_provider", "--network-refresh-rate", "0"]).is_err());
        assert!(
            Cli::try_parse_from([
                "stats_provider",
                "--network-refresh-rate",
                max_plus_one.as_str()
            ])
            .is_err()
        );
    }

    #[test]
    fn test_cli_parses_valid_bounds() {
        let interval = DEFAULT_INTERVAL.to_string();
        let rate = DEFAULT_NETWORK_REFRESH_RATE.to_string();

        assert!(
            Cli::try_parse_from([
                "stats_provider",
                "--all",
                "--interval",
                interval.as_str(),
                "--network-refresh-rate",
                rate.as_str(),
            ])
            .is_ok()
        );
    }

    #[test]
    fn test_all_battery_flags_const_has_expected_values() {
        assert_eq!(
            ALL_BATTERY_FLAGS,
            &["percentage", "remaining", "state", "time_to_full"]
        );
    }

    #[test]
    fn test_all_cpu_flags_const_has_expected_values() {
        assert_eq!(
            ALL_CPU_FLAGS,
            &["count", "frequency", "temperature", "usage"]
        );
    }

    #[test]
    fn test_all_disk_flags_const_has_expected_values() {
        assert_eq!(ALL_DISK_FLAGS, &["count", "free", "total", "usage", "used"]);
    }

    #[test]
    fn test_all_memory_flags_const_contains_ram_and_swap() {
        assert!(ALL_MEMORY_FLAGS.contains(&"ram_available"));
        assert!(ALL_MEMORY_FLAGS.contains(&"swp_total"));
        assert_eq!(ALL_MEMORY_FLAGS.len(), 8);
    }

    #[test]
    fn test_all_system_flags_const_has_expected_values() {
        assert!(ALL_SYSTEM_FLAGS.contains(&"arch"));
        assert!(ALL_SYSTEM_FLAGS.contains(&"distro"));
        assert_eq!(ALL_SYSTEM_FLAGS.len(), 7);
    }

    #[test]
    fn test_all_uptime_flags_const_has_expected_values() {
        assert_eq!(ALL_UPTIME_FLAGS, &["week", "day", "hour", "min", "sec"]);
    }

    #[test]
    fn test_cli_command_debug_assert() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
