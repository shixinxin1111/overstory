use crate::commands::{publish, AppState};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc;
use tauri::{Emitter, Manager};

pub fn watch(app: &tauri::AppHandle) -> std::io::Result<RecommendedWatcher> {
    let dir = app.state::<AppState>().store.lock().unwrap().dir.clone();
    let (sender, receiver) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event {
            if event
                .paths
                .iter()
                .any(|p| p.file_name().is_some_and(|n| n == "state.json"))
            {
                let _ = sender.send(());
            }
        }
    })
    .map_err(std::io::Error::other)?;
    watcher
        .watch(&dir, RecursiveMode::NonRecursive)
        .map_err(std::io::Error::other)?;
    let app = app.clone();
    std::thread::spawn(move || {
        while receiver.recv().is_ok() {
            while receiver.try_recv().is_ok() {}
            let state = app.state::<AppState>();
            let mut store = state.store.lock().unwrap();
            match store.refresh() {
                Ok(true) => publish(&app, &store.snapshot),
                Ok(false) => {}
                Err(error) => {
                    let _ = app.emit("app:error", error);
                }
            }
        }
    });
    Ok(watcher)
}
