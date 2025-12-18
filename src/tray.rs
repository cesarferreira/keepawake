use crate::{
    cli::Cli,
    platform,
    schedule::{DailyWindow, ScheduleStatus},
};
use chrono::{Local, Timelike};
use resvg::{tiny_skia, usvg};
use std::time::{Duration, Instant, SystemTime};
use tao::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
};

const STATUS_REFRESH: Duration = Duration::from_secs(30);

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
    let mut next_ping = start;
    let steam_interval = Duration::from_secs(2);
    let mut next_steam: Option<Instant> = None;
    let mut steam_frame = 0usize;
    let mut next_status_refresh = start;
    let has_schedule = active_window.is_some();
    let mut status_dirty = true;

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
        let item = MenuItem::with_id(
            format!("activate_{minutes}m"),
            label.to_string(),
            true,
            None,
        );
        activation_choices.push((
            item,
            ActivationChoice::Timed(Duration::from_secs(minutes * 60)),
        ));
    }

    let until_start_index = activation_choices.len();
    let current_hour = Local::now().hour() as u32;
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
    if let Err(err) = activate_menu.append_items(&activation_refs) {
        if !args.daemon {
            eprintln!("failed to build Activate for submenu: {err}");
        }
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
    ]) {
        if !args.daemon {
            eprintln!("failed to build tray menu: {err}");
        }
    }

    let mut tray_icon: Option<TrayIcon> = None;
    let icon_frames = match build_icon_frames() {
        Ok(frames) => frames,
        Err(err) => {
            if !args.daemon {
                eprintln!("Warning: failed to build tray icon: {err}; using fallback.");
            }
            vec![fallback_icon()]
        }
    };
    if icon_frames.len() > 1 {
        next_steam = Some(start + steam_interval);
    }

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

    let mut status_details = compute_status(&mut plan, active_window.as_ref(), start, Local::now());
    status_item.set_text(&status_details.label);
    pause_item.set_text(match plan {
        ActivationPlan::ManualOff => "Resume now",
        _ => "Pause now",
    });

    event_loop.run(move |event, _, control_flow| match event {
        Event::NewEvents(StartCause::Init) => {
            if tray_icon.is_none() {
                match build_tray_icon(
                    format!("keepawake: {}", status_details.label),
                    Some(title_with_spacing(&status_details.title)),
                    &menu,
                    icon_frames[0].clone(),
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
                        return;
                    }
                }
            }
        }
        Event::MainEventsCleared => {
            let now = Instant::now();
            let now_local = Local::now();
            let now_minutes = now_local.hour() * 60 + now_local.minute();

            for (item, minutes) in until_choices.iter() {
                let enabled = *minutes > now_minutes;
                item.set_enabled(enabled);
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
                    status_dirty = true;
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
                    status_dirty = true;
                    break;
                }
            }

            if let Some(limit) = duration_limit {
                if now.duration_since(start) >= limit {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }

            if let Some(steam_tick) = next_steam {
                if now >= steam_tick && icon_frames.len() > 1 {
                    steam_frame = (steam_frame + 1) % icon_frames.len();
                    if let Some(tray) = tray_icon.as_ref() {
                        if let Err(err) = tray.set_icon(Some(icon_frames[steam_frame].clone())) {
                            if !args.daemon {
                                eprintln!("Warning: failed to update tray icon: {err}");
                            }
                        }
                    }
                    next_steam = Some(now + steam_interval);
                }
            }

            let previous_plan = plan;
            status_details = compute_status(&mut plan, active_window.as_ref(), now, now_local);
            if plan != previous_plan {
                status_dirty = true;
            }

            if status_details.active && now >= next_ping {
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
                next_ping = now + interval;
            } else if now >= next_ping {
                next_ping = now + interval;
            }

            if status_dirty || now >= next_status_refresh {
                status_item.set_text(&status_details.label);
                pause_item.set_text(match plan {
                    ActivationPlan::ManualOff => "Resume now",
                    _ => "Pause now",
                });

                if let Some(tray) = tray_icon.as_ref() {
                    let _ = tray.set_tooltip(Some(format!("keepawake: {}", status_details.label)));
                    let _ = tray.set_title(Some(title_with_spacing(&status_details.title)));
                }

                status_dirty = false;
                next_status_refresh = now + STATUS_REFRESH;
            }

            let mut next_wake = next_status_refresh;
            if let Some(steam_tick) = next_steam {
                if steam_tick < next_wake {
                    next_wake = steam_tick;
                }
            }
            if status_details.active && next_ping < next_wake {
                next_wake = next_ping;
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
        Event::LoopDestroyed => {
            if !args.daemon {
                println!("keepawake exiting after {:?}", start.elapsed());
            }
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
    if let ActivationPlan::ManualTimed { end } = plan {
        if now >= *end {
            *plan = if schedule.is_some() {
                ActivationPlan::FollowSchedule
            } else {
                ActivationPlan::ManualOff
            };
        }
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
        } else if matches!(plan, ActivationPlan::FollowSchedule) && remaining.is_some() {
            let text = format_remaining(remaining.unwrap());
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
        .with_tooltip(tooltip)
        .with_menu(Box::new(menu.clone()));

    if let Some(text) = title {
        builder = builder.with_title(text);
    }

    builder.build().map_err(|err| err.to_string())
}

fn build_icon_frames() -> Result<Vec<Icon>, String> {
    let steam_frames = [
        (3.0f32, 0.10f32),
        (1.5f32, 0.55f32),
        (0.0f32, 0.90f32),
        (-1.5f32, 0.55f32),
    ];

    let mut frames = Vec::with_capacity(steam_frames.len());
    for (offset, opacity) in steam_frames {
        frames.push(render_svg_frame(offset, opacity)?);
    }

    Ok(frames)
}

fn render_svg_frame(steam_offset: f32, steam_opacity: f32) -> Result<Icon, String> {
    const ICON_PX: u32 = 128;
    // These paths come from tray.svg (cup and handle) and tray-animated.svg (steam), scaled via viewBox.
    let steam = |x: u8| -> String {
        format!(
            r#"<path d="M{} {:.2}v6" stroke-opacity="{:.2}" />"#,
            x,
            8.0f32 - steam_offset,
            steam_opacity
        )
    };

    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#f4f7ff" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round">
  {steam1}
  {steam2}
  {steam3}
  <path d="M19 11H5v5a7 7 0 0 0 14 0v-5Z" />
  <path d="M19 13h1a2 2 0 0 1 0 4h-1" />
</svg>"##,
        steam1 = steam(8),
        steam2 = steam(12),
        steam3 = steam(16),
    );

    render_svg_to_icon(&svg, ICON_PX)
}

fn render_svg_to_icon(svg: &str, target_size: u32) -> Result<Icon, String> {
    let options = usvg::Options::default();
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();

    let tree = usvg::Tree::from_str(svg, &options, &fontdb)
        .map_err(|err| format!("failed to parse tray svg: {err}"))?;

    let vb = tree.view_box().rect;
    let scale_x = target_size as f32 / vb.width();
    let scale_y = target_size as f32 / vb.height();
    let scale = scale_x.min(scale_y);
    let transform =
        tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, -vb.x() * scale, -vb.y() * scale);

    let mut pixmap =
        tiny_skia::Pixmap::new(target_size, target_size).ok_or("failed to allocate pixmap")?;
    let mut pixmap_ref = pixmap.as_mut();

    resvg::render(&tree, transform, &mut pixmap_ref);

    Icon::from_rgba(pixmap.data().to_vec(), target_size, target_size).map_err(|err| err.to_string())
}

fn fallback_icon() -> Icon {
    // Simple fallback circle to ensure the tray is not empty if SVG rendering fails.
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
