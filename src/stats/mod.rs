mod cpu;
mod disk;
mod memory;
mod network;
mod system;

use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind};

pub use cpu::get_cpu_stats;
pub use disk::get_disk_stats;
pub use memory::get_memory_stats;
pub use network::get_network_stats;
pub use system::get_system_stats;

pub fn build_refresh_kind() -> RefreshKind {
    RefreshKind::nothing()
        .with_cpu(CpuRefreshKind::nothing().with_cpu_usage().with_frequency())
        .with_memory(MemoryRefreshKind::nothing().with_ram().with_swap())
}
