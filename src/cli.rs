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
    fn test_every_all_flag_round_trips_through_cli() {
        type FlagCase = (
            &'static str,
            &'static [&'static str],
            fn(&Cli) -> Option<&[String]>,
        );
        let cases: [FlagCase; 6] = [
            ("battery", ALL_BATTERY_FLAGS, |c| c.battery.as_deref()),
            ("cpu", ALL_CPU_FLAGS, |c| c.cpu.as_deref()),
            ("disk", ALL_DISK_FLAGS, |c| c.disk.as_deref()),
            ("memory", ALL_MEMORY_FLAGS, |c| c.memory.as_deref()),
            ("system", ALL_SYSTEM_FLAGS, |c| c.system.as_deref()),
            ("uptime", ALL_UPTIME_FLAGS, |c| c.uptime.as_deref()),
        ];

        for (arg, flags, accessor) in cases {
            // Every flag in the const slice round-trips through the CLI on its own.
            for &flag in flags {
                let flag_arg = format!("--{arg}");
                let parsed = Cli::try_parse_from(["stats_provider", flag_arg.as_str(), flag])
                    .expect("expected a known flag value to parse");
                assert_eq!(accessor(&parsed), Some(&[flag.to_string()][..]));
            }

            // Passing every flag at once yields the full const slice.
            let flag_arg = format!("--{arg}");
            let mut all_args = vec!["stats_provider", flag_arg.as_str()];
            all_args.extend(flags.iter().copied());
            let parsed = Cli::try_parse_from(all_args).expect("expected all flags to parse");
            let expected: Vec<String> = flags.iter().map(|s| s.to_string()).collect();
            assert_eq!(accessor(&parsed), Some(expected.as_slice()));
        }
    }

    #[test]
    fn test_unknown_flag_values_are_rejected() {
        for arg in ["battery", "cpu", "disk", "memory", "system", "uptime"] {
            let flag_arg = format!("--{arg}");
            let result = Cli::try_parse_from(["stats_provider", flag_arg.as_str(), "bogus"]);
            assert!(result.is_err(), "--{arg} should reject an unknown value");
        }
    }

    #[test]
    fn test_interval_and_network_refresh_rate_defaults() {
        let cli = Cli::try_parse_from(["stats_provider", "--all"]).unwrap();

        assert_eq!(cli.interval, DEFAULT_INTERVAL);
        assert_eq!(cli.network_refresh_rate, DEFAULT_NETWORK_REFRESH_RATE);
        assert!((MIN_INTERVAL..=MAX_INTERVAL).contains(&cli.interval));
        assert!(cli.network_refresh_rate >= MIN_NETWORK_REFRESH_RATE);
        assert!(cli.network_refresh_rate <= MAX_NETWORK_REFRESH_RATE);
    }

    #[test]
    fn test_cli_command_debug_assert() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
