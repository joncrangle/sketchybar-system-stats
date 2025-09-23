mod cli;
mod sketchybar;
mod stats;

use anyhow::{Context, Result};
use sketchybar::Sketchybar;
use stats::{
    get_cpu_stats, get_disk_stats, get_memory_stats, get_network_stats, get_system_stats,
    get_uptime_stats,
};
use sysinfo::{Disks, Networks, System};

struct ProcessedFlags {
    cpu_flags: Option<Vec<String>>,
    disk_flags: Option<Vec<String>>,
    memory_flags: Option<Vec<String>>,
    network_flags: Option<Vec<String>>,
    uptime_flags: Option<Vec<String>>,
}

impl ProcessedFlags {
    fn cpu_flag_refs(&self) -> Option<Vec<&str>> {
        self.cpu_flags
            .as_ref()
            .map(|flags| flags.iter().map(String::as_str).collect())
    }

    fn disk_flag_refs(&self) -> Option<Vec<&str>> {
        self.disk_flags
            .as_ref()
            .map(|flags| flags.iter().map(String::as_str).collect())
    }

    fn memory_flag_refs(&self) -> Option<Vec<&str>> {
        self.memory_flags
            .as_ref()
            .map(|flags| flags.iter().map(String::as_str).collect())
    }

    fn uptime_flag_refs(&self) -> Option<Vec<&str>> {
        self.uptime_flags
            .as_ref()
            .map(|flags| flags.iter().map(String::as_str).collect())
    }
}

struct StatsContext<'a> {
    system: &'a mut System,
    disks: &'a mut Disks,
    networks: &'a mut Networks,
}

struct StatsConfig {
    flags: ProcessedFlags,
    refresh_kind: sysinfo::RefreshKind,
}

fn process_cli_flags(cli: &cli::Cli) -> ProcessedFlags {
    ProcessedFlags {
        cpu_flags: cli.cpu.clone(),
        disk_flags: cli.disk.clone(),
        memory_flags: cli.memory.clone(),
        network_flags: cli.network.clone(),
        uptime_flags: cli.uptime.clone(),
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

    // Only fail if no interfaces are available at all
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
) -> Result<()> {
    if cli.all || cli.system.is_some() {
        system.refresh_specifics(*refresh_kind);
        let system_flags = match &cli.system {
            Some(flags) => flags.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
            None => cli::all_system_flags(),
        };
        sketchybar
            .send_message(
                "trigger",
                "system_stats",
                Some(&get_system_stats(&system_flags).join("")),
                cli.verbose,
            )
            .await?;
    }

    Ok(())
}

async fn get_stats(cli: &cli::Cli, sketchybar: &Sketchybar) -> Result<()> {
    let refresh_kind = stats::build_refresh_kind();
    let mut system = System::new_with_specifics(refresh_kind);
    let mut disks = Disks::new_with_refreshed_list();
    let mut networks = Networks::new_with_refreshed_list();

    // Validate network interfaces exist if specified
    if let Some(network_flags) = &cli.network {
        validate_network_interfaces(&networks, network_flags, cli.verbose)?;
    }

    let flags = process_cli_flags(cli);
    send_initial_system_stats(cli, sketchybar, &mut system, &refresh_kind).await?;

    let config = StatsConfig {
        flags,
        refresh_kind,
    };

    let mut context = StatsContext {
        system: &mut system,
        disks: &mut disks,
        networks: &mut networks,
    };

    run_stats_loop(cli, sketchybar, &config, &mut context).await
}

async fn run_stats_loop(
    cli: &cli::Cli,
    sketchybar: &Sketchybar,
    config: &StatsConfig,
    context: &mut StatsContext<'_>,
) -> Result<()> {
    let mut network_refresh_tick = 0;

    loop {
        let (commands, updated_tick) =
            collect_stats_commands(cli, config, context, network_refresh_tick).await?;
        network_refresh_tick = updated_tick;

        if cli.verbose {
            println!("Current message: {}", commands.join(""));
        }
        sketchybar
            .send_message(
                "trigger",
                "system_stats",
                Some(&commands.join("")),
                cli.verbose,
            )
            .await?;
    }
}

async fn collect_stats_commands(
    cli: &cli::Cli,
    config: &StatsConfig,
    context: &mut StatsContext<'_>,
    network_refresh_tick: u32,
) -> Result<(Vec<String>, u32)> {
    let mut commands: Vec<String> = Vec::new();

    tokio::time::sleep(tokio::time::Duration::from_secs(cli.interval.into())).await;
    context.system.refresh_specifics(config.refresh_kind);
    context.disks.refresh(true);

    // Refresh network interfaces less frequently than other stats to reduce overhead.
    // Network interfaces don't change rapidly, so we only refresh the full interface
    // list every N stat collection cycles (configurable via --network-refresh-rate).
    // In between full refreshes, we just update existing interface data.
    let mut updated_tick = network_refresh_tick + 1;
    if updated_tick >= cli.network_refresh_rate {
        *context.networks = Networks::new_with_refreshed_list();
        updated_tick = 0;
    } else {
        context.networks.refresh(true);
    }

    if cli.all {
        commands.push(get_cpu_stats(context.system, &cli::all_cpu_flags()).join(""));
        commands.push(get_disk_stats(context.disks, &cli::all_disk_flags()).join(""));
        commands.push(get_memory_stats(context.system, &cli::all_memory_flags()).join(""));
        commands.push(get_network_stats(context.networks, None, cli.interval).join(""));
        commands.push(get_uptime_stats(&cli::all_uptime_flags()));
    } else {
        if let Some(cpu_flag_refs) = config.flags.cpu_flag_refs() {
            commands.push(get_cpu_stats(context.system, &cpu_flag_refs).join(""));
        }

        if let Some(disk_flag_refs) = config.flags.disk_flag_refs() {
            commands.push(get_disk_stats(context.disks, &disk_flag_refs).join(""));
        }

        if let Some(memory_flag_refs) = config.flags.memory_flag_refs() {
            commands.push(get_memory_stats(context.system, &memory_flag_refs).join(""));
        }

        if let Some(network_flags) = &config.flags.network_flags {
            commands.push(
                get_network_stats(context.networks, Some(network_flags), cli.interval).join(""),
            );
        }

        if let Some(uptime_flag_refs) = config.flags.uptime_flag_refs() {
            commands.push(get_uptime_stats(&uptime_flag_refs));
        }
    }

    Ok((commands, updated_tick))
}

#[cfg(target_os = "macos")]
#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::parse_args();

    // Validate CLI arguments
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
