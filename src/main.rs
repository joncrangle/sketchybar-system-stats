extern crate sketchybar_rs;

use clap::Parser;
use std::thread;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

#[derive(Parser, Debug)]
#[command(name = "stats_provider", version, about, long_about = None, arg_required_else_help = true)]
struct Cli {
    #[arg(short = 'a', long, help = "Get all stats")]
    all: bool,

    #[arg(short = 'c', long, num_args = 0.., value_parser = ["count", "usage"])]
    cpu: Option<Vec<String>>,

    #[arg(short = 'd', long, num_args = 0.., value_parser = ["count", "free", "total", "usage", "used"])]
    disk: Option<Vec<String>>,

    #[arg(short = 'm', long, num_args = 0.., value_parser = ["free", "total", "usage", "used"])]
    memory: Option<Vec<String>>,

    #[arg(
        short = 'i',
        long,
        default_value_t = 5,
        help = "Refresh interval in seconds"
    )]
    interval: u32,

    #[arg(long, default_value_t = false, help = "Enable verbose output")]
    verbose: bool,
}

const BYTES_PER_GB: f32 = 1_073_741_824.0;

fn get_cpu_stats(s: &System, flags: &[&str]) -> String {
    let cpu_count = s.cpus().len() as f32;
    let total_usage: f32 = s.cpus().iter().map(|cpu| cpu.cpu_usage()).sum();

    let mut result = String::with_capacity(100);

    for &flag in flags {
        match flag {
            "count" => {
                result.push_str(&format!("CPU_COUNT=\"{}\" ", cpu_count));
            }
            "usage" => {
                let avg_cpu_usage: f32 = (total_usage / cpu_count).round();
                result.push_str(&format!("CPU_USAGE=\"{:.0}%\" ", avg_cpu_usage));
            }
            _ => {}
        }
    }

    result.trim_end().to_string()
}

fn get_disk_stats(flags: &[&str]) -> String {
    let disks = Disks::new_with_refreshed_list();
    let (total_space, used_space) = disks.list().iter().fold((0, 0), |(total, used), disk| {
        (
            total + disk.total_space(),
            used + disk.total_space() - disk.available_space(),
        )
    });
    let disk_usage_percentage = if total_space > 0 {
        ((used_space as f32 / total_space as f32) * 100.0).round() as u32
    } else {
        0
    };

    let mut result = String::with_capacity(100);

    for &flag in flags {
        match flag {
            "count" => {
                result.push_str(&format!("DISK_COUNT=\"{}\" ", disks.list().len()));
            }
            "free" => {
                result.push_str(&format!(
                    "DISK_FREE=\"{:.1}GB\" ",
                    (total_space as f32 - used_space as f32) / BYTES_PER_GB
                ));
            }
            "total" => {
                result.push_str(&format!(
                    "DISK_TOTAL=\"{:.1}GB\" ",
                    total_space as f32 / BYTES_PER_GB
                ));
            }
            "usage" => {
                result.push_str(&format!("DISK_USAGE=\"{:.0}%\" ", disk_usage_percentage));
            }
            "used" => {
                result.push_str(&format!(
                    "DISK_USED=\"{:.1}GB\" ",
                    used_space as f32 / BYTES_PER_GB
                ));
            }
            _ => {}
        }
    }

    result.trim_end().to_string()
}

fn get_memory_stats(s: &System, flags: &[&str]) -> String {
    let total_memory = s.total_memory();
    let used_memory = s.used_memory();
    let memory_usage_percentage = ((used_memory as f32 / total_memory as f32) * 100.0).round();

    let mut result = String::with_capacity(100);

    for &flag in flags {
        match flag {
            "free" => {
                result.push_str(&format!(
                    "MEMORY_FREE=\"{:.1}GB\" ",
                    (total_memory as f32 - used_memory as f32) / BYTES_PER_GB
                ));
            }
            "total" => {
                result.push_str(&format!(
                    "MEMORY_TOTAL=\"{:.1}GB\" ",
                    total_memory as f32 / BYTES_PER_GB
                ));
            }
            "usage" => {
                result.push_str(&format!(
                    "MEMORY_USAGE=\"{:.0}%\" ",
                    memory_usage_percentage
                ));
            }
            "used" => {
                result.push_str(&format!(
                    "MEMORY_USED=\"{:.1}GB\" ",
                    used_memory as f32 / BYTES_PER_GB
                ));
            }
            _ => {}
        }
    }

    result.trim_end().to_string()
}

fn send_to_sketchybar(event: &str, vars: String, verbose: bool) {
    let command = format!("--trigger {} {}", event, vars);

    if let Err(e) = sketchybar_rs::message(&command, None) {
        eprintln!("Failed to send to SketchyBar: {}", e);
    } else if verbose {
        println!("Successfully sent to SketchyBar: {}", command);
    }
}

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        println!("SketchyBar Stats Provider is running.");
        println!("Stats Provider CLI: {:?}", cli);
    }

    let refresh_kind = RefreshKind::new()
        .with_cpu(CpuRefreshKind::new().with_cpu_usage())
        .with_memory(MemoryRefreshKind::new().with_ram());

    let mut s = System::new_with_specifics(refresh_kind.clone());

    let all_cpu_flags = &["count", "usage"];
    let all_memory_flags = &["free", "total", "usage", "used"];
    let all_disk_flags = &["count", "free", "total", "usage", "used"];

    loop {
        s.refresh_specifics(refresh_kind.clone());

        let mut commands = String::with_capacity(300);

        if cli.all {
            commands.push_str(&get_cpu_stats(&s, all_cpu_flags));
            commands.push_str(" ");
            commands.push_str(&get_memory_stats(&s, all_memory_flags));
            commands.push_str(" ");
            commands.push_str(&get_disk_stats(all_disk_flags));
        } else {
            if let Some(cpu_flags) = &cli.cpu {
                commands.push_str(&get_cpu_stats(
                    &s,
                    &cpu_flags.iter().map(String::as_str).collect::<Vec<&str>>(),
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

            if let Some(disk_flags) = &cli.disk {
                commands.push_str(&get_disk_stats(
                    &disk_flags.iter().map(String::as_str).collect::<Vec<&str>>(),
                ));
                commands.push_str(" ");
            }
        }

        let message = commands.trim_end().to_string();

        if cli.verbose {
            println!("Current message: {}", message);
        }
        send_to_sketchybar("system_stats", message, cli.verbose);

        thread::sleep(Duration::from_secs(cli.interval.into()));
    }
}
