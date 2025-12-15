mod cli;
mod platform;
mod tray;

use clap::Parser;
use std::{
    env, thread,
    time::{Duration, Instant, SystemTime},
};

fn main() {
    let args = cli::Cli::parse();

    let use_tray = if args.no_tray { false } else { args.tray };

    if use_tray {
        tray::run_with_tray(args);
    }

    let interval = Duration::from_secs(args.interval);
    let duration_limit = args
        .duration
        .map(|minutes| Duration::from_secs(minutes * 60));

    if !args.daemon {
        let duration_msg = match args.duration {
            Some(minutes) => format!(", duration: {minutes}m"),
            None => String::new(),
        };

        println!(
            "keepawake starting on {} (interval: {}s{})",
            env::consts::OS,
            args.interval,
            duration_msg
        );

        if args.debug {
            println!("debug logging enabled");
        }
    }

    let start = Instant::now();

    loop {
        let elapsed = start.elapsed();
        if let Some(limit) = duration_limit {
            if elapsed >= limit {
                break;
            }
        }

        match platform::keep_awake() {
            Ok(_) => {
                if args.debug && !args.daemon {
                    println!("keepawake ping at {:?}", SystemTime::now());
                }
            }
            Err(err) => {
                if !args.daemon {
                    eprintln!("Warning: {err}");
                }
            }
        }

        let sleep_for = match duration_limit {
            Some(limit) => {
                let elapsed = start.elapsed();
                if elapsed >= limit {
                    break;
                }

                let remaining = limit - elapsed;
                if remaining < interval {
                    remaining
                } else {
                    interval
                }
            }
            None => interval,
        };

        thread::sleep(sleep_for);
    }

    if !args.daemon {
        println!("keepawake exiting after {:?}", start.elapsed());
    }
}
