use crate::{cli::Cli, platform};
use std::time::{Duration, Instant, SystemTime};
use tao::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

pub fn run_with_tray(args: Cli) -> ! {
    let event_loop = EventLoopBuilder::new().build();

    let interval = Duration::from_secs(args.interval);
    let duration_limit = args
        .duration
        .map(|minutes| Duration::from_secs(minutes * 60));
    let start = Instant::now();
    let mut next_ping = start;

    let menu = Menu::new();
    let status_item = MenuItem::with_id(
        "status",
        format!("Keeping awake every {}s", args.interval),
        false,
        None,
    );
    let duration_item = MenuItem::with_id(
        "duration",
        args.duration
            .map(|m| format!("Duration: {m}m"))
            .unwrap_or_else(|| "Duration: until stopped".to_string()),
        false,
        None,
    );
    let debug_item = MenuItem::with_id(
        "debug",
        format!("Debug: {}", if args.debug { "on" } else { "off" }),
        false,
        None,
    );
    let separator = PredefinedMenuItem::separator();
    let quit_item = MenuItem::with_id("quit", "Quit", true, None);
    if let Err(err) = menu.append_items(&[
        &status_item,
        &duration_item,
        &debug_item,
        &separator,
        &quit_item,
    ]) {
        if !args.daemon {
            eprintln!("failed to build tray menu: {err}");
        }
    }

    let mut tray_icon: Option<TrayIcon> = None;

    if !args.daemon {
        let duration_msg = match args.duration {
            Some(minutes) => format!(", duration: {minutes}m"),
            None => String::new(),
        };
        println!(
            "keepawake starting with tray ({}s{}{})",
            args.interval,
            duration_msg,
            if args.debug { ", debug" } else { "" }
        );
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(next_ping);

        match event {
            Event::NewEvents(StartCause::Init) => {
                if tray_icon.is_none() {
                    match build_tray_icon(
                        format!(
                            "keepawake: every {}s{}",
                            args.interval,
                            args.duration
                                .map(|m| format!(", {}m limit", m))
                                .unwrap_or_default()
                        ),
                        &menu,
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

                while let Ok(event) = MenuEvent::receiver().try_recv() {
                    if event.id == quit_item.id() {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                }

                if let Some(limit) = duration_limit {
                    if now.duration_since(start) >= limit {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                }

                if now >= next_ping {
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
                    *control_flow = ControlFlow::WaitUntil(next_ping);
                }
            }
            Event::LoopDestroyed => {
                if !args.daemon {
                    println!("keepawake exiting after {:?}", start.elapsed());
                }
            }
            _ => {}
        }
    })
}

fn build_tray_icon(tooltip: String, menu: &Menu) -> Result<TrayIcon, String> {
    let icon = build_icon()?;

    TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip(tooltip)
        .with_menu(Box::new(menu.clone()))
        .build()
        .map_err(|err| err.to_string())
}

fn build_icon() -> Result<Icon, String> {
    let width = 64u32;
    let height = 64u32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);

    // start transparent
    for _ in 0..(width * height) {
        rgba.extend_from_slice(&[0, 0, 0, 0]);
    }

    let mut set_pixel = |x: i32, y: i32, color: [u8; 4]| {
        if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
            let idx = ((y as u32 * width + x as u32) * 4) as usize;
            rgba[idx..idx + 4].copy_from_slice(&color);
        }
    };

    let cup_color = [240, 240, 240, 255];
    let shadow_color = [200, 200, 200, 255];
    let steam_color = [180, 180, 180, 255];
    let saucer_color = [180, 180, 180, 255];
    let saucer_shadow = [160, 160, 160, 255];

    // cup body (scaled up)
    for y in 26..46 {
        for x in 16..46 {
            set_pixel(x, y, cup_color);
        }
    }
    // cup shadow line
    for x in 16..46 {
        set_pixel(x, 46, shadow_color);
    }
    // handle ring
    for y in 27..45 {
        for x in 44..56 {
            let dx = x as f32 - 45.5;
            let dy = y as f32 - 35.5;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= 8.5 && dist >= 6.0 {
                set_pixel(x, y, cup_color);
            }
        }
    }
    // steam lines (staggered)
    for (x, offset) in [(22, 0), (32, 2), (40, 4)] {
        for y in 12..26 {
            if (y + offset) % 4 != 0 {
                set_pixel(x, y, steam_color);
            }
        }
    }
    // saucer
    for x in 12..52 {
        set_pixel(x, 47, saucer_color);
    }
    for x in 14..50 {
        set_pixel(x, 48, saucer_shadow);
    }

    Icon::from_rgba(rgba, width, height).map_err(|err| err.to_string())
}
