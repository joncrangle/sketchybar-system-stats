mod cli;
mod sketchybar;
mod sketchybar2;
mod stats;

use sketchybar::send_to_sketchybar;
use stats::{get_cpu_stats, get_disk_stats, get_memory_stats, get_network_stats, get_system_stats};
use sysinfo::{Disks, Networks, System};

async fn get_stats(cli: &cli::Cli) -> Result<(), Box<dyn std::error::Error>> {
    let refresh_kind = stats::build_refresh_kind();
    let mut system = System::new_with_specifics(refresh_kind);
    let mut disks = Disks::new_with_refreshed_list();
    let mut networks = Networks::new_with_refreshed_list();
    let mut include_uptime = false;

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
        send_to_sketchybar(
            "trigger",
            "system_stats",
            Some(get_system_stats(&system_flags).join("")),
            cli.bar.as_ref(),
            cli.verbose,
        );
    };

    loop {
        let mut commands: Vec<String> = Vec::new();
        tokio::time::sleep(tokio::time::Duration::from_secs(cli.interval.into())).await;
        system.refresh_specifics(refresh_kind);

        if cli.all {
            disks.refresh();
            networks.refresh();
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
                disks.refresh();
                commands.push(get_disk_stats(&disks, disk_flags).join(""));
            }

            if let Some(memory_flags) = &memory_flags {
                commands.push(get_memory_stats(&system, memory_flags).join(""));
            }

            if let Some(network_flags) = &network_flags {
                networks.refresh();
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
        send_to_sketchybar(
            "trigger",
            "system_stats",
            Some(commands.join("")),
            cli.bar.as_ref(),
            cli.verbose,
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::parse_args();

    if cli.verbose {
        println!("SketchyBar Stats Provider is running.");
        println!("Stats Provider CLI: {:?}", cli);
    }

    send_to_sketchybar(
        "add event",
        "system_stats",
        None,
        cli.bar.as_ref(),
        cli.verbose,
    );

    if let Err(e) = get_stats(&cli).await {
        eprintln!("Error occurred: {}", e);
    }
    Ok(())
}
