use crate::{
    cli::Cli,
    platform,
    runtime::{AwakeControl, ReconcileOutcome},
    schedule::{DailyWindow, ScheduleStatus},
};
use chrono::{Local, Timelike};
use std::{
    io::Cursor,
    time::{Duration, Instant, SystemTime},
};
use tao::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
};

const STATUS_REFRESH: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivationPlan {
    FollowSchedule,
    ManualIndefinite,
    ManualTimed { end: Instant },
    ManualOff,
}

#[derive(Debug, Clone, Copy)]
enum ActivationChoice {
    Indefinite,
    Timed(Duration),
    FollowSchedule,
    UntilMinutes(u32),
}

struct StatusDetails {
    active: bool,
    label: String,
    title: String,
    remaining: Option<Duration>,
    starts_in: Option<Duration>,
}

pub fn run_with_tray(args: Cli, active_window: Option<DailyWindow>) -> ! {
    let event_loop = EventLoopBuilder::new().build();

    let interval = Duration::from_secs(args.interval);
    let duration_limit = args
        .duration
        .map(|minutes| Duration::from_secs(minutes * 60));
    let start = Instant::now();
    let mut next_platform_refresh = start;
    let mut next_status_refresh = start + STATUS_REFRESH;
    let has_schedule = active_window.is_some();
    let mut last_until_hour = None;
    let mut controller = platform::new_controller();

    let mut plan = if let Some(limit) = duration_limit {
        ActivationPlan::ManualTimed { end: start + limit }
    } else if has_schedule {
        ActivationPlan::FollowSchedule
    } else {
        ActivationPlan::ManualIndefinite
    };

    let menu = Menu::new();
    let status_item = MenuItem::with_id("status", "Starting…", false, None);
    let interval_item = MenuItem::with_id(
        "interval",
        format!("Interval: {}s", args.interval),
        false,
        None,
    );
    let window_item = MenuItem::with_id(
        "window",
        format!(
            "Daily window: {}",
            active_window
                .as_ref()
                .map(|w| w.label().to_string())
                .unwrap_or_else(|| "off".to_string())
        ),
        false,
        None,
    );
    let debug_item = MenuItem::with_id(
        "debug",
        format!("Debug: {}", if args.debug { "on" } else { "off" }),
        false,
        None,
    );
    let pause_item = MenuItem::with_id("pause", "Pause now", true, None);
    let separator = PredefinedMenuItem::separator();
    let separator2 = PredefinedMenuItem::separator();
    let quit_item = MenuItem::with_id("quit", "Quit", true, None);

    let mut activation_choices: Vec<(MenuItem, ActivationChoice)> = Vec::new();
    let activate_separator = PredefinedMenuItem::separator();
    let mut until_choices: Vec<(MenuItem, u32)> = Vec::new();

    if has_schedule {
        let item = MenuItem::with_id("activate_schedule", "Follow daily window", true, None);
        activation_choices.push((item, ActivationChoice::FollowSchedule));
    }
    let until_stopped = MenuItem::with_id("activate_indef", "Until stopped", true, None);
    activation_choices.push((until_stopped, ActivationChoice::Indefinite));

    for (minutes, label) in [
        (5, "5 minutes"),
        (10, "10 minutes"),
        (15, "15 minutes"),
        (30, "30 minutes"),
        (60, "1 hour"),
        (120, "2 hours"),
        (300, "5 hours"),
    ] {
        let item = MenuItem::with_id(format!("activate_{minutes}m"), label, true, None);
        activation_choices.push((
            item,
            ActivationChoice::Timed(Duration::from_secs(minutes * 60)),
        ));
    }

    let until_start_index = activation_choices.len();
    let current_hour = Local::now().hour();
    if current_hour < 23 {
        for hour in current_hour + 1..24 {
            let minutes = hour * 60;
            let label = format!("Until {}", format_ampm(minutes as u16));
            let item = MenuItem::with_id(format!("activate_until_{hour:02}"), label, true, None);
            activation_choices.push((item.clone(), ActivationChoice::UntilMinutes(minutes)));
            until_choices.push((item, minutes));
        }
    }

    let mut activation_refs: Vec<&dyn IsMenuItem> = Vec::new();
    for (idx, (item, _)) in activation_choices.iter().enumerate() {
        if idx == until_start_index && !until_choices.is_empty() {
            activation_refs.push(&activate_separator);
        }
        activation_refs.push(item as &dyn IsMenuItem);
    }
    let activate_menu = Submenu::with_id("activate_for", "Activate for", true);
    if let Err(err) = activate_menu.append_items(&activation_refs)
        && !args.daemon
    {
        eprintln!("failed to build Activate for submenu: {err}");
    }

    if let Err(err) = menu.append_items(&[
        &status_item,
        &interval_item,
        &window_item,
        &debug_item,
        &separator,
        &activate_menu,
        &pause_item,
        &separator2,
        &quit_item,
    ]) && !args.daemon
    {
        eprintln!("failed to build tray menu: {err}");
    }

    let mut tray_icon: Option<TrayIcon> = None;
    let icon = match load_static_icon() {
        Ok(icon) => icon,
        Err(err) => {
            if !args.daemon {
                eprintln!("Warning: failed to load tray icon: {err}; using fallback.");
            }
            fallback_icon()
        }
    };

    if !args.daemon {
        let duration_msg = match args.duration {
            Some(minutes) => format!(", duration: {minutes}m"),
            None => String::new(),
        };
        println!(
            "keepawake starting with tray ({}s{}{}{})",
            args.interval,
            duration_msg,
            if args.debug { ", debug" } else { "" },
            if has_schedule { ", daily window" } else { "" }
        );
    }

    let initial_status = compute_status(&mut plan, active_window.as_ref(), start, Local::now());
    let mut rendered_label = initial_status.label.clone();
    let mut rendered_pause = match plan {
        ActivationPlan::ManualOff => "Resume now",
        _ => "Pause now",
    }
    .to_string();
    let mut rendered_tooltip = format!("keepawake: {}", initial_status.label);
    let mut rendered_title = title_with_spacing(&initial_status.title);
    status_item.set_text(&rendered_label);
    pause_item.set_text(&rendered_pause);

    event_loop.run(move |event, _, control_flow| match event {
        Event::NewEvents(StartCause::Init) => {
            if tray_icon.is_none() {
                match build_tray_icon(
                    rendered_tooltip.clone(),
                    Some(rendered_title.clone()),
                    &menu,
                    icon.clone(),
                ) {
                    Ok(icon) => {
                        tray_icon = Some(icon);
                        #[cfg(target_os = "macos")]
                        unsafe {
                            use core_foundation::runloop::{CFRunLoopGetMain, CFRunLoopWakeUp};
                            let rl = CFRunLoopGetMain();
                            CFRunLoopWakeUp(rl);
                        }
                    }
                    Err(err) => {
                        if !args.daemon {
                            eprintln!("Warning: failed to create tray icon: {err}");
                        }
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
        }
        Event::MainEventsCleared => {
            let now = Instant::now();
            let now_local = Local::now();
            let now_minutes = now_local.hour() * 60 + now_local.minute();

            if last_until_hour != Some(now_local.hour()) {
                for (item, minutes) in &until_choices {
                    item.set_enabled(*minutes > now_minutes);
                }
                last_until_hour = Some(now_local.hour());
            }

            while let Ok(event) = MenuEvent::receiver().try_recv() {
                if event.id == quit_item.id() {
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                if event.id == pause_item.id() {
                    plan = match plan {
                        ActivationPlan::ManualOff => {
                            if has_schedule {
                                ActivationPlan::FollowSchedule
                            } else {
                                ActivationPlan::ManualIndefinite
                            }
                        }
                        _ => ActivationPlan::ManualOff,
                    };
                    continue;
                }

                for (item, choice) in activation_choices.iter() {
                    if event.id != item.id() {
                        continue;
                    }
                    plan = match choice {
                        ActivationChoice::Indefinite => ActivationPlan::ManualIndefinite,
                        ActivationChoice::Timed(duration) => ActivationPlan::ManualTimed {
                            end: now + *duration,
                        },
                        ActivationChoice::FollowSchedule => ActivationPlan::FollowSchedule,
                        ActivationChoice::UntilMinutes(minutes) => {
                            let target_secs = *minutes * 60;
                            let now_secs = now_local.num_seconds_from_midnight();
                            if target_secs <= now_secs {
                                continue;
                            }
                            ActivationPlan::ManualTimed {
                                end: now + Duration::from_secs((target_secs - now_secs) as u64),
                            }
                        }
                    };
                    break;
                }
            }

            if duration_limit.is_some_and(|limit| now.duration_since(start) >= limit) {
                *control_flow = ControlFlow::Exit;
                return;
            }

            let status_details = compute_status(&mut plan, active_window.as_ref(), now, now_local);

            let refresh_due =
                controller.requires_periodic_refresh() && now >= next_platform_refresh;
            let reconcile = crate::runtime::reconcile_awake(
                &mut controller,
                status_details.active,
                refresh_due,
            );
            match &reconcile {
                Ok(ReconcileOutcome::StateChanged) if args.debug && !args.daemon => {
                    println!(
                        "keepawake {} at {:?}",
                        if status_details.active {
                            "enabled"
                        } else {
                            "disabled"
                        },
                        SystemTime::now()
                    );
                }
                Ok(ReconcileOutcome::Refreshed) if args.debug && !args.daemon => {
                    println!("keepawake refresh at {:?}", SystemTime::now());
                }
                Err(err) if !args.daemon => eprintln!("Warning: {err}"),
                _ => {}
            }
            if controller.requires_periodic_refresh()
                && (refresh_due || matches!(reconcile, Ok(ReconcileOutcome::StateChanged)))
            {
                next_platform_refresh = now + interval;
            }

            if status_details.label != rendered_label {
                status_item.set_text(&status_details.label);
                rendered_label.clone_from(&status_details.label);
            }

            let pause_text = match plan {
                ActivationPlan::ManualOff => "Resume now",
                _ => "Pause now",
            };
            if pause_text != rendered_pause {
                pause_item.set_text(pause_text);
                pause_text.clone_into(&mut rendered_pause);
            }

            if let Some(tray) = tray_icon.as_ref() {
                let tooltip = format!("keepawake: {}", status_details.label);
                if tooltip != rendered_tooltip {
                    if let Err(err) = tray.set_tooltip(Some(&tooltip))
                        && !args.daemon
                    {
                        eprintln!("Warning: failed to update tray tooltip: {err}");
                    }
                    rendered_tooltip = tooltip;
                }

                let title = title_with_spacing(&status_details.title);
                if title != rendered_title {
                    tray.set_title(Some(&title));
                    rendered_title = title;
                }
            }

            if now >= next_status_refresh {
                next_status_refresh = now + STATUS_REFRESH;
            }

            let mut next_wake = next_status_refresh;
            if controller.requires_periodic_refresh() && next_platform_refresh < next_wake {
                next_wake = next_platform_refresh;
            }
            if controller.is_active() != status_details.active {
                let retry = now + interval;
                if retry < next_wake {
                    next_wake = retry;
                }
            }
            if let Some(remaining) = status_details.remaining {
                let end_tick = now + remaining;
                if end_tick < next_wake {
                    next_wake = end_tick;
                }
            }
            if let Some(wait) = status_details.starts_in {
                let start_tick = now + wait;
                if start_tick < next_wake {
                    next_wake = start_tick;
                }
            }

            *control_flow = ControlFlow::WaitUntil(next_wake);
        }
        Event::LoopDestroyed if !args.daemon => {
            println!("keepawake exiting after {:?}", start.elapsed());
        }
        _ => {}
    })
}

fn compute_status(
    plan: &mut ActivationPlan,
    schedule: Option<&DailyWindow>,
    now: Instant,
    now_local: chrono::DateTime<Local>,
) -> StatusDetails {
    if let ActivationPlan::ManualTimed { end } = plan
        && now >= *end
    {
        *plan = if schedule.is_some() {
            ActivationPlan::FollowSchedule
        } else {
            ActivationPlan::ManualOff
        };
    }

    let mut active = false;
    let mut remaining = None;
    let mut starts_in = None;

    match plan {
        ActivationPlan::ManualIndefinite => {
            active = true;
        }
        ActivationPlan::ManualTimed { end } => {
            active = true;
            remaining = Some(end.saturating_duration_since(now));
        }
        ActivationPlan::FollowSchedule => {
            if let Some(window) = schedule {
                match window.status(now_local) {
                    ScheduleStatus::Active { remaining: rem } => {
                        active = true;
                        remaining = Some(rem);
                    }
                    ScheduleStatus::Inactive { starts_in: wait } => {
                        starts_in = Some(wait);
                    }
                }
            } else {
                active = true;
            }
        }
        ActivationPlan::ManualOff => {}
    }

    let (label, title) = if active {
        if let ActivationPlan::ManualTimed { .. } = plan {
            let text = format_remaining(remaining.unwrap_or_else(|| Duration::from_secs(0)));
            (format!("Active — {text} left"), format!("{text} left"))
        } else if matches!(plan, ActivationPlan::FollowSchedule)
            && let Some(remaining) = remaining
        {
            let text = format_remaining(remaining);
            (format!("Active — {text} left in window"), text)
        } else {
            (
                "Active — until stopped".to_string(),
                "until stopped".to_string(),
            )
        }
    } else if matches!(plan, ActivationPlan::ManualOff) {
        (
            "Paused — not keeping awake".to_string(),
            "paused".to_string(),
        )
    } else if let Some(wait) = starts_in {
        let text = format_remaining(wait);
        let at = schedule
            .map(|w| format_clock(w.start_minutes()))
            .unwrap_or_default();
        if at.is_empty() {
            (
                format!("Inactive — starts in {text}"),
                format!("starts in {text}"),
            )
        } else {
            (
                format!("Inactive — starts at {at} ({text})"),
                format!("starts {at}"),
            )
        }
    } else {
        ("Inactive — waiting".to_string(), "idle".to_string())
    };

    StatusDetails {
        active,
        label,
        title,
        remaining,
        starts_in,
    }
}

fn format_remaining(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;

    if hours > 0 {
        format!("{hours}h{minutes:02}")
    } else {
        format!("{minutes}min")
    }
}

fn format_clock(minutes: u16) -> String {
    let hour = minutes / 60;
    let minute = minutes % 60;
    format!("{:02}:{:02}", hour, minute)
}

fn format_ampm(minutes: u16) -> String {
    let hour24 = minutes / 60;
    let minute = minutes % 60;
    let suffix = if hour24 < 12 { "am" } else { "pm" };
    let hour12 = match hour24 % 12 {
        0 => 12,
        v => v,
    };
    if minute == 0 {
        format!("{hour12}{suffix}")
    } else {
        format!("{hour12}:{minute:02}{suffix}")
    }
}

fn title_with_spacing(text: &str) -> String {
    // Prefix with spaces so the tray title sits slightly away from the icon.
    format!("  {text}")
}

fn build_tray_icon(
    tooltip: String,
    title: Option<String>,
    menu: &Menu,
    icon: Icon,
) -> Result<TrayIcon, String> {
    let mut builder = TrayIconBuilder::new()
        .with_icon(icon)
        .with_icon_as_template(cfg!(target_os = "macos"))
        .with_tooltip(tooltip)
        .with_menu(Box::new(menu.clone()));

    if let Some(text) = title {
        builder = builder.with_title(text);
    }

    builder.build().map_err(|err| err.to_string())
}

fn load_static_icon() -> Result<Icon, String> {
    let mut decoder =
        png::Decoder::new(Cursor::new(include_bytes!("../assets/tray.png").as_slice()));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|err| format!("failed to read embedded tray PNG: {err}"))?;
    let output_size = reader
        .output_buffer_size()
        .ok_or("embedded tray PNG is too large to decode")?;
    let mut rgba = vec![0; output_size];
    let info = reader
        .next_frame(&mut rgba)
        .map_err(|err| format!("failed to decode embedded tray PNG: {err}"))?;

    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err(format!(
            "embedded tray PNG decoded as {:?}/{:?}, expected RGBA/8-bit",
            info.color_type, info.bit_depth
        ));
    }

    rgba.truncate(info.buffer_size());
    Icon::from_rgba(rgba, info.width, info.height).map_err(|err| err.to_string())
}

fn fallback_icon() -> Icon {
    // Simple fallback circle to ensure the tray is not empty if the embedded PNG is invalid.
    let size = 64u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let center = (size as f32) / 2.0;
    let radius = center - 2.0;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - center;
            let dy = y as f32 + 0.5 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= radius {
                let idx = ((y * size + x) * 4) as usize;
                rgba[idx..idx + 4].copy_from_slice(&[50, 120, 220, 255]);
            }
        }
    }
    Icon::from_rgba(rgba, size, size).expect("fallback icon must be valid")
}

#[cfg(test)]
mod tests {
    use super::{ActivationPlan, compute_status, load_static_icon};
    use crate::schedule::DailyWindow;
    use chrono::Local;
    use std::time::Instant;

    #[test]
    fn embedded_static_icon_decodes() {
        assert!(load_static_icon().is_ok());
    }

    #[test]
    fn expired_manual_activation_turns_off_without_schedule() {
        let now = Instant::now();
        let mut plan = ActivationPlan::ManualTimed { end: now };

        let status = compute_status(&mut plan, None, now, Local::now());

        assert_eq!(plan, ActivationPlan::ManualOff);
        assert!(!status.active);
    }

    #[test]
    fn expired_manual_activation_returns_to_schedule() {
        let now = Instant::now();
        let schedule = DailyWindow::parse("00:00-00:00").unwrap();
        let mut plan = ActivationPlan::ManualTimed { end: now };

        let status = compute_status(&mut plan, Some(&schedule), now, Local::now());

        assert_eq!(plan, ActivationPlan::FollowSchedule);
        assert!(status.active);
    }
}
