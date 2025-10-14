mod cli;
mod sketchybar;
mod stats;

use anyhow::{Context, Result};
use sketchybar::Sketchybar;
use stats::{
    get_cpu_stats, get_disk_stats, get_memory_stats, get_network_stats, get_system_stats,
    get_uptime_stats,
};
use sysinfo::{Components, Disks, Networks, System};

struct ProcessedFlags<'a> {
    cpu_flags: Option<&'a [String]>,
    disk_flags: Option<&'a [String]>,
    memory_flags: Option<&'a [String]>,
    network_flags: Option<&'a [String]>,
    uptime_flags: Option<&'a [String]>,
}

impl<'a> ProcessedFlags<'a> {
    fn cpu_flag_refs(&self) -> Option<Vec<&str>> {
        self.cpu_flags
            .map(|flags| flags.iter().map(String::as_str).collect())
    }

    fn disk_flag_refs(&self) -> Option<Vec<&str>> {
        self.disk_flags
            .map(|flags| flags.iter().map(String::as_str).collect())
    }

    fn memory_flag_refs(&self) -> Option<Vec<&str>> {
        self.memory_flags
            .map(|flags| flags.iter().map(String::as_str).collect())
    }

    fn uptime_flag_refs(&self) -> Option<Vec<&str>> {
        self.uptime_flags
            .map(|flags| flags.iter().map(String::as_str).collect())
    }
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

    for interface in requested_interfaces {
        if !available_interfaces.contains(interface) && verbose {
            eprintln!(
                "Warning: Network interface '{}' not found. Available interfaces: {}",
                interface,
                available_interfaces.join(", ")
            );
        }
    }

    if available_interfaces.is_empty() {
        anyhow::bail!("No network interfaces available on this system");
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

#[cfg(target_os = "macos")]
#[tokio::main]
async fn main() -> Result<()> {
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
