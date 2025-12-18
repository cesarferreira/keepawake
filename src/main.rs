mod cli;
mod platform;
mod schedule;
mod tray;

use clap::Parser;
use schedule::ScheduleStatus;
use std::{
    env, process, thread,
    time::{Duration, Instant, SystemTime},
};

fn main() {
    let args = cli::Cli::parse();

    let active_window = args
        .active_window
        .as_deref()
        .map(schedule::DailyWindow::parse)
        .transpose()
        .unwrap_or_else(|err| {
            eprintln!("Invalid --active-window value: {err}");
            process::exit(2);
        });

    let use_tray = if args.no_tray { false } else { args.tray };

    if use_tray {
        tray::run_with_tray(args, active_window);
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

        let schedule_state = active_window
            .as_ref()
            .map(|window| window.status(chrono::Local::now()));
        let schedule_active = schedule_state
            .as_ref()
            .map(|state| matches!(state, ScheduleStatus::Active { .. }))
            .unwrap_or(true);

        if schedule_active {
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
        } else if args.debug && !args.daemon {
            println!(
                "keepawake idle (outside active window) at {:?}",
                SystemTime::now()
            );
        }

        let mut sleep_for = match duration_limit {
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

        if let Some(state) = schedule_state {
            let until_change = match state {
                ScheduleStatus::Active { remaining } => remaining,
                ScheduleStatus::Inactive { starts_in } => starts_in,
            };
            if until_change < sleep_for {
                sleep_for = until_change;
            }
        }

        if sleep_for.is_zero() {
            sleep_for = Duration::from_millis(250);
        }

        thread::sleep(sleep_for);
    }

    if !args.daemon {
        println!("keepawake exiting after {:?}", start.elapsed());
    }
}
