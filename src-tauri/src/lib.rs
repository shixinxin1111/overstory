mod cli_install;
mod commands;
mod data_sync;
mod skill_install;
pub(crate) use overstory_core::{backup, models, modules, storage};
#[cfg(test)]
mod tests;
mod window_controller;

use commands::AppState;
use models::Presentation;
use std::sync::Mutex;
use tauri::utils::{config::WindowEffectsConfig, WindowEffect, WindowEffectState};
use tauri::{LogicalPosition, Manager, TitleBarStyle, WebviewUrl, WebviewWindowBuilder};

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let store = storage::Store::open(
                storage::default_data_dir().map_err(|e| std::io::Error::other(e.message))?,
            )
            .map_err(|e| std::io::Error::other(e.message))?;
            app.manage(AppState {
                store: Mutex::new(store),
                pending: Mutex::new(None),
                backup: Mutex::new(None),
                data_watch: Mutex::new(None),
            });
            *app.state::<AppState>().data_watch.lock().unwrap() =
                Some(data_sync::watch(app.handle())?);
            WebviewWindowBuilder::new(app, "floating", WebviewUrl::App("index.html".into()))
                .title("Overstory")
                .inner_size(192., 36.)
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .visible_on_all_workspaces(true)
                .resizable(false)
                .shadow(false)
                .visible(false)
                .focused(false)
                .accept_first_mouse(true)
                .build()?;
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Overstory")
                .inner_size(1040., 720.)
                .min_inner_size(700., 500.)
                .transparent(true)
                .title_bar_style(TitleBarStyle::Overlay)
                .hidden_title(true)
                .accept_first_mouse(true)
                .traffic_light_position(LogicalPosition::new(16., 20.))
                .effects(WindowEffectsConfig {
                    effects: vec![WindowEffect::Sidebar],
                    state: Some(WindowEffectState::FollowsWindowActiveState),
                    ..Default::default()
                })
                .visible(false)
                .build()?;
            let mode = app
                .state::<AppState>()
                .store
                .lock()
                .unwrap()
                .snapshot
                .window_state
                .mode
                .clone();
            window_controller::prepare(app.handle(), &mode)
                .map_err(|e| std::io::Error::other(e.message))?;
            use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
            let handle = app.handle();
            let main = MenuItem::with_id(handle, "open-main", "打开主窗", true, Some("Cmd+1"))?;
            let panel = MenuItem::with_id(handle, "open-panel", "展开面板", true, Some("Cmd+2"))?;
            let orb = MenuItem::with_id(handle, "collapse", "切换迷你悬浮窗", true, Some("Cmd+3"))?;
            let quit = MenuItem::with_id(handle, "quit", "退出 Overstory", true, Some("Cmd+Q"))?;
            let app_menu = Submenu::with_items(
                handle,
                "Overstory",
                true,
                &[
                    &main,
                    &panel,
                    &orb,
                    &PredefinedMenuItem::separator(handle)?,
                    &quit,
                ],
            )?;
            let edit = Submenu::with_items(
                handle,
                "编辑",
                true,
                &[
                    &PredefinedMenuItem::undo(handle, None)?,
                    &PredefinedMenuItem::redo(handle, None)?,
                    &PredefinedMenuItem::separator(handle)?,
                    &PredefinedMenuItem::cut(handle, None)?,
                    &PredefinedMenuItem::copy(handle, None)?,
                    &PredefinedMenuItem::paste(handle, None)?,
                    &PredefinedMenuItem::select_all(handle, None)?,
                ],
            )?;
            app.set_menu(Menu::with_items(handle, &[&app_menu, &edit])?)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            let target = match event.id().as_ref() {
                "open-main" => Some(Presentation::Main),
                "open-panel" => Some(Presentation::Panel),
                "collapse" => Some(Presentation::Orb),
                "quit" => None,
                _ => return,
            };
            if let Err(e) = window_controller::request(app, target) {
                use tauri::Emitter;
                let _ = app.emit("app:error", e);
            }
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window_controller::request(window.app_handle(), Some(Presentation::Orb));
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::execute_module_action,
            commands::update_preferences,
            commands::request_presentation,
            commands::acknowledge_transition,
            commands::request_quit,
            commands::renderer_ready,
            commands::orb_menu,
            commands::open_bookmark,
            commands::open_external,
            commands::export_backup,
            commands::import_backup,
            cli_install::cli_status,
            cli_install::manage_cli,
            skill_install::skill_status,
            skill_install::manage_skill
        ])
        .build(tauri::generate_context!())
        .expect("无法启动 Overstory");
    app.run(|app, event| match event {
        tauri::RunEvent::Reopen { .. } => {
            let _ = window_controller::request(app, Some(Presentation::Main));
        }
        tauri::RunEvent::ExitRequested {
            code: None, api, ..
        } => {
            api.prevent_exit();
            let _ = window_controller::request(app, None);
        }
        _ => {}
    });
}
