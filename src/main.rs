mod cli;
mod sketchybar;
mod stats;

use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fs2::FileExt;
use sketchybar::Sketchybar;
use stats::{
    NetworkRateBaselines, get_battery_stats, get_cpu_stats, get_disk_stats, get_gpu_stats,
    get_load_stats, get_memory_stats, get_network_stats, get_system_stats, get_uptime_stats,
};
use sysinfo::{Components, Disks, Gpus, Networks, System};

struct ProcessedFlags<'a> {
    battery_flags: Option<Vec<&'a str>>,
    cpu_flags: Option<Vec<&'a str>>,
    disk_flags: Option<Vec<&'a str>>,
    gpu_flags: Option<Vec<&'a str>>,
    load_flags: Option<Vec<&'a str>>,
    memory_flags: Option<Vec<&'a str>>,
    network_flags: Option<&'a [String]>,
    system_flags: Option<Vec<&'a str>>,
    uptime_flags: Option<Vec<&'a str>>,
}

struct StatsContext<'a> {
    system: &'a mut System,
    disks: &'a mut Disks,
    gpus: &'a mut Gpus,
    networks: &'a mut Networks,
    components: &'a mut Components,
    network_baselines: NetworkRateBaselines,
}

struct StatsConfig<'a> {
    flags: ProcessedFlags<'a>,
    refresh_kind: sysinfo::RefreshKind,
    system_stats: String,
}

fn to_str_refs(flags: Option<&[String]>) -> Option<Vec<&str>> {
    flags.map(|v| v.iter().map(String::as_str).collect())
}

fn process_cli_flags(cli: &cli::Cli) -> ProcessedFlags<'_> {
    ProcessedFlags {
        battery_flags: to_str_refs(cli.battery.as_deref()),
        cpu_flags: to_str_refs(cli.cpu.as_deref()),
        disk_flags: to_str_refs(cli.disk.as_deref()),
        gpu_flags: to_str_refs(cli.gpu.as_deref()),
        load_flags: to_str_refs(cli.load.as_deref()),
        memory_flags: to_str_refs(cli.memory.as_deref()),
        network_flags: cli.network.as_deref(),
        system_flags: to_str_refs(cli.system.as_deref()),
        uptime_flags: to_str_refs(cli.uptime.as_deref()),
    }
}

fn validate_network_interfaces(
    networks: &Networks,
    requested_interfaces: &[String],
    verbose: bool,
) -> Result<()> {
    if networks.is_empty() {
        anyhow::bail!("No network interfaces available on this system");
    }

    for interface in requested_interfaces {
        if !networks.contains_key(OsStr::new(interface)) {
            let available_interfaces: Vec<String> = networks
                .keys()
                .map(|name| name.to_string_lossy().into_owned())
                .collect();
            let msg = format!(
                "Network interface '{}' not found. Available interfaces: {}",
                interface,
                available_interfaces.join(", ")
            );
            if verbose {
                eprintln!("Warning: {}", msg);
            }
            anyhow::bail!("{}", msg);
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
async fn get_stats(cli: &cli::Cli, sketchybar: &Sketchybar) -> Result<()> {
    let flags = process_cli_flags(cli);
    let refresh_kind = stats::build_refresh_kind(
        select_flags(cli.all, cli::ALL_CPU_FLAGS, flags.cpu_flags.as_deref()),
        select_flags(
            cli.all,
            cli::ALL_MEMORY_FLAGS,
            flags.memory_flags.as_deref(),
        ),
    );
    let mut system =
        System::new_with_specifics(refresh_kind).context("Failed to initialize system stats")?;
    // These start empty: each tick refreshes, and so populates, only the ones the
    // selected stats read.
    let mut disks = Disks::new().context("Failed to initialize disk stats")?;
    let mut gpus = Gpus::new().context("Failed to initialize GPU stats")?;
    let mut networks = Networks::new().context("Failed to initialize network stats")?;
    let mut components = Components::new().context("Failed to initialize component stats")?;

    if let Some(network_flags) = &cli.network {
        networks.refresh(false);
        validate_network_interfaces(&networks, network_flags, cli.verbose)?;
    }

    let mut message_buffer = String::with_capacity(512);
    let system_stats = cache_system_stats(cli, &flags);

    let config = StatsConfig {
        flags,
        refresh_kind,
        system_stats,
    };

    let mut context = StatsContext {
        system: &mut system,
        disks: &mut disks,
        gpus: &mut gpus,
        networks: &mut networks,
        components: &mut components,
        network_baselines: NetworkRateBaselines::default(),
    };

    run_stats_loop(cli, sketchybar, &config, &mut context, &mut message_buffer).await
}

#[cfg(target_os = "macos")]
async fn run_stats_loop(
    cli: &cli::Cli,
    sketchybar: &Sketchybar,
    config: &StatsConfig<'_>,
    context: &mut StatsContext<'_>,
    message_buffer: &mut String,
) -> Result<()> {
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .context("Failed to register SIGTERM handler")?;
    let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(cli.interval.into()));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            result = async {
                ticker.tick().await;
                collect_and_send_stats(cli, sketchybar, config, context, message_buffer).await
            } => {
                if let Err(e) = result {
                    eprintln!("Warning: failed to collect/send stats: {e:#}");
                }
            }
            _ = tokio::signal::ctrl_c() => {
                if cli.verbose {
                    println!("Received shutdown signal (SIGINT), cleaning up...");
                }
                println!("SketchyBar Stats Provider is shutting down.");
                return Ok(());
            }
            _ = sigterm.recv() => {
                if cli.verbose {
                    println!("Received shutdown signal (SIGTERM), cleaning up...");
                }
                println!("SketchyBar Stats Provider is shutting down.");
                return Ok(());
            }
        }
    }
}

async fn collect_and_send_stats(
    cli: &cli::Cli,
    sketchybar: &Sketchybar,
    config: &StatsConfig<'_>,
    context: &mut StatsContext<'_>,
    buf: &mut String,
) -> Result<()> {
    collect_stats_commands(cli, config, context, buf);

    if cli.verbose {
        println!("Current message: {}", buf);
    }
    sketchybar
        .send_message("trigger", "system_stats", Some(buf), cli.verbose)
        .await
        .context("Failed to send stats to SketchyBar")?;

    Ok(())
}

fn select_flags<'a>(
    all: bool,
    all_flags: &'a [&'a str],
    configured_flags: Option<&'a [&'a str]>,
) -> Option<&'a [&'a str]> {
    if all {
        Some(all_flags)
    } else {
        configured_flags
    }
}

fn cache_system_stats(cli: &cli::Cli, flags: &ProcessedFlags<'_>) -> String {
    let mut cached = String::with_capacity(256);
    if let Some(system_flags) = select_flags(
        cli.all,
        cli::ALL_SYSTEM_FLAGS,
        flags.system_flags.as_deref(),
    ) {
        get_system_stats(system_flags, &mut cached);
    }
    cached
}

fn collect_stats_commands(
    cli: &cli::Cli,
    config: &StatsConfig<'_>,
    context: &mut StatsContext<'_>,
    buf: &mut String,
) {
    buf.clear();

    let battery_flags = select_flags(
        cli.all,
        cli::ALL_BATTERY_FLAGS,
        config.flags.battery_flags.as_deref(),
    );
    let cpu_flags = select_flags(
        cli.all,
        cli::ALL_CPU_FLAGS,
        config.flags.cpu_flags.as_deref(),
    );
    let disk_flags = select_flags(
        cli.all,
        cli::ALL_DISK_FLAGS,
        config.flags.disk_flags.as_deref(),
    );
    let gpu_flags = select_flags(
        cli.all,
        cli::ALL_GPU_FLAGS,
        config.flags.gpu_flags.as_deref(),
    );
    let load_flags = select_flags(
        cli.all,
        cli::ALL_LOAD_FLAGS,
        config.flags.load_flags.as_deref(),
    );
    let memory_flags = select_flags(
        cli.all,
        cli::ALL_MEMORY_FLAGS,
        config.flags.memory_flags.as_deref(),
    );
    let network_interfaces: Option<&[String]> = if cli.all {
        None
    } else {
        config.flags.network_flags
    };
    let collect_network = cli.all || network_interfaces.is_some();
    let uptime_flags = select_flags(
        cli.all,
        cli::ALL_UPTIME_FLAGS,
        config.flags.uptime_flags.as_deref(),
    );

    // Refresh only the sources the selected stats read. `refresh(true)` also picks
    // up new interfaces, disks and GPUs and drops the ones that are gone.
    context.system.refresh_specifics(config.refresh_kind);
    if disk_flags.is_some() {
        context.disks.refresh(true);
    }
    if gpu_flags.is_some() {
        context.gpus.refresh(true);
    }
    if cpu_flags.is_some_and(|flags| flags.contains(&"temperature")) {
        context.components.refresh(false);
    }
    if collect_network {
        context.networks.refresh(true);
    }

    if let Some(battery_flags) = battery_flags {
        get_battery_stats(battery_flags, cli.no_units, buf);
    }

    if let Some(cpu_flags) = cpu_flags {
        get_cpu_stats(
            context.system,
            context.components,
            cpu_flags,
            cli.no_units,
            buf,
        );
    }

    if let Some(disk_flags) = disk_flags {
        get_disk_stats(context.disks, disk_flags, cli.no_units, buf);
    }

    if let Some(gpu_flags) = gpu_flags {
        get_gpu_stats(context.gpus, gpu_flags, cli.no_units, buf);
    }

    if let Some(load_flags) = load_flags {
        get_load_stats(load_flags, buf);
    }

    if let Some(memory_flags) = memory_flags {
        get_memory_stats(context.system, memory_flags, cli.no_units, buf);
    }

    if collect_network {
        get_network_stats(
            context.networks,
            network_interfaces,
            &mut context.network_baselines,
            cli.no_units,
            buf,
        );
    }

    if let Some(uptime_flags) = uptime_flags {
        get_uptime_stats(uptime_flags, cli.no_units, buf);
    }

    buf.push_str(&config.system_stats);

    if buf.ends_with(' ') {
        buf.pop();
    }
}

fn stable_bar_hash(bar: &str) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    bar.as_bytes().iter().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

fn lock_file_path(bar: Option<&str>) -> PathBuf {
    let filename = match bar {
        Some("sketchybar") | None => "stats_provider.lock".to_string(),
        Some(name) => format!("stats_provider_{:016x}.lock", stable_bar_hash(name)),
    };
    std::env::temp_dir().join(filename)
}

fn acquire_lock_at(path: &Path) -> Result<Option<File>> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .with_context(|| format!("Failed to open lock file {}", path.display()))?;

    match file.try_lock_exclusive() {
        Ok(()) => Ok(Some(file)),
        Err(error) if error.raw_os_error() == fs2::lock_contended_error().raw_os_error() => {
            Ok(None)
        }
        Err(error) => {
            Err(error).with_context(|| format!("Failed to lock lock file {}", path.display()))
        }
    }
}

fn acquire_lock(bar: Option<&str>) -> Result<Option<File>> {
    let path = lock_file_path(bar);
    acquire_lock_at(&path)
}

#[cfg(target_os = "macos")]
#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::parse_args();
    cli::validate_cli(&cli).context("Invalid CLI arguments")?;

    let _lock = match acquire_lock(cli.bar.as_deref())? {
        Some(lock) => lock,
        None => {
            eprintln!("another stats_provider instance is already running; exiting");
            return Ok(());
        }
    };

    println!("SketchyBar Stats Provider is running.");

    if cli.verbose {
        println!("Stats Provider CLI: {cli:?}");
    }
    let sketchybar =
        Sketchybar::new(cli.bar.as_deref()).context("Failed to create Sketchybar instance")?;

    get_stats(&cli, &sketchybar).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn test_lock_file_path_distinct_for_different_bars() {
        let default_lock = lock_file_path(None);
        let named_default_lock = lock_file_path(Some("sketchybar"));
        let bar1_lock = lock_file_path(Some("bar1"));
        let bar2_lock = lock_file_path(Some("bar2"));

        assert_eq!(default_lock, named_default_lock);
        assert_ne!(default_lock, bar1_lock);
        assert_ne!(bar1_lock, bar2_lock);
        assert_eq!(bar1_lock, lock_file_path(Some("bar1")));
    }

    #[test]
    fn test_lock_file_path_keeps_untrusted_bar_name_inside_temp_dir() {
        let lock_path = lock_file_path(Some("../../outside/with/slashes"));

        assert_eq!(lock_path.parent(), Some(std::env::temp_dir().as_path()));
        assert_eq!(
            lock_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap(),
            format!(
                "stats_provider_{:016x}.lock",
                stable_bar_hash("../../outside/with/slashes")
            )
        );
    }

    #[test]
    fn test_acquire_lock_prevents_second_instance() {
        let test_path = std::env::temp_dir().join(format!(
            "test_stats_provider_lock_{}_{}.lock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let first = acquire_lock_at(&test_path).unwrap();
        let second = acquire_lock_at(&test_path).unwrap();
        assert!(first.is_some(), "first acquire on clean path must succeed");
        assert!(second.is_none(), "second acquire on locked path must fail");

        drop(first);
        let third = acquire_lock_at(&test_path).unwrap();
        assert!(third.is_some(), "acquire after drop must succeed");
        drop(third);
        let _ = std::fs::remove_file(test_path);
    }

    #[test]
    fn test_process_cli_flags() {
        let cli = cli::Cli {
            all: false,
            battery: None,
            cpu: Some(vec!["usage".to_string()]),
            disk: None,
            gpu: None,
            load: None,
            memory: Some(vec!["ram_total".to_string()]),
            network: None,
            system: None,
            uptime: None,
            interval: 5,
            bar: None,
            verbose: false,
            no_units: false,
        };

        let flags = process_cli_flags(&cli);

        assert!(flags.cpu_flags.is_some());
        assert!(flags.disk_flags.is_none());
        assert!(flags.memory_flags.is_some());
        assert!(flags.network_flags.is_none());
    }

    #[test]
    fn test_processed_flags_cpu_flag_refs() {
        let cpu_flags = vec!["usage".to_string(), "count".to_string()];
        let cli = cli::Cli {
            all: false,
            battery: None,
            cpu: Some(cpu_flags),
            disk: None,
            gpu: None,
            load: None,
            memory: None,
            network: None,
            system: None,
            uptime: None,
            interval: 5,
            bar: None,
            verbose: false,
            no_units: false,
        };

        let flags = process_cli_flags(&cli);
        assert!(flags.cpu_flags.is_some());
        let refs = flags.cpu_flags.as_deref().unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0], "usage");
        assert_eq!(refs[1], "count");
    }

    #[test]
    fn test_processed_flags_returns_none_when_empty() {
        let flags = ProcessedFlags {
            battery_flags: None,
            cpu_flags: None,
            disk_flags: None,
            gpu_flags: None,
            load_flags: None,
            memory_flags: None,
            network_flags: None,
            system_flags: None,
            uptime_flags: None,
        };

        assert!(flags.cpu_flags.is_none());
        assert!(flags.disk_flags.is_none());
        assert!(flags.memory_flags.is_none());
        assert!(flags.uptime_flags.is_none());
    }

    #[test]
    fn test_cache_system_stats_all_overrides_configured_subset() {
        let cli = cli::Cli::try_parse_from(["stats_provider", "--all", "--system", "arch"])
            .expect("--all and a system subset should parse together");
        let flags = process_cli_flags(&cli);

        let cached = cache_system_stats(&cli, &flags);

        for key in [
            "ARCH=\"",
            "DISTRO=\"",
            "HOST_NAME=\"",
            "KERNEL_VERSION=\"",
            "SYSTEM_NAME=\"",
            "OS_VERSION=\"",
            "LONG_OS_VERSION=\"",
        ] {
            assert!(cached.contains(key), "missing selected system key {key}");
        }
    }

    #[test]
    fn test_cache_system_stats_keeps_configured_subset_without_all() {
        let cli = cli::Cli::try_parse_from(["stats_provider", "--system", "arch"])
            .expect("system subset should parse");
        let flags = process_cli_flags(&cli);

        let cached = cache_system_stats(&cli, &flags);

        assert!(cached.contains("ARCH=\""));
        assert!(!cached.contains("DISTRO=\""));
        assert!(!cached.contains("HOST_NAME=\""));
    }

    #[test]
    fn test_collect_stats_commands_system_only_replays_cached_metadata() {
        let cli = cli::Cli::try_parse_from(["stats_provider", "--system", "host_name"])
            .expect("system-only selection should parse");
        let config = StatsConfig {
            flags: process_cli_flags(&cli),
            refresh_kind: sysinfo::RefreshKind::nothing(),
            system_stats: "HOST_NAME=\"cached host\" ".to_string(),
        };
        let mut system = System::new().unwrap();
        let mut disks = Disks::new().unwrap();
        let mut gpus = Gpus::new().unwrap();
        let mut networks = Networks::new().unwrap();
        let mut components = Components::new().unwrap();
        let mut context = StatsContext {
            system: &mut system,
            disks: &mut disks,
            gpus: &mut gpus,
            networks: &mut networks,
            components: &mut components,
            network_baselines: NetworkRateBaselines::default(),
        };
        let mut buf = String::new();

        collect_stats_commands(&cli, &config, &mut context, &mut buf);
        assert_eq!(buf, "HOST_NAME=\"cached host\"");
        collect_stats_commands(&cli, &config, &mut context, &mut buf);
        assert_eq!(buf, "HOST_NAME=\"cached host\"");
    }

    #[test]
    fn test_collect_stats_commands_skips_unselected_refreshes() {
        let cli = cli::Cli::try_parse_from(["stats_provider", "--cpu", "count"])
            .expect("CPU count selection should parse");
        let flags = process_cli_flags(&cli);
        let refresh_kind = stats::build_refresh_kind(flags.cpu_flags.as_deref(), None);
        let config = StatsConfig {
            flags,
            refresh_kind,
            system_stats: String::new(),
        };
        let mut system = System::new_with_specifics(refresh_kind).unwrap();
        let mut disks = Disks::new().unwrap();
        let mut gpus = Gpus::new().unwrap();
        let mut networks = Networks::new().unwrap();
        let mut components = Components::new().unwrap();
        let mut context = StatsContext {
            system: &mut system,
            disks: &mut disks,
            gpus: &mut gpus,
            networks: &mut networks,
            components: &mut components,
            network_baselines: NetworkRateBaselines::default(),
        };
        let mut buf = String::new();

        collect_stats_commands(&cli, &config, &mut context, &mut buf);

        assert!(buf.starts_with("CPU_COUNT=\""), "{buf}");
        assert!(context.disks.is_empty());
        assert!(context.gpus.is_empty());
        assert!(context.networks.is_empty());
        assert!(context.components.is_empty());
    }

    #[test]
    fn test_validate_network_interfaces_rejects_unknown() {
        let networks = Networks::new_with_refreshed_list().unwrap();
        let requested = vec!["definitely-not-an-interface-xyz".to_string()];

        let result = validate_network_interfaces(&networks, &requested, false);

        assert!(result.is_err());
    }

    #[test]
    fn test_collect_stats_commands_all_dispatch_emits_every_flag() {
        let cli = cli::Cli {
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
            interval: 5,
            bar: None,
            verbose: false,
            no_units: false,
        };
        let flags = process_cli_flags(&cli);
        let refresh_kind =
            stats::build_refresh_kind(Some(cli::ALL_CPU_FLAGS), Some(cli::ALL_MEMORY_FLAGS));
        let config = StatsConfig {
            flags,
            refresh_kind,
            system_stats: "TEST_CACHED_METADATA=\"stable\" ".to_string(),
        };
        // Start empty like `get_stats`, so the first tick must populate these.
        let mut system = System::new_with_specifics(refresh_kind).unwrap();
        let mut disks = Disks::new().unwrap();
        let mut gpus = Gpus::new().unwrap();
        let mut networks = Networks::new().unwrap();
        let mut components = Components::new().unwrap();
        let mut context = StatsContext {
            system: &mut system,
            disks: &mut disks,
            gpus: &mut gpus,
            networks: &mut networks,
            components: &mut components,
            network_baselines: NetworkRateBaselines::default(),
        };
        let mut buf = String::new();

        collect_stats_commands(&cli, &config, &mut context, &mut buf);
        assert!(buf.contains("TEST_CACHED_METADATA=\"stable\""));

        for key in ["CPU_COUNT=", "CPU_FREQUENCY=", "CPU_TEMP=", "CPU_USAGE="] {
            assert!(buf.contains(key), "missing CPU key {key} in: {buf}");
        }
        for key in [
            "DISK_COUNT=",
            "DISK_FREE=",
            "DISK_TOTAL=",
            "DISK_USED=",
            "DISK_USAGE=",
        ] {
            assert!(buf.contains(key), "missing disk key {key} in: {buf}");
        }
        for key in [
            "RAM_AVAILABLE=",
            "RAM_TOTAL=",
            "RAM_USED=",
            "RAM_USAGE=",
            "SWP_FREE=",
            "SWP_TOTAL=",
            "SWP_USED=",
            "SWP_USAGE=",
        ] {
            assert!(buf.contains(key), "missing memory key {key} in: {buf}");
        }
        assert!(buf.contains("GPU_USAGE=\""), "missing GPU usage in: {buf}");
        for key in ["LOAD_AVG_1=\"", "LOAD_AVG_5=\"", "LOAD_AVG_15=\""] {
            assert!(buf.contains(key), "missing load key {key} in: {buf}");
        }
        assert!(buf.contains("UPTIME=\""), "missing uptime in: {buf}");
        assert!(buf.contains("NETWORK_RX_"), "missing network rx in: {buf}");
        assert!(buf.contains("NETWORK_TX_"), "missing network tx in: {buf}");

        // Battery is hardware-dependent: percentage and state are always
        // emitted together when a battery exists, otherwise the machine is
        // battery-less and no battery keys appear.
        if buf.contains("BATTERY_PERCENTAGE=") {
            assert!(
                buf.contains("BATTERY_STATE="),
                "battery state missing in: {buf}"
            );
        }

        assert!(
            !context.disks.is_empty(),
            "disks should be listed on the first tick"
        );
        assert!(
            !context.networks.is_empty(),
            "networks should be listed on the first tick"
        );

        collect_stats_commands(&cli, &config, &mut context, &mut buf);
        assert_eq!(buf.matches("TEST_CACHED_METADATA=\"stable\"").count(), 1);
    }
}
