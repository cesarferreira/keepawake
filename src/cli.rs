use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "keepawake",
    about = "Prevent the system from sleeping by periodically pinging OS-specific APIs.",
    version
)]
pub struct Cli {
    /// Call keep-awake every N seconds
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..))]
    pub interval: u64,

    /// Stop after N minutes
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    pub duration: Option<u64>,

    /// Run silently with no output
    #[arg(long)]
    pub daemon: bool,

    /// Print debug logs
    #[arg(long)]
    pub debug: bool,

    /// Show a system tray icon while running
    #[arg(long)]
    pub tray: bool,
}
