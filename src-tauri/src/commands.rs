use crate::{backup::Backup, models::*, modules, storage::Store, window_controller};
use serde_json::Value;
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

pub struct AppState {
    pub store: Mutex<Store>,
    pub pending: Mutex<Option<window_controller::Transition>>,
    pub backup: Mutex<Option<Backup>>,
    pub data_watch: Mutex<Option<notify::RecommendedWatcher>>,
}
pub fn publish(app: &tauri::AppHandle, snapshot: &Snapshot) {
    let _ = app.emit("app:snapshot-changed", snapshot);
}

#[tauri::command]
pub fn get_snapshot(state: State<AppState>) -> Result<Snapshot> {
    state.store.lock().unwrap().read()
}
#[tauri::command]
pub fn execute_module_action(
    app: tauri::AppHandle,
    state: State<AppState>,
    module_id: String,
    action: Value,
) -> Result<Snapshot> {
    let mut store = state.store.lock().unwrap();
    let result = overstory_core::service::execute(&mut store, &module_id, &action);
    // A rejected edit can still have loaded a newer external snapshot.
    if store.load_error.is_none() {
        publish(&app, &store.snapshot);
    }
    let s = result?.snapshot;
    Ok(s)
}
#[tauri::command]
pub fn update_preferences(
    app: tauri::AppHandle,
    state: State<AppState>,
    preferences: Preferences,
    expected_revision: u64,
) -> Result<Snapshot> {
    let mut store = state.store.lock().unwrap();
    let result = store.transact(|s| {
        if s.revision != expected_revision {
            return Err(AppError::new("STALE_REVISION", "数据已更新，请重试"));
        }
        if s.preferences.show_todo_count == preferences.show_todo_count
            && s.preferences.module_order == preferences.module_order
        {
            return Ok(false);
        }
        s.preferences = preferences;
        Ok(true)
    });
    if store.load_error.is_none() {
        publish(&app, &store.snapshot);
    }
    let s = result?;
    Ok(s)
}
#[tauri::command]
pub fn request_presentation(app: tauri::AppHandle, presentation: Presentation) -> Result<()> {
    window_controller::request(&app, Some(presentation))
}
#[tauri::command]
pub fn request_quit(app: tauri::AppHandle) -> Result<()> {
    window_controller::request(&app, None)
}
#[tauri::command]
pub fn orb_menu(window: tauri::WebviewWindow) -> Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    let app = window.app_handle();
    let panel = MenuItem::with_id(app, "open-panel", "展开面板", true, None::<&str>)
        .map_err(window_controller::window_error)?;
    let main = MenuItem::with_id(app, "open-main", "打开主窗", true, None::<&str>)
        .map_err(window_controller::window_error)?;
    let quit = MenuItem::with_id(app, "quit", "退出 Overstory", true, None::<&str>)
        .map_err(window_controller::window_error)?;
    let separator = PredefinedMenuItem::separator(app).map_err(window_controller::window_error)?;
    let menu = Menu::with_items(app, &[&panel, &main, &separator, &quit])
        .map_err(window_controller::window_error)?;
    window
        .popup_menu(&menu)
        .map_err(window_controller::window_error)
}
#[tauri::command]
pub fn acknowledge_transition(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    token: String,
    session: Value,
    success: bool,
) -> Result<()> {
    window_controller::acknowledge(&app, &window, token, session, success)
}
#[tauri::command]
pub fn renderer_ready(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    presentation: Option<Presentation>,
) -> Result<()> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    if presentation
        .as_ref()
        .is_some_and(|mode| mode != &store.snapshot.window_state.mode)
    {
        return Ok(());
    }
    let mode = store.snapshot.window_state.mode.clone();
    drop(store);
    let relevant = if mode == Presentation::Main {
        "main"
    } else {
        "floating"
    };
    if window.label() == relevant {
        window.show().map_err(window_controller::window_error)?;
        if presentation.is_some() && mode != Presentation::Orb {
            window
                .set_focus()
                .map_err(window_controller::window_error)?;
        }
    }
    Ok(())
}
#[tauri::command]
pub fn open_bookmark(app: tauri::AppHandle, state: State<AppState>, id: String) -> Result<()> {
    let snapshot = state.store.lock().unwrap().read()?;
    let data = modules::data(&snapshot, "bookmarks")?;
    let item = data
        .items
        .iter()
        .find(|i| i.id == id)
        .ok_or_else(|| invalid("网页不存在"))?;
    let url = modules::normalize_url(item.url.as_deref().unwrap_or(""))?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::new("OPEN_FAILED", format!("无法打开网页：{e}")))
}
#[tauri::command]
pub fn open_external(app: tauri::AppHandle, url: String) -> Result<()> {
    let url = modules::normalize_url(&url)?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::new("OPEN_FAILED", format!("无法打开网页：{e}")))
}
#[tauri::command]
pub async fn export_backup(app: tauri::AppHandle) -> Result<bool> {
    let snapshot = get_snapshot(app.state())?;
    let path = app
        .dialog()
        .file()
        .set_title("导出 Overstory 备份")
        .add_filter("JSON 备份", &["json"])
        .set_file_name(format!(
            "Overstory-{}.json",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        ))
        .blocking_save_file();
    let Some(path) = path else {
        return Ok(false);
    };
    let path = path.into_path().map_err(|e| invalid(&e.to_string()))?;
    crate::storage::atomic_write(
        &path,
        &serde_json::to_vec_pretty(&Backup::from_snapshot(&snapshot))?,
    )?;
    Ok(true)
}
#[tauri::command]
pub async fn import_backup(app: tauri::AppHandle, confirm: bool) -> Result<Value> {
    if !confirm {
        let path = app
            .dialog()
            .file()
            .set_title("选择 Overstory 备份")
            .add_filter("JSON 备份", &["json"])
            .blocking_pick_file();
        let Some(path) = path else {
            return Ok(Value::Null);
        };
        let path = path.into_path().map_err(|e| invalid(&e.to_string()))?;
        let backup = Backup::parse(&std::fs::read(path)?)?;
        let mut counts = serde_json::Map::new();
        for module in ["todos", "bookmarks", "notes"] {
            counts.insert(
                module.into(),
                serde_json::json!(backup.modules[module].data["items"]
                    .as_array()
                    .map(Vec::len)
                    .unwrap_or(0)),
            );
        }
        *app.state::<AppState>().backup.lock().unwrap() = Some(backup);
        Ok(Value::Object(counts))
    } else {
        let state = app.state::<AppState>();
        let backup = state
            .backup
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| invalid("请先选择备份文件"))?;
        let snapshot = backup.restore(&mut state.store.lock().unwrap())?;
        publish(&app, &snapshot);
        Ok(serde_json::to_value(snapshot)?)
    }
}
