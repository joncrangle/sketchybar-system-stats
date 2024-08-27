mod cli;
mod sketchybar;
mod stats;

use sketchybar::send_to_sketchybar;
use stats::{get_cpu_stats, get_disk_stats, get_memory_stats, get_network_stats, get_system_stats};
use std::thread;
use std::time::Duration;
use sysinfo::{Disks, Networks, System};

fn main() {
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

    let refresh_kind = stats::build_refresh_kind();
    let mut system = System::new_with_specifics(refresh_kind);
    let mut disks = Disks::new_with_refreshed_list();
    let mut networks = Networks::new_with_refreshed_list();
    let mut include_uptime = false;

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
            Some(get_system_stats(&system_flags)),
            cli.bar.as_ref(),
            cli.verbose,
        );
    };

    loop {
        system.refresh_specifics(refresh_kind);
        disks.refresh();
        networks.refresh();

        let mut commands = String::new();

        if cli.all {
            commands.push_str(&get_cpu_stats(&system, &cli::all_cpu_flags()));
            commands.push(' ');
            commands.push_str(&get_disk_stats(&disks, &cli::all_disk_flags()));
            commands.push(' ');
            commands.push_str(&get_memory_stats(&system, &cli::all_memory_flags()));
            commands.push(' ');
            commands.push_str(&get_network_stats(&networks, None, cli.interval));
            commands.push(' ');
        } else {
            if let Some(cpu_flags) = &cli.cpu {
                commands.push_str(&get_cpu_stats(
                    &system,
                    &cpu_flags.iter().map(String::as_str).collect::<Vec<&str>>(),
                ));
                commands.push(' ');
            }

            if let Some(disk_flags) = &cli.disk {
                commands.push_str(&get_disk_stats(
                    &disks,
                    &disk_flags.iter().map(String::as_str).collect::<Vec<&str>>(),
                ));
                commands.push(' ');
            }

            if let Some(memory_flags) = &cli.memory {
                commands.push_str(&get_memory_stats(
                    &system,
                    &memory_flags
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<&str>>(),
                ));
                commands.push(' ');
            }

            if let Some(networks_flags) = &cli.network {
                commands.push_str(&get_network_stats(
                    &networks,
                    Some(&networks_flags),
                    cli.interval,
                ));
                commands.push(' ');
            }

            // Get system stat that changes within the main loop
            if include_uptime {
                commands.push_str(&format!("UPTIME=\"{} mins\" ", System::uptime() / 60));
                commands.push(' ');
            }
        }

        let message = commands.trim_end().to_string();

        if cli.verbose {
            println!("Current message: {}", message);
        }
        send_to_sketchybar(
            "trigger",
            "system_stats",
            Some(message),
            cli.bar.as_ref(),
            cli.verbose,
        );

        thread::sleep(Duration::from_secs(cli.interval.into()));
    }
}
