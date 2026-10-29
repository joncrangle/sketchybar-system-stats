use anyhow::{Result, bail};
use clap::Parser;

// Default values as constants
pub const DEFAULT_INTERVAL: u32 = 5;
pub const MIN_INTERVAL: u32 = 1;
pub const MAX_INTERVAL: u32 = 3600; // 1 hour max

pub const ALL_BATTERY_FLAGS: &[&str] = &["percentage", "remaining", "state", "time_to_full"];
pub const ALL_CPU_FLAGS: &[&str] = &["count", "frequency", "temperature", "usage"];
pub const ALL_DISK_FLAGS: &[&str] = &["count", "free", "total", "usage", "used"];
pub const ALL_GPU_FLAGS: &[&str] = &["usage"];
pub const ALL_LOAD_FLAGS: &[&str] = &["one", "five", "fifteen"];
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

    #[arg(short = 'g', long, num_args = 1.., value_parser = clap::builder::PossibleValuesParser::new(ALL_GPU_FLAGS), help = "Get GPU stats")]
    pub gpu: Option<Vec<String>>,

    #[arg(short = 'l', long, num_args = 0.., value_parser = clap::builder::PossibleValuesParser::new(ALL_LOAD_FLAGS), help = "Get load averages over 1, 5 and 15 minutes")]
    pub load: Option<Vec<String>>,

    #[arg(short = 'm', long, num_args = 1.., value_parser = clap::builder::PossibleValuesParser::new(ALL_MEMORY_FLAGS), help = "Get memory stats")]
    pub memory: Option<Vec<String>>,

    #[arg(short = 'n', long, num_args = 1.., help = "Network rx/tx in KiB/s. Specify network interfaces (e.g., -n eth0 en0 lo0). At least one is required.")]
    pub network: Option<Vec<String>>,

    #[arg(short = 's', long, num_args = 1.., value_parser = clap::builder::PossibleValuesParser::new(ALL_SYSTEM_FLAGS), help = "Get system stats")]
    pub system: Option<Vec<String>>,

    #[arg(short = 'u', long, num_args = 0.., value_parser = clap::builder::PossibleValuesParser::new(ALL_UPTIME_FLAGS), help = "Get uptime stats")]
    pub uptime: Option<Vec<String>>,

    #[arg(
        short = 'i',
        long,
        default_value_t = DEFAULT_INTERVAL,
        value_parser = clap::value_parser!(u32).range((MIN_INTERVAL as i64)..=(MAX_INTERVAL as i64)),
        help = "Refresh interval in seconds (1-3600)"
    )]
    pub interval: u32,

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
        && cli.gpu.is_none()
        && cli.load.is_none()
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
            gpu: None,
            load: None,
            memory: None,
            network: None,
            system: None,
            uptime: None,
            interval: DEFAULT_INTERVAL,
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
            gpu: None,
            load: None,
            memory: None,
            network: None,
            system: None,
            uptime: None,
            interval: DEFAULT_INTERVAL,
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
            gpu: None,
            load: None,
            memory: None,
            network: None,
            system: None,
            uptime: None,
            interval: DEFAULT_INTERVAL,
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
    fn test_every_all_flag_round_trips_through_cli() {
        type FlagCase = (
            &'static str,
            &'static [&'static str],
            fn(&Cli) -> Option<&[String]>,
        );
        let cases: [FlagCase; 8] = [
            ("battery", ALL_BATTERY_FLAGS, |c| c.battery.as_deref()),
            ("cpu", ALL_CPU_FLAGS, |c| c.cpu.as_deref()),
            ("disk", ALL_DISK_FLAGS, |c| c.disk.as_deref()),
            ("gpu", ALL_GPU_FLAGS, |c| c.gpu.as_deref()),
            ("load", ALL_LOAD_FLAGS, |c| c.load.as_deref()),
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
        for arg in [
            "battery", "cpu", "disk", "gpu", "load", "memory", "system", "uptime",
        ] {
            let flag_arg = format!("--{arg}");
            let result = Cli::try_parse_from(["stats_provider", flag_arg.as_str(), "bogus"]);
            assert!(result.is_err(), "--{arg} should reject an unknown value");
        }
    }

    #[test]
    fn test_uptime_flag_without_values_parses_as_empty_selection() {
        let parsed = Cli::try_parse_from(["stats_provider", "--uptime", "--no-units"])
            .expect("bare --uptime should parse before the next option");

        assert_eq!(parsed.uptime, Some(Vec::new()));
        assert!(parsed.no_units);
    }

    #[test]
    fn test_load_flag_without_values_is_a_valid_selection() {
        let parsed = Cli::try_parse_from(["stats_provider", "--load"])
            .expect("bare --load should parse as an empty selection");

        assert_eq!(parsed.load, Some(Vec::new()));
        assert!(validate_cli(&parsed).is_ok());
    }

    #[test]
    fn test_interval_default() {
        let cli = Cli::try_parse_from(["stats_provider", "--all"]).unwrap();

        assert_eq!(cli.interval, DEFAULT_INTERVAL);
        assert!((MIN_INTERVAL..=MAX_INTERVAL).contains(&cli.interval));
    }

    #[test]
    fn test_cli_command_debug_assert() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
