extern crate sketchybar_rs;

use std::thread;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

fn cpu_usage(s: &mut System) -> f32 {
    s.refresh_cpu_all();
    let total_usage: f32 = s.cpus().iter().map(|cpu| cpu.cpu_usage()).sum();
    let cpu_count = s.cpus().len() as f32;
    total_usage / cpu_count
}

fn disk_space() -> (u64, u64) {
    let disks = Disks::new_with_refreshed_list();
    let (total_space, available_space) =
        disks
            .list()
            .iter()
            .fold((0, 0), |(total, available), disk| {
                (
                    total + disk.total_space(),
                    available + disk.available_space(),
                )
            });
    (total_space, available_space)
}

fn memory_usage(s: &System) -> (u64, u64) {
    (s.total_memory(), s.used_memory())
}

fn send_to_sketchybar(event: &str, vars: Option<&str>) {
    let command = match vars {
        Some(v) => format!("--trigger {} {}", event, v),
        None => format!("--trigger {}", event),
    };

    if let Err(e) = sketchybar_rs::message(&command, None) {
        eprintln!("Failed to send to SketchyBar: {}", e);
    } else {
        println!("Successfully sent to SketchyBar: {}", command);
    }
}

fn main() {
    let mut s = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::new().with_cpu_usage())
            .with_memory(MemoryRefreshKind::new().with_ram()),
    );

    loop {
        let cpu_avg_usage = cpu_usage(&mut s);
        let (disk_total, disk_available) = disk_space();
        let (memory_total, memory_used) = memory_usage(&s);

        let disk_used = disk_total - disk_available;
        let disk_usage_percentage = ((disk_used as f32 / disk_total as f32) * 100.0).round() as u32;
        let memory_usage_percentage =
            ((memory_used as f32 / memory_total as f32) * 100.0).round() as u32;

        let vars = format!(
            "CPU_USAGE=\"{:.0}%\" MEMORY_USAGE=\"{:.0}%\" DISK_USAGE=\"{:.0}%\"",
            cpu_avg_usage.round() as u32,
            memory_usage_percentage,
            disk_usage_percentage
        );

        send_to_sketchybar("system_stats", Some(&vars));

        println!("Program is running. Current message: {}", vars);

        thread::sleep(Duration::from_secs(5));
    }
}
