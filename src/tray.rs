use crate::{cli::Cli, platform};
use resvg::{tiny_skia, usvg};
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
    let steam_interval = Duration::from_secs(2);
    let mut next_steam: Option<Instant> = None;
    let mut steam_frame = 0usize;

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
            "keepawake starting with tray ({}s{}{})",
            args.interval,
            duration_msg,
            if args.debug { ", debug" } else { "" }
        );
    }

    event_loop.run(move |event, _, control_flow| {
        let mut next_wake = next_ping;
        if let Some(steam_tick) = next_steam {
            if steam_tick < next_wake {
                next_wake = steam_tick;
            }
        }
        *control_flow = ControlFlow::WaitUntil(next_wake);

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

                if let Some(steam_tick) = next_steam {
                    if now >= steam_tick && icon_frames.len() > 1 {
                        steam_frame = (steam_frame + 1) % icon_frames.len();
                        if let Some(tray) = tray_icon.as_ref() {
                            if let Err(err) = tray.set_icon(Some(icon_frames[steam_frame].clone()))
                            {
                                if !args.daemon {
                                    eprintln!("Warning: failed to update tray icon: {err}");
                                }
                            }
                        }
                        next_steam = Some(now + steam_interval);
                        let mut target = next_ping;
                        if let Some(steam_next) = next_steam {
                            if steam_next < target {
                                target = steam_next;
                            }
                        }
                        *control_flow = ControlFlow::WaitUntil(target);
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
                    let mut target = next_ping;
                    if let Some(steam_tick) = next_steam {
                        if steam_tick < target {
                            target = steam_tick;
                        }
                    }
                    *control_flow = ControlFlow::WaitUntil(target);
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

fn build_tray_icon(tooltip: String, menu: &Menu, icon: Icon) -> Result<TrayIcon, String> {
    TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip(tooltip)
        .with_menu(Box::new(menu.clone()))
        .build()
        .map_err(|err| err.to_string())
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
