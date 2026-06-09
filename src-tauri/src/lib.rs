mod api;
mod db;
mod engine;
mod models;
mod sdk_services;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;
use tauri::Manager;

pub struct AppState {
    pub db: Arc<db::MetadataDb>,
    pub cancelled: Arc<AtomicBool>,
    pub next_id: AtomicU64,
    pub thumb_dir: PathBuf,
}

fn get_app_dir(app: &tauri::App) -> PathBuf {
    if let Ok(dir) = app.path().app_data_dir() {
        return dir;
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join("Library/Application Support/com.huajing.desktop")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_dir = get_app_dir(app);
            let thumb_dir = app_dir.join("thumbs");
            std::fs::create_dir_all(&app_dir)?;
            std::fs::create_dir_all(&thumb_dir)?;
            let db = db::MetadataDb::new(app_dir.join("metadata.db"))?;
            
            let max_id = db.scan_prefix_max_numeric("item:").unwrap_or(0);
            let next_id = AtomicU64::new(max_id + 1);
            
            app.manage(Arc::new(AppState {
                db: Arc::new(db),
                cancelled: Arc::new(AtomicBool::new(false)),
                next_id,
                thumb_dir,
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api::items_cmds::query_items,
            api::items_cmds::import_files,
            api::items_cmds::cancel_import,
            api::items_cmds::delete_items,
            api::items_cmds::clear_all_items,
            api::items_cmds::get_thumbnail_path,
            api::items_cmds::get_item_colors,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
