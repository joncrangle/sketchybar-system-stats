use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "stats_provider", version, about, long_about = None, arg_required_else_help = true)]
pub struct Cli {
    #[arg(short = 'a', long, help = "Get all stats")]
    pub all: bool,

    #[arg(short = 'c', long, num_args = 0.., value_parser = all_cpu_flags())]
    pub cpu: Option<Vec<String>>,

    #[arg(short = 'd', long, num_args = 0.., value_parser = all_disk_flags())]
    pub disk: Option<Vec<String>>,

    #[arg(short = 'm', long, num_args = 0.., value_parser = all_memory_flags())]
    pub memory: Option<Vec<String>>,

    #[arg(
        short = 'i',
        long,
        default_value_t = 5,
        help = "Refresh interval in seconds"
    )]
    pub interval: u32,

    #[arg(short = 'b', long, help = "Bar name (optional)")]
    pub bar: Option<String>,

    #[arg(long, default_value_t = false, help = "Enable verbose output")]
    pub verbose: bool,
}

pub fn parse_args() -> Cli {
    Cli::parse()
}

pub fn all_cpu_flags() -> Vec<&'static str> {
    vec!["count", "temperature", "usage"]
}

pub fn all_disk_flags() -> Vec<&'static str> {
    vec!["count", "free", "total", "usage", "used"]
}

pub fn all_ram_flags() -> Vec<&'static str> {
    vec!["ram_available", "ram_total", "ram_usage", "ram_used"]
}

pub fn all_swp_flags() -> Vec<&'static str> {
    vec!["swp_free", "swp_total", "swp_usage", "swp_used"]
}

pub fn all_memory_flags() -> Vec<&'static str> {
    let mut flags = Vec::new();
    flags.extend(all_ram_flags());
    flags.extend(all_swp_flags());
    flags
}
