use crate::{cli::Cli, platform};
use std::time::{Duration, Instant, SystemTime};
use tao::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
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
    let quit_item = MenuItem::with_id("quit", "Quit", true, None);
    if let Err(err) = menu.append_items(&[&quit_item]) {
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
                        format!("keepawake ({:?})", env_version_string(&args)),
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
    let width = 32u32;
    let height = 32u32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);

    let center = (width as f32) / 2.0;
    let radius = center - 1.0;

    for y in 0..height as usize {
        for x in 0..width as usize {
            let dx = x as f32 + 0.5 - center;
            let dy = y as f32 + 0.5 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= radius {
                let fade = ((radius - dist) / radius * 80.0) as u8;
                let g = 170u8.saturating_add(fade.min(80));
                rgba.extend_from_slice(&[0, g, 255, 255]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    Icon::from_rgba(rgba, width, height).map_err(|err| err.to_string())
}

fn env_version_string(args: &Cli) -> String {
    let duration = args
        .duration
        .map(|m| format!("{m}m"))
        .unwrap_or_else(|| "∞".to_string());

    format!("{}s/{duration}", args.interval)
}
