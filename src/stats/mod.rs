mod battery;
mod constants;
mod cpu;
mod disk;
mod gpu;
mod load;
mod memory;
mod network;
mod system;
mod uptime;

use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind};

use crate::cli;

pub use battery::get_battery_stats;
pub use constants::*;
pub use cpu::get_cpu_stats;
pub use disk::get_disk_stats;
pub use gpu::get_gpu_stats;
pub use load::get_load_stats;
pub use memory::get_memory_stats;
pub use network::NetworkRateBaselines;
pub use network::get_network_stats;
pub use system::get_system_stats;
pub use uptime::get_uptime_stats;

/// Builds a refresh kind covering only the CPU and memory data read by the selected flags.
pub fn build_refresh_kind(
    cpu_flags: Option<&[&str]>,
    memory_flags: Option<&[&str]>,
) -> RefreshKind {
    let mut refresh_kind = RefreshKind::nothing();

    // The CPU list is populated on the first CPU refresh even when nothing else is
    // requested, which `count` relies on. Frequency is only read once by sysinfo.
    if let Some(flags) = cpu_flags {
        let mut cpu = CpuRefreshKind::nothing();
        if flags.contains(&"usage") {
            cpu = cpu.with_usage();
        }
        if flags.contains(&"frequency") {
            cpu = cpu.with_frequency();
        }
        refresh_kind = refresh_kind.with_cpu(cpu);
    }

    if let Some(flags) = memory_flags {
        let mut memory = MemoryRefreshKind::nothing();
        if flags.iter().any(|flag| cli::ALL_RAM_FLAGS.contains(flag)) {
            memory = memory.with_ram();
        }
        if flags.iter().any(|flag| cli::ALL_SWP_FLAGS.contains(flag)) {
            memory = memory.with_swap();
        }
        refresh_kind = refresh_kind.with_memory(memory);
    }

    refresh_kind
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_refresh_kind_nothing_selected() {
        assert_eq!(build_refresh_kind(None, None), RefreshKind::nothing());
    }

    #[test]
    fn test_build_refresh_kind_all_flags_refresh_everything_read() {
        let refresh_kind =
            build_refresh_kind(Some(cli::ALL_CPU_FLAGS), Some(cli::ALL_MEMORY_FLAGS));

        let cpu = refresh_kind.cpu().expect("CPU refresh");
        assert!(cpu.usage());
        assert!(cpu.frequency());
        let memory = refresh_kind.memory().expect("memory refresh");
        assert!(memory.ram());
        assert!(memory.swap());
    }

    #[test]
    fn test_build_refresh_kind_skips_unselected_data() {
        let refresh_kind = build_refresh_kind(Some(&["count"]), Some(&["swp_used"]));

        let cpu = refresh_kind.cpu().expect("CPU list refresh for count");
        assert!(!cpu.usage());
        assert!(!cpu.frequency());
        let memory = refresh_kind.memory().expect("memory refresh");
        assert!(!memory.ram());
        assert!(memory.swap());
    }
}
