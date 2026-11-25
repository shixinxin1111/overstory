use crate::{
    commands::{publish, AppState},
    models::*,
};
use serde_json::Value;
use tauri::utils::{config::WindowEffectsConfig, WindowEffect, WindowEffectState};
use tauri::{Emitter, LogicalPosition, LogicalSize, Manager};

#[derive(Clone)]
pub struct Transition {
    pub token: String,
    pub target: Option<Presentation>,
    pub source: String,
}
pub fn window_error(e: tauri::Error) -> AppError {
    AppError::new("WINDOW_FAILED", format!("窗口操作失败：{e}"))
}

pub fn clamp(mut b: Bounds, work: &Bounds, mode: &Presentation) -> Bounds {
    if *mode == Presentation::Orb {
        b.width = 192.;
        b.height = 36.;
    }
    let (min_w, min_h) = match mode {
        Presentation::Orb => (192., 36.),
        Presentation::Panel => (340., 420.),
        Presentation::Main => (700., 500.),
    };
    b.width = b.width.max(min_w).min(work.width);
    b.height = b.height.max(min_h).min(work.height);
    b.x = b.x.max(work.x).min(work.x + work.width - b.width);
    b.y = b.y.max(work.y).min(work.y + work.height - b.height);
    b
}
fn work_area(window: &tauri::WebviewWindow, desired: Option<&Bounds>) -> Result<Bounds> {
    let monitors = window.available_monitors().map_err(window_error)?;
    let selected = desired.and_then(|b| {
        monitors.iter().find(|m| {
            let p = m.position().to_logical::<f64>(m.scale_factor());
            let s = m.size().to_logical::<f64>(m.scale_factor());
            b.x + b.width / 2. >= p.x
                && b.x + b.width / 2. < p.x + s.width
                && b.y + b.height / 2. >= p.y
                && b.y + b.height / 2. < p.y + s.height
        })
    });
    let monitor = selected
        .cloned()
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or_else(|| invalid("无法获取显示器"))?;
    let scale = monitor.scale_factor();
    let p = monitor.work_area().position.to_logical::<f64>(scale);
    let s = monitor.work_area().size.to_logical::<f64>(scale);
    Ok(Bounds {
        x: p.x,
        y: p.y,
        width: s.width,
        height: s.height,
    })
}
// Keep this geometry in sync with WindowSwitches and the window-toolbar CSS:
// 30px buttons, 4px gap, 8px right inset, 36px toolbar height.
fn switch_inset(mode: &Presentation, target: &Presentation) -> f64 {
    let left = match mode {
        Presentation::Main => *target == Presentation::Panel,
        Presentation::Panel | Presentation::Orb => *target == Presentation::Main,
    };
    23. + if left { 34. } else { 0. }
}

pub fn align_switch(
    mut destination: Bounds,
    source: &Bounds,
    source_mode: &Presentation,
    target: &Presentation,
) -> Bounds {
    destination.x = source.x + source.width - switch_inset(source_mode, target)
        + switch_inset(target, source_mode)
        - destination.width;
    destination.y = source.y;
    destination
}

pub fn prepare(app: &tauri::AppHandle, mode: &Presentation) -> Result<()> {
    prepare_at(app, mode, None)
}

fn prepare_at(
    app: &tauri::AppHandle,
    mode: &Presentation,
    source: Option<(&Bounds, &Presentation)>,
) -> Result<()> {
    let state = app.state::<AppState>();
    let s = state.store.lock().unwrap().snapshot.window_state.clone();
    let window = app
        .get_webview_window(if *mode == Presentation::Main {
            "main"
        } else {
            "floating"
        })
        .unwrap();
    let saved = match mode {
        Presentation::Orb => &s.orb_position,
        Presentation::Panel => &s.panel_bounds,
        Presentation::Main => &s.main_bounds,
    };
    let work = work_area(&window, source.map(|(bounds, _)| bounds).or(saved.as_ref()))?;
    let default = match mode {
        Presentation::Orb => Bounds {
            x: work.x + work.width - 216.,
            y: work.y + (work.height - 36.) / 2.,
            width: 192.,
            height: 36.,
        },
        Presentation::Panel => {
            let orb = s.orb_position.as_ref();
            Bounds {
                x: orb
                    .map(|b| b.x + b.width - 380.)
                    .unwrap_or(work.x + work.width - 404.),
                y: orb.map(|b| b.y).unwrap_or(work.y + 100.),
                width: 380.,
                height: 520.,
            }
        }
        Presentation::Main => Bounds {
            x: work.x + (work.width - 1040.) / 2.,
            y: work.y + (work.height - 720.) / 2.,
            width: 1040.,
            height: 720.,
        },
    };
    let mut bounds = clamp(saved.clone().unwrap_or(default), &work, mode);
    if let Some((source_bounds, source_mode)) = source {
        // Preserve the return button's screen position even near a display edge.
        // Clamping x/y here would move the button away from the pointer.
        bounds = align_switch(bounds, source_bounds, source_mode, mode);
    }
    if *mode != Presentation::Main {
        window
            .set_effects(if *mode == Presentation::Panel {
                Some(WindowEffectsConfig {
                    effects: vec![WindowEffect::Popover],
                    state: Some(WindowEffectState::Active),
                    radius: Some(12.),
                    ..Default::default()
                })
            } else {
                None
            })
            .map_err(window_error)?;
        window
            .set_shadow(*mode == Presentation::Panel)
            .map_err(window_error)?;
    }
    window
        .set_resizable(*mode != Presentation::Orb)
        .map_err(window_error)?;
    window
        .set_min_size(if *mode == Presentation::Orb {
            None
        } else {
            Some(LogicalSize::new(
                if *mode == Presentation::Panel {
                    340.
                } else {
                    700.
                },
                if *mode == Presentation::Panel {
                    420.
                } else {
                    500.
                },
            ))
        })
        .map_err(window_error)?;
    window
        .set_size(LogicalSize::new(bounds.width, bounds.height))
        .map_err(window_error)?;
    window
        .set_position(LogicalPosition::new(bounds.x, bounds.y))
        .map_err(window_error)?;
    Ok(())
}
fn capture(window: &tauri::WebviewWindow) -> Result<Bounds> {
    let scale = window.scale_factor().map_err(window_error)?;
    let p = window
        .outer_position()
        .map_err(window_error)?
        .to_logical::<f64>(scale);
    let s = window
        .inner_size()
        .map_err(window_error)?
        .to_logical::<f64>(scale);
    Ok(Bounds {
        x: p.x,
        y: p.y,
        width: s.width,
        height: s.height,
    })
}
pub fn request(app: &tauri::AppHandle, target: Option<Presentation>) -> Result<()> {
    let state = app.state::<AppState>();
    let mode = state
        .store
        .lock()
        .unwrap()
        .snapshot
        .window_state
        .mode
        .clone();
    let mut pending = state.pending.lock().unwrap();
    if let Some(p) = pending.as_mut() {
        p.target = target;
        return Ok(());
    }
    if target.as_ref() == Some(&mode) {
        return Ok(());
    }
    let source = if mode == Presentation::Main {
        "main"
    } else {
        "floating"
    }
    .to_string();
    let token = uuid::Uuid::new_v4().to_string();
    *pending = Some(Transition {
        token: token.clone(),
        target: target.clone(),
        source: source.clone(),
    });
    app.emit_to(
        source,
        "app:prepare-transition",
        serde_json::json!({"token":token,"target":target}),
    )
    .map_err(window_error)?;
    let timeout_app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(8));
        let state = timeout_app.state::<AppState>();
        let mut pending = state.pending.lock().unwrap();
        if pending.as_ref().is_some_and(|p| p.token == token) {
            *pending = None;
            let _ = timeout_app.emit(
                "app:error",
                AppError::new("TRANSITION_TIMEOUT", "窗口未完成保存交接，请重试。"),
            );
        }
    });
    Ok(())
}
pub fn acknowledge(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    token: String,
    session: Value,
    success: bool,
) -> Result<()> {
    let state = app.state::<AppState>();
    let mut guard = state.pending.lock().unwrap();
    let Some(pending) = guard.clone() else {
        return Ok(());
    };
    if pending.token != token || pending.source != window.label() {
        return Err(invalid("无效的窗口交接"));
    }
    *guard = None;
    drop(guard);
    if !success {
        return Err(AppError::new(
            "SAVE_REQUIRED",
            "草稿尚未保存，窗口保持原状。请重试保存。",
        ));
    }
    let mut store = state.store.lock().unwrap();
    let original = store.snapshot.window_state.clone();
    let mut candidate = store.snapshot.clone();
    let bounds = capture(window)?;
    match original.mode {
        Presentation::Orb => candidate.window_state.orb_position = Some(bounds.clone()),
        Presentation::Panel => candidate.window_state.panel_bounds = Some(bounds.clone()),
        Presentation::Main => candidate.window_state.main_bounds = Some(bounds.clone()),
    }
    candidate.window_state.monitor_info = Some(work_area(window, None)?);
    candidate.session = session;
    if let Some(target) = &pending.target {
        candidate.window_state.mode = target.clone();
    }
    // Persist before hiding the source. A failed disk write must leave the editor visible.
    let snapshot = if store.load_error.is_some() {
        if pending.target.is_none() {
            app.exit(0);
            return Ok(());
        }
        candidate.revision = store.snapshot.revision;
        store.snapshot = candidate;
        store.snapshot.clone()
    } else {
        store.transact(|latest| {
            latest.window_state = candidate.window_state;
            latest.session = candidate.session;
            Ok(true)
        })?
    };
    drop(store);
    if pending.target.is_none() {
        app.exit(0);
        return Ok(());
    }
    let target = pending.target.unwrap();
    let destination = app
        .get_webview_window(if target == Presentation::Main {
            "main"
        } else {
            "floating"
        })
        .unwrap();
    let switch = (|| -> Result<()> {
        window.hide().map_err(window_error)?;
        prepare_at(app, &target, Some((&bounds, &original.mode)))?;
        publish(app, &snapshot);
        // The destination commits its new UI before renderer_ready reveals it.
        destination
            .emit("app:present-window", &snapshot)
            .map_err(window_error)?;
        Ok(())
    })();
    if let Err(error) = switch {
        if destination.label() != window.label() {
            let _ = destination.hide();
        }
        let mut store = state.store.lock().unwrap();
        if let Ok(s) = store.transact(|latest| {
            latest.window_state = original.clone();
            Ok(true)
        }) {
            publish(app, &s);
        }
        drop(store);
        let _ = prepare(app, &original.mode);
        let _ = window.show();
        return Err(error);
    }
    Ok(())
}
