mod cli;
mod sketchybar;
mod stats;

use std::fs::File;

use anyhow::{Context, Result};
use fs2::FileExt;
use sketchybar::Sketchybar;
use stats::{
    get_battery_stats, get_cpu_stats, get_disk_stats, get_memory_stats, get_network_stats,
    get_system_stats, get_uptime_stats,
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
    components: &'a Components,
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
            None => cli::all_system_flags(),
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
    let components = Components::new_with_refreshed_list();

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
        components: &components,
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
            result = collect_stats_commands(cli, config, context, network_refresh_tick, message_buffer) => {
                network_refresh_tick = result?;

                if cli.verbose {
                    println!("Current message: {}", message_buffer);
                }
                sketchybar
                    .send_message("trigger", "system_stats", Some(message_buffer), cli.verbose)
                    .await?;
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

async fn collect_stats_commands(
    cli: &cli::Cli,
    config: &StatsConfig<'_>,
    context: &mut StatsContext<'_>,
    network_refresh_tick: u32,
    buf: &mut String,
) -> Result<u32> {
    buf.clear();

    tokio::time::sleep(tokio::time::Duration::from_secs(cli.interval.into())).await;
    context.system.refresh_specifics(config.refresh_kind);
    context.disks.refresh(true);

    let mut updated_tick = network_refresh_tick + 1;
    if updated_tick >= cli.network_refresh_rate {
        *context.networks = Networks::new_with_refreshed_list();
        updated_tick = 0;
    } else {
        context.networks.refresh(true);
    }

    if cli.all {
        get_battery_stats(&cli::all_battery_flags(), cli.no_units, buf);
        get_cpu_stats(
            context.system,
            context.components,
            &cli::all_cpu_flags(),
            cli.no_units,
            buf,
        );
        get_disk_stats(context.disks, &cli::all_disk_flags(), cli.no_units, buf);
        get_memory_stats(context.system, &cli::all_memory_flags(), cli.no_units, buf);
        get_network_stats(context.networks, None, cli.interval, cli.no_units, buf);
        get_uptime_stats(&cli::all_uptime_flags(), buf);
    } else {
        if let Some(battery_flag_refs) = config.flags.battery_flag_refs() {
            get_battery_stats(&battery_flag_refs, cli.no_units, buf);
        }

        if let Some(cpu_flag_refs) = config.flags.cpu_flag_refs() {
            get_cpu_stats(
                context.system,
                context.components,
                &cpu_flag_refs,
                cli.no_units,
                buf,
            );
        }

        if let Some(disk_flag_refs) = config.flags.disk_flag_refs() {
            get_disk_stats(context.disks, &disk_flag_refs, cli.no_units, buf);
        }

        if let Some(memory_flag_refs) = config.flags.memory_flag_refs() {
            get_memory_stats(context.system, &memory_flag_refs, cli.no_units, buf);
        }

        if let Some(network_flags) = config.flags.network_flags {
            get_network_stats(
                context.networks,
                Some(network_flags),
                cli.interval,
                cli.no_units,
                buf,
            );
        }

        if let Some(uptime_flag_refs) = config.flags.uptime_flag_refs() {
            get_uptime_stats(&uptime_flag_refs, buf);
        }
    }

    Ok(updated_tick)
}

fn acquire_lock() -> Option<File> {
    let file = File::create("/tmp/stats_provider.lock").ok()?;
    file.try_lock_exclusive().ok()?;
    Some(file)
}

#[cfg(target_os = "macos")]
#[tokio::main]
async fn main() -> Result<()> {
    let _lock = match acquire_lock() {
        Some(lock) => lock,
        None => return Ok(()),
    };

    let cli = cli::parse_args();

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
}
