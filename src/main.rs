mod cli;
mod sketchybar;
mod stats;

use anyhow::{Context, Result};
use sketchybar::Sketchybar;
use stats::{get_cpu_stats, get_disk_stats, get_memory_stats, get_network_stats, get_system_stats};
use sysinfo::{Disks, Networks, System};

async fn get_stats(cli: &cli::Cli, sketchybar: &Sketchybar) -> Result<()> {
    let refresh_kind = stats::build_refresh_kind();
    let mut system = System::new_with_specifics(refresh_kind);
    let mut disks = Disks::new_with_refreshed_list();
    let mut networks = Networks::new_with_refreshed_list();
    let mut include_uptime = false;

    let mut network_refresh_counter = 0;
    let network_refresh_rate = 5;

    let cpu_flags = cli
        .cpu
        .as_ref()
        .map(|flags| flags.iter().map(String::as_str).collect::<Vec<&str>>());
    let disk_flags = cli
        .disk
        .as_ref()
        .map(|flags| flags.iter().map(String::as_str).collect::<Vec<&str>>());
    let memory_flags = cli
        .memory
        .as_ref()
        .map(|flags| flags.iter().map(String::as_str).collect::<Vec<&str>>());
    let network_flags = cli.network.as_ref().map(|flags| flags.to_vec());

    // Get system stats that do not change before the main loop
    if cli.all || cli.system.is_some() {
        system.refresh_specifics(refresh_kind);
        let system_flags = match &cli.system {
            Some(flags) => flags.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
            None => cli::all_system_flags(),
        };
        include_uptime = system_flags.contains(&"uptime");
        sketchybar
            .send_message(
                "trigger",
                "system_stats",
                Some(&get_system_stats(&system_flags).join("")),
                cli.verbose,
            )
            .await?;
    };

    loop {
        let mut commands: Vec<String> = Vec::new();
        tokio::time::sleep(tokio::time::Duration::from_secs(cli.interval.into())).await;
        system.refresh_specifics(refresh_kind);
        disks.refresh(true);

        network_refresh_counter += 1;
        if network_refresh_counter >= network_refresh_rate {
            networks = Networks::new_with_refreshed_list();
            network_refresh_counter = 0;
        } else {
            networks.refresh(true);
        }

        if cli.all {
            commands.push(get_cpu_stats(&system, &cli::all_cpu_flags()).join(""));
            commands.push(get_disk_stats(&disks, &cli::all_disk_flags()).join(""));
            commands.push(get_memory_stats(&system, &cli::all_memory_flags()).join(""));
            commands.push(get_network_stats(&networks, None, cli.interval).join(""));
            commands.push(format!("UPTIME=\"{} mins\" ", System::uptime() / 60));
        } else {
            if let Some(cpu_flags) = &cpu_flags {
                commands.push(get_cpu_stats(&system, cpu_flags).join(""));
            }

            if let Some(disk_flags) = &disk_flags {
                commands.push(get_disk_stats(&disks, disk_flags).join(""));
            }

            if let Some(memory_flags) = &memory_flags {
                commands.push(get_memory_stats(&system, memory_flags).join(""));
            }

            if let Some(network_flags) = &network_flags {
                commands
                    .push(get_network_stats(&networks, Some(network_flags), cli.interval).join(""));
            }

            // Get system stat that changes within the main loop
            if include_uptime {
                commands.push(format!("UPTIME=\"{} mins\" ", System::uptime() / 60));
            }
        }

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

#[cfg(target_os = "macos")]
#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::parse_args();
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
