mod cli;
mod sketchybar;
mod stats;

use sketchybar::send_to_sketchybar;
use stats::{get_cpu_stats, get_disk_stats, get_memory_stats};
use std::thread;
use std::time::Duration;
use sysinfo::{Disks, System};

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
    let mut s = System::new_with_specifics(refresh_kind.clone());
    let mut disks = Disks::new_with_refreshed_list();

    loop {
        s.refresh_specifics(refresh_kind.clone());
        disks.refresh();

        let mut commands = String::new();

        if cli.all {
            commands.push_str(&get_cpu_stats(&s, &cli::all_cpu_flags()));
            commands.push_str(" ");
            commands.push_str(&get_disk_stats(&disks, &cli::all_disk_flags()));
            commands.push_str(" ");
            commands.push_str(&get_memory_stats(&s, &cli::all_memory_flags()));
            commands.push_str(" ");
        } else {
            if let Some(cpu_flags) = &cli.cpu {
                commands.push_str(&get_cpu_stats(
                    &s,
                    &cpu_flags.iter().map(String::as_str).collect::<Vec<&str>>(),
                ));
                commands.push_str(" ");
            }

            if let Some(disk_flags) = &cli.disk {
                commands.push_str(&get_disk_stats(
                    &disks,
                    &disk_flags.iter().map(String::as_str).collect::<Vec<&str>>(),
                ));
                commands.push_str(" ");
            }

            if let Some(memory_flags) = &cli.memory {
                commands.push_str(&get_memory_stats(
                    &s,
                    &memory_flags
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<&str>>(),
                ));
                commands.push_str(" ");
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
