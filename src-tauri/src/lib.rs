use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

pub mod commands;
pub mod db;
pub mod session;

use session::SessionManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("lulu.db");
            let database = db::init_database(&db_path)?;
            app.manage(database);
            app.manage(Arc::new(Mutex::new(SessionManager::new())));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::spawn_session,
            commands::list_sessions,
            commands::get_session,
            commands::kill_session,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let app = window.app_handle();
                if let Some(state) = app.try_state::<Arc<Mutex<SessionManager>>>() {
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    runtime.block_on(async {
                        let manager = state.lock().await;
                        manager.kill_all().await;
                    });
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
