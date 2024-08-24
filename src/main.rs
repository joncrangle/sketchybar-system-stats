extern crate sketchybar_rs;

use clap::Parser;
use std::thread;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use tokio::task;

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

pub struct MessageBuilder {
    parts: Vec<String>,
}

impl MessageBuilder {
    pub fn new() -> Self {
        Self { parts: Vec::new() }
    }

    pub fn add_part(mut self, part: String) -> Self {
        self.parts.push(part);
        self
    }

    pub fn build(self) -> String {
        self.parts.join(" ")
    }
}

const BYTES_PER_GB: f32 = 1_073_741_824.0;

fn get_cpu_stats(s: &System, flags: &[String]) -> String {
    fn cpu_usage(cpu_count: f32, total_usage: f32) -> String {
        let avg_cpu_usage: f32 = (total_usage / cpu_count).round();
        format!("CPU_USAGE=\"{:.0}%\"", avg_cpu_usage)
    }

    let mut result = Vec::new();

    let cpu_count = s.cpus().len() as f32;
    let total_usage: f32 = s.cpus().iter().map(|cpu| cpu.cpu_usage()).sum();

    for flag in flags {
        match flag.as_str() {
            "count" => {
                result.push(format!("CPU_COUNT=\"{}\"", cpu_count));
            }
            "usage" => {
                result.push(cpu_usage(cpu_count, total_usage));
            }
            _ => {
                result.push(format!("CPU_{}=\"\"", flag));
            }
        }
    }

    result.join(" ")
}

fn get_disk_stats(flags: &[String]) -> String {
    let mut result = Vec::new();

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

    for flag in flags {
        match flag.as_str() {
            "count" => {
                result.push(format!("DISK_COUNT=\"{}\"", disks.list().len()));
            }
            "free" => {
                result.push(format!(
                    "DISK_FREE=\"{:.1}GB\"",
                    (total_space as f32 - used_space as f32) / BYTES_PER_GB
                ));
            }
            "total" => {
                result.push(format!(
                    "DISK_TOTAL=\"{:.1}GB\"",
                    total_space as f32 / BYTES_PER_GB
                ));
            }
            "usage" => {
                result.push(format!("DISK_USAGE=\"{:.0}%\"", disk_usage_percentage));
            }
            "used" => {
                result.push(format!(
                    "DISK_USED=\"{:.1}GB\"",
                    used_space as f32 / BYTES_PER_GB
                ));
            }
            _ => {}
        }
    }

    result.join(" ")
}

fn get_memory_stats(s: &System, flags: &[String]) -> String {
    let mut result = Vec::new();

    let total_memory = s.total_memory();
    let used_memory = s.used_memory();
    let memory_usage_percentage = ((used_memory as f32 / total_memory as f32) * 100.0).round();

    for flag in flags {
        match flag.as_str() {
            "free" => {
                result.push(format!(
                    "MEMORY_FREE=\"{:.1}GB\"",
                    (total_memory as f32 - used_memory as f32) / BYTES_PER_GB
                ));
            }
            "total" => {
                result.push(format!(
                    "MEMORY_TOTAL=\"{:.1}GB\"",
                    total_memory as f32 / BYTES_PER_GB
                ));
            }
            "usage" => {
                result.push(format!("MEMORY_USAGE=\"{:.0}%\"", memory_usage_percentage));
            }
            "used" => {
                result.push(format!(
                    "MEMORY_USED=\"{:.1}GB\"",
                    used_memory as f32 / BYTES_PER_GB
                ));
            }
            _ => {}
        }
    }

    result.join(" ")
}

async fn send_to_sketchybar(event: &str, vars: String, verbose: bool) {
    let command = format!("--trigger {} {}", event, vars);

    if let Err(e) = sketchybar_rs::message(&command, None) {
        eprintln!("Failed to send to SketchyBar: {}", e);
    } else if verbose {
        println!("Successfully sent to SketchyBar: {}", command);
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        println!("SketchyBar Stats Provider is running.");
        println!("Stats Provider CLI: {:?}", cli);
    }

    let mut s = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::new().with_cpu_usage())
            .with_memory(MemoryRefreshKind::new().with_ram()),
    );

    loop {
        s.refresh_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::new().with_cpu_usage())
                .with_memory(MemoryRefreshKind::new().with_ram()),
        );
        let mut builder = MessageBuilder::new();

        let mut commands = Vec::new();

        let all_cpu_flags = vec!["count".to_string(), "usage".to_string()];
        let all_memory_flags = vec![
            "free".to_string(),
            "total".to_string(),
            "usage".to_string(),
            "used".to_string(),
        ];
        let all_disk_flags = vec![
            "count".to_string(),
            "free".to_string(),
            "total".to_string(),
            "usage".to_string(),
            "used".to_string(),
        ];

        if cli.all {
            commands.push(get_cpu_stats(&s, &all_cpu_flags));
            commands.push(get_memory_stats(&s, &all_memory_flags));
            commands.push(get_disk_stats(&all_disk_flags));
        } else {
            if let Some(cpu_flags) = &cli.cpu {
                commands.push(get_cpu_stats(&s, cpu_flags));
            }

            if let Some(memory_flags) = &cli.memory {
                commands.push(get_memory_stats(&s, memory_flags));
            }

            if let Some(disk_flags) = &cli.disk {
                commands.push(get_disk_stats(disk_flags));
            }
        }

        for cmd in &commands {
            builder = builder.add_part(cmd.to_string());
        }
        let message = builder.build();

        if cli.verbose {
            println!("Current message: {}", message);
        }
        task::spawn(async move { send_to_sketchybar("system_stats", message, cli.verbose).await });

        thread::sleep(Duration::from_secs(cli.interval.into()));
    }
}
