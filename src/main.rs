mod cli;
mod platform;
mod runtime;
mod schedule;
mod tray;

use clap::Parser;
use runtime::{AwakeControl, ReconcileOutcome};
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
    let mut controller = platform::new_controller();
    let mut next_refresh = start;

    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(start);
        if duration_limit.is_some_and(|limit| elapsed >= limit) {
            break;
        }

        let schedule_state = active_window
            .as_ref()
            .map(|window| window.status(chrono::Local::now()));
        let schedule_active = schedule_state
            .as_ref()
            .map(|state| matches!(state, ScheduleStatus::Active { .. }))
            .unwrap_or(true);

        let refresh_due = controller.requires_periodic_refresh() && now >= next_refresh;
        let outcome = runtime::reconcile_awake(&mut controller, schedule_active, refresh_due);
        let sync_failed = outcome.is_err();
        let state_changed = matches!(&outcome, Ok(ReconcileOutcome::StateChanged));

        match outcome {
            Ok(ReconcileOutcome::StateChanged) => {
                if args.debug && !args.daemon {
                    println!(
                        "keepawake {} at {:?}",
                        if schedule_active {
                            "enabled"
                        } else {
                            "disabled"
                        },
                        SystemTime::now()
                    );
                }
            }
            Ok(ReconcileOutcome::Refreshed) => {
                if args.debug && !args.daemon {
                    println!("keepawake refresh at {:?}", SystemTime::now());
                }
            }
            Ok(ReconcileOutcome::Unchanged) => {}
            Err(err) => {
                if !args.daemon {
                    eprintln!("Warning: {err}");
                }
            }
        }

        if controller.requires_periodic_refresh() && (refresh_due || state_changed) {
            next_refresh = now + interval;
        }

        let duration_delay = duration_limit.map(|limit| limit.saturating_sub(elapsed));
        let schedule_delay = schedule_state.map(|state| match state {
            ScheduleStatus::Active { remaining } => remaining,
            ScheduleStatus::Inactive { starts_in } => starts_in,
        });
        let refresh_delay = (controller.is_active() && controller.requires_periodic_refresh())
            .then(|| next_refresh.saturating_duration_since(now));
        let retry_delay =
            (sync_failed || schedule_active != controller.is_active()).then_some(interval);

        let sleep_for =
            runtime::minimum_delay(&[duration_delay, schedule_delay, refresh_delay, retry_delay]);

        match sleep_for {
            Some(delay) if delay.is_zero() => thread::sleep(Duration::from_millis(250)),
            Some(delay) => thread::sleep(delay),
            None => thread::park(),
        };
    }

    if let Err(err) = controller.set_active(false)
        && !args.daemon
    {
        eprintln!("Warning: failed to release keep-awake state: {err}");
    }

    if !args.daemon {
        println!("keepawake exiting after {:?}", start.elapsed());
    }
}
