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

    /// Show a system tray icon while running (default; disable with --no-tray)
    #[arg(long, default_value_t = true)]
    pub tray: bool,

    /// Run without a system tray icon
    #[arg(long)]
    pub no_tray: bool,

    /// Daily active window, e.g. "9am-5pm" or "21:30-06:00"
    #[arg(
        long,
        value_name = "START-END",
        help = "Daily active window, e.g. \"9am-5pm\" or \"21:30-06:00\""
    )]
    pub active_window: Option<String>,
}
