mod cpu;
mod disk;
mod memory;

use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind};

pub use cpu::get_cpu_stats;
pub use disk::get_disk_stats;
pub use memory::get_memory_stats;

pub fn build_refresh_kind() -> RefreshKind {
    RefreshKind::new()
        .with_cpu(CpuRefreshKind::new().with_cpu_usage())
        .with_memory(MemoryRefreshKind::new().with_ram().with_swap())
}
