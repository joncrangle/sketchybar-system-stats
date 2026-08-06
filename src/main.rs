mod cli;
mod sketchybar;
mod stats;

use std::fs::File;

use anyhow::{Context, Result};
use fs2::FileExt;
use sketchybar::Sketchybar;
use stats::{
    NetworkRateBaselines, get_battery_stats, get_cpu_stats, get_disk_stats, get_memory_stats,
    get_network_stats, get_system_stats, get_uptime_stats,
};
use sysinfo::{Components, Disks, Networks, System};

struct ProcessedFlags<'a> {
    battery_flags: Option<&'a [String]>,
    cpu_flags: Option<&'a [String]>,
    disk_flags: Option<&'a [String]>,
    memory_flags: Option<&'a [String]>,
    network_flags: Option<&'a [String]>,
    uptime_flags: Option<&'a [String]>,
}

macro_rules! flag_refs_method {
    ($method_name:ident, $field:ident) => {
        fn $method_name(&self) -> Option<Vec<&str>> {
            self.$field
                .map(|flags| flags.iter().map(String::as_str).collect())
        }
    };
}

impl<'a> ProcessedFlags<'a> {
    flag_refs_method!(battery_flag_refs, battery_flags);
    flag_refs_method!(cpu_flag_refs, cpu_flags);
    flag_refs_method!(disk_flag_refs, disk_flags);
    flag_refs_method!(memory_flag_refs, memory_flags);
    flag_refs_method!(uptime_flag_refs, uptime_flags);
}

struct StatsContext<'a> {
    system: &'a mut System,
    disks: &'a mut Disks,
    networks: &'a mut Networks,
    components: &'a mut Components,
    network_baselines: NetworkRateBaselines,
}

struct StatsConfig<'a> {
    flags: ProcessedFlags<'a>,
    refresh_kind: sysinfo::RefreshKind,
}

fn process_cli_flags(cli: &cli::Cli) -> ProcessedFlags<'_> {
    ProcessedFlags {
        battery_flags: cli.battery.as_deref(),
        cpu_flags: cli.cpu.as_deref(),
        disk_flags: cli.disk.as_deref(),
        memory_flags: cli.memory.as_deref(),
        network_flags: cli.network.as_deref(),
        uptime_flags: cli.uptime.as_deref(),
    }
}

fn validate_network_interfaces(
    networks: &Networks,
    requested_interfaces: &[String],
    verbose: bool,
) -> Result<()> {
    let available_interfaces: Vec<String> = networks.keys().map(|name| name.to_string()).collect();

    if available_interfaces.is_empty() {
        anyhow::bail!("No network interfaces available on this system");
    }

    for interface in requested_interfaces {
        if !available_interfaces.contains(interface) {
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

async fn send_initial_system_stats(
    cli: &cli::Cli,
    sketchybar: &Sketchybar,
    system: &mut System,
    refresh_kind: &sysinfo::RefreshKind,
    buf: &mut String,
) -> Result<()> {
    if cli.all || cli.system.is_some() {
        system.refresh_specifics(*refresh_kind);
        let system_flags = match &cli.system {
            Some(flags) => flags.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
            None => cli::ALL_SYSTEM_FLAGS.to_vec(),
        };
        buf.clear();
        get_system_stats(&system_flags, buf);
        sketchybar
            .send_message("trigger", "system_stats", Some(buf), cli.verbose)
            .await?;
    }

    Ok(())
}

async fn get_stats(cli: &cli::Cli, sketchybar: &Sketchybar) -> Result<()> {
    let refresh_kind = stats::build_refresh_kind();
    let mut system = System::new_with_specifics(refresh_kind);
    let mut disks = Disks::new_with_refreshed_list();
    let mut networks = Networks::new_with_refreshed_list();
    let mut components = Components::new_with_refreshed_list();

    if let Some(network_flags) = &cli.network {
        validate_network_interfaces(&networks, network_flags, cli.verbose)?;
    }

    let flags = process_cli_flags(cli);
    let mut message_buffer = String::with_capacity(512);

    send_initial_system_stats(
        cli,
        sketchybar,
        &mut system,
        &refresh_kind,
        &mut message_buffer,
    )
    .await?;

    let config = StatsConfig {
        flags,
        refresh_kind,
    };

    let mut context = StatsContext {
        system: &mut system,
        disks: &mut disks,
        networks: &mut networks,
        components: &mut components,
        network_baselines: NetworkRateBaselines::default(),
    };

    run_stats_loop(cli, sketchybar, &config, &mut context, &mut message_buffer).await
}

async fn run_stats_loop(
    cli: &cli::Cli,
    sketchybar: &Sketchybar,
    config: &StatsConfig<'_>,
    context: &mut StatsContext<'_>,
    message_buffer: &mut String,
) -> Result<()> {
    let mut network_refresh_tick = 0;

    loop {
        tokio::select! {
            result = collect_and_send_stats(
                cli,
                sketchybar,
                config,
                context,
                network_refresh_tick,
                message_buffer,
            ) => {
                network_refresh_tick = result?;
            }
            _ = tokio::signal::ctrl_c() => {
                if cli.verbose {
                    println!("Received shutdown signal, cleaning up...");
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
    network_refresh_tick: u32,
    buf: &mut String,
) -> Result<u32> {
    let updated_tick = collect_stats_commands(cli, config, context, network_refresh_tick, buf)?;

    if cli.verbose {
        println!("Current message: {}", buf);
    }
    sketchybar
        .send_message("trigger", "system_stats", Some(buf), cli.verbose)
        .await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(cli.interval.into())).await;

    Ok(updated_tick)
}

fn collect_stats_commands(
    cli: &cli::Cli,
    config: &StatsConfig<'_>,
    context: &mut StatsContext<'_>,
    network_refresh_tick: u32,
    buf: &mut String,
) -> Result<u32> {
    buf.clear();

    context.system.refresh_specifics(config.refresh_kind);
    context.disks.refresh(true);
    context.components.refresh(false);

    let mut updated_tick = network_refresh_tick + 1;
    if updated_tick >= cli.network_refresh_rate {
        *context.networks = Networks::new_with_refreshed_list();
        updated_tick = 0;
    } else {
        context.networks.refresh(true);
    }

    let battery_flags: Option<Vec<&str>> = if cli.all {
        Some(cli::ALL_BATTERY_FLAGS.to_vec())
    } else {
        config.flags.battery_flag_refs()
    };
    if let Some(battery_flags) = battery_flags {
        get_battery_stats(&battery_flags, cli.no_units, buf);
    }

    let cpu_flags: Option<Vec<&str>> = if cli.all {
        Some(cli::ALL_CPU_FLAGS.to_vec())
    } else {
        config.flags.cpu_flag_refs()
    };
    if let Some(cpu_flags) = cpu_flags {
        get_cpu_stats(
            context.system,
            context.components,
            &cpu_flags,
            cli.no_units,
            buf,
        );
    }

    let disk_flags: Option<Vec<&str>> = if cli.all {
        Some(cli::ALL_DISK_FLAGS.to_vec())
    } else {
        config.flags.disk_flag_refs()
    };
    if let Some(disk_flags) = disk_flags {
        get_disk_stats(context.disks, &disk_flags, cli.no_units, buf);
    }

    let memory_flags: Option<Vec<&str>> = if cli.all {
        Some(cli::ALL_MEMORY_FLAGS.to_vec())
    } else {
        config.flags.memory_flag_refs()
    };
    if let Some(memory_flags) = memory_flags {
        get_memory_stats(context.system, &memory_flags, cli.no_units, buf);
    }

    let network_interfaces: Option<&[String]> = if cli.all {
        None
    } else {
        config.flags.network_flags
    };
    if cli.all || network_interfaces.is_some() {
        get_network_stats(
            context.networks,
            network_interfaces,
            &mut context.network_baselines,
            cli.no_units,
            buf,
        );
    }

    let uptime_flags: Option<Vec<&str>> = if cli.all {
        Some(cli::ALL_UPTIME_FLAGS.to_vec())
    } else {
        config.flags.uptime_flag_refs()
    };
    if let Some(uptime_flags) = uptime_flags {
        get_uptime_stats(&uptime_flags, buf);
    }

    Ok(updated_tick)
}

fn lock_file_path() -> std::path::PathBuf {
    std::env::temp_dir().join("stats_provider.lock")
}

fn acquire_lock() -> Option<File> {
    let path = lock_file_path();
    let file = File::create(path).ok()?;
    file.try_lock_exclusive().ok()?;
    Some(file)
}

#[cfg(target_os = "macos")]
#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::parse_args();

    let _lock = match acquire_lock() {
        Some(lock) => lock,
        None => {
            eprintln!("another stats_provider instance is already running; exiting");
            return Ok(());
        }
    };

    cli::validate_cli(&cli).context("Invalid CLI arguments")?;

    println!("SketchyBar Stats Provider is running.");

    if cli.verbose {
        println!("Stats Provider CLI: {cli:?}");
    }
    let sketchybar =
        Sketchybar::new(cli.bar.as_deref()).context("Failed to create Sketchybar instance")?;

    sketchybar
        .send_message("add event", "system_stats", None, cli.verbose)
        .await?;

    get_stats(&cli, &sketchybar).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquire_lock_prevents_second_instance() {
        // Host-state-independent: passes whether or not a real instance already
        // holds the lock, but fails if exclusive locking is removed (both opens
        // would then succeed).
        let first = acquire_lock();
        let second = acquire_lock();
        assert!(
            !(first.is_some() && second.is_some()),
            "two concurrent instances should never both acquire the lock"
        );
    }

    #[test]
    fn test_process_cli_flags() {
        let cli = cli::Cli {
            all: false,
            battery: None,
            cpu: Some(vec!["usage".to_string()]),
            disk: None,
            memory: Some(vec!["ram_total".to_string()]),
            network: None,
            system: None,
            uptime: None,
            interval: 5,
            network_refresh_rate: 5,
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
        let flags = ProcessedFlags {
            battery_flags: None,
            cpu_flags: Some(&cpu_flags),
            disk_flags: None,
            memory_flags: None,
            network_flags: None,
            uptime_flags: None,
        };

        let refs = flags.cpu_flag_refs();
        assert!(refs.is_some());
        let refs_vec = refs.unwrap();
        assert_eq!(refs_vec.len(), 2);
        assert_eq!(refs_vec[0], "usage");
        assert_eq!(refs_vec[1], "count");
    }

    #[test]
    fn test_processed_flags_returns_none_when_empty() {
        let flags = ProcessedFlags {
            battery_flags: None,
            cpu_flags: None,
            disk_flags: None,
            memory_flags: None,
            network_flags: None,
            uptime_flags: None,
        };

        assert!(flags.cpu_flag_refs().is_none());
        assert!(flags.disk_flag_refs().is_none());
        assert!(flags.memory_flag_refs().is_none());
        assert!(flags.uptime_flag_refs().is_none());
    }

    #[test]
    fn test_validate_network_interfaces_rejects_unknown() {
        let networks = Networks::new_with_refreshed_list();
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
            memory: None,
            network: None,
            system: None,
            uptime: None,
            interval: 5,
            network_refresh_rate: 5,
            bar: None,
            verbose: false,
            no_units: false,
        };
        let flags = process_cli_flags(&cli);
        let config = StatsConfig {
            flags,
            refresh_kind: stats::build_refresh_kind(),
        };
        let mut system = System::new_with_specifics(stats::build_refresh_kind());
        let mut disks = Disks::new_with_refreshed_list();
        let mut networks = Networks::new_with_refreshed_list();
        let mut components = Components::new_with_refreshed_list();
        let mut context = StatsContext {
            system: &mut system,
            disks: &mut disks,
            networks: &mut networks,
            components: &mut components,
            network_baselines: NetworkRateBaselines::default(),
        };
        let mut buf = String::new();

        let updated_tick =
            collect_stats_commands(&cli, &config, &mut context, 0, &mut buf).unwrap();

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
        assert!(buf.contains("UPTIME=\""), "missing uptime in: {buf}");
        assert!(buf.contains("NETWORK_RX_"), "missing network rx in: {buf}");
        assert!(buf.contains("NETWORK_TX_"), "missing network tx in: {buf}");

        // System stats are startup-only (send_initial_system_stats), so the
        // per-tick buffer must not contain any system keys even with --all.
        // Keys carry the opening quote to avoid substring collisions with
        // e.g. a NETWORK_RX_SYSTEM_NAME= key.
        for key in [
            "ARCH=\"",
            "DISTRO=\"",
            "HOST_NAME=\"",
            "KERNEL_VERSION=\"",
            "SYSTEM_NAME=\"",
            "OS_VERSION=\"",
            "LONG_OS_VERSION=\"",
        ] {
            assert!(
                !buf.contains(key),
                "system key {key} leaked into per-tick buffer: {buf}"
            );
        }

        // Battery is hardware-dependent: percentage and state are always
        // emitted together when a battery exists, otherwise the machine is
        // battery-less and no battery keys appear.
        if buf.contains("BATTERY_PERCENTAGE=") {
            assert!(
                buf.contains("BATTERY_STATE="),
                "battery state missing in: {buf}"
            );
        }

        assert_eq!(
            updated_tick, 1,
            "tick 0 + 1 below refresh rate 5, got {updated_tick}"
        );
    }

    #[test]
    fn test_collect_stats_commands_network_refresh_tick_wraps() {
        let cli = cli::Cli {
            all: true,
            battery: None,
            cpu: None,
            disk: None,
            memory: None,
            network: None,
            system: None,
            uptime: None,
            interval: 5,
            network_refresh_rate: 5,
            bar: None,
            verbose: false,
            no_units: false,
        };
        let flags = process_cli_flags(&cli);
        let config = StatsConfig {
            flags,
            refresh_kind: stats::build_refresh_kind(),
        };
        let mut system = System::new_with_specifics(stats::build_refresh_kind());
        let mut disks = Disks::new_with_refreshed_list();
        let mut networks = Networks::new_with_refreshed_list();
        let mut components = Components::new_with_refreshed_list();
        let mut context = StatsContext {
            system: &mut system,
            disks: &mut disks,
            networks: &mut networks,
            components: &mut components,
            network_baselines: NetworkRateBaselines::default(),
        };
        let mut buf = String::new();

        // tick 4 + 1 == refresh rate 5: re-list the interfaces and reset to 0.
        let wrapped = collect_stats_commands(&cli, &config, &mut context, 4, &mut buf).unwrap();
        assert_eq!(
            wrapped, 0,
            "tick at refresh rate - 1 should wrap to 0, got {wrapped}"
        );

        // tick 0 + 1 < refresh rate 5: just increment.
        let incremented = collect_stats_commands(&cli, &config, &mut context, 0, &mut buf).unwrap();
        assert_eq!(
            incremented, 1,
            "tick below refresh rate should increment, got {incremented}"
        );
    }
}
