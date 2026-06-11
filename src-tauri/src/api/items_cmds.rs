use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Instant;
use tauri::State;
use tauri::Emitter;
use walkdir::WalkDir;

use crate::AppState;
use crate::models::*;
use crate::sdk_services::SdkBridge;

fn is_image(path: &std::path::Path) -> bool {
    if crate::engine::Engine::is_image_by_magic(path) {
        return true;
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let e = ext.to_lowercase();
        matches!(e.as_str(),
            "jpg" | "jpeg" | "jfif" | "jpe"
            | "png" | "gif" | "webp" | "bmp" | "dib"
            | "svg" | "tiff" | "tif"
            | "heic" | "heif" | "hif"
            | "avif" | "avifs"
            | "ico" | "cur"
        )
    } else {
        false
    }
}

#[tauri::command]
pub async fn query_items(
    state: State<'_, Arc<AppState>>,
    query: ItemQuery,
) -> Result<ItemQueryResult, String> {
    SdkBridge::query_items(&*state, &query).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_files(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    paths: Vec<String>,
) -> Result<(), String> {
    state.cancelled.store(false, Ordering::SeqCst);
    let app = app_handle.clone();
    let cancelled = state.cancelled.clone();

    // ── 1. Walk dirs, collect paths ──
    let mut skipped_count = 0u64;
    let mut all_paths: Vec<String> = Vec::new();
    for p in &paths {
        let path = std::path::Path::new(&p);
        if path.is_dir() {
            for entry in WalkDir::new(path).follow_links(false).into_iter() {
                match entry {
                    Ok(e) => {
                        if e.file_type().is_file() && is_image(e.path()) {
                            all_paths.push(e.path().to_string_lossy().to_string());
                        }
                    }
                    Err(_) => { skipped_count += 1; }
                }
            }
        } else if path.is_file() && is_image(path) {
            all_paths.push(p.clone());
        }
    }

    let total = all_paths.len();
    if total == 0 {
        app.emit("import-progress", ImportProgress {
            items: vec![], index: 0, total: 0, skipped: skipped_count, cancelled: false,
        }).ok();
        return Ok(());
    }

    let (tx, rx) = mpsc::channel::<Option<Item>>();

    // ── 2. Workers: tx is MOVED into spawn_blocking ──
    // The Sender stays alive INSIDE the blocking thread until ALL processing is done.
    // Only when the spawn_blocking closure finishes does the Sender drop → channel closes.
    let state_for_thread = state.inner().clone();
    let cancelled_clone = cancelled.clone();
    tauri::async_runtime::spawn_blocking(move || {
        use rayon::prelude::*;

        // Wrap tx in Arc<Mutex> so rayon workers can share it
        let tx = Arc::new(Mutex::new(tx));
        let final_tx = tx.clone();

        all_paths.par_iter().for_each(|path_str| {
            if cancelled_clone.load(Ordering::SeqCst) {
                return;
            }
            let result = SdkBridge::import_single_file_fast(&state_for_thread, path_str.clone());
            // Use unwrap_or_else to recover from poisoned mutex
            // (if another worker panicked, we don't want cascading crash)
            let guard = tx.lock().unwrap_or_else(|e| e.into_inner());
            let _ = guard.send(result.ok());
        });

        // final_tx (and the enclosed Sender) is dropped here → channel disconnects
        drop(final_tx);
        tracing::info!("导入处理完成: {} 个文件", total);
    });

    // ── 3. Collector: non-blocking recv with 100ms timeout ──
    let cancelled_clone2 = cancelled.clone();
    let state_for_thumb = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        let mut batch: Vec<Item> = Vec::new();
        let mut done: usize = 0;
        let mut successes: usize = 0;
        let mut last_emit = Instant::now();

        loop {
            if cancelled_clone2.load(Ordering::SeqCst) {
                app.emit("import-progress", ImportProgress {
                    items: vec![], index: done, total, skipped: skipped_count, cancelled: true,
                }).ok();
                return;
            }

            // 100ms timeout → non-blocking so tokio can dispatch emit() events
            match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(Some(item)) => {
                    batch.push(item);
                    successes += 1;
                    done += 1;
                }
                Ok(None) => {
                    done += 1;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Nothing received, flush batch if needed
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // All senders dropped → processing fully complete
                    break;
                }
            }

            // Flush batch every 100ms or 50 items
            let elapsed = last_emit.elapsed();
            if (elapsed >= std::time::Duration::from_millis(100) || batch.len() >= 50) && !batch.is_empty() {
                let send_batch = std::mem::take(&mut batch);
                app.emit("import-progress", ImportProgress {
                    items: send_batch, index: successes, total, skipped: skipped_count, cancelled: false,
                }).ok();
                last_emit = Instant::now();
            }
        }

        // Flush remaining
        if !batch.is_empty() {
            app.emit("import-progress", ImportProgress {
                items: std::mem::take(&mut batch), index: successes, total, skipped: skipped_count, cancelled: false,
            }).ok();
        }

        // Done
        app.emit("import-progress", ImportProgress {
            items: vec![], index: successes, total: successes, skipped: skipped_count, cancelled: false,
        }).ok();

        // Silently generate missing thumbnails + colors in background (no progress events)
        let bg_state = state_for_thumb;
        tauri::async_runtime::spawn_blocking(move || {
            let mut need_work: Vec<String> = Vec::new();
            for result in bg_state.db.scan_prefix("item:") {
                if let Ok((_, value)) = result {
                    if let Ok(item) = serde_json::from_slice::<crate::models::Item>(&value) {
                        let id = item.id;
                        let thumb_path = bg_state.thumb_dir.join(format!("{}.jpg", &id));
                        if !thumb_path.exists() {
                            need_work.push(id);
                        }
                    }
                }
            }
            if need_work.is_empty() { return; }
            use rayon::prelude::*;
            need_work.par_iter().for_each(|id| {
                let item_key = format!("item:{}", id);
                if let Ok(Some(data)) = bg_state.db.get(&item_key) {
                    if let Ok(item) = serde_json::from_slice::<crate::models::Item>(&data) {
                        let src = std::path::Path::new(&item.file_path);
                        if !src.exists() { return; }
                        if let Some((thumb_bytes, _, _, ext)) = crate::engine::Engine::generate_thumbnail(src) {
                            let _ = std::fs::write(&bg_state.thumb_dir.join(format!("{}.{}", id, ext)), &thumb_bytes);
                        }
                        let colors = crate::engine::Engine::extract_colors(src, 9);
                        if !colors.is_empty() {
                            if let Ok(json) = serde_json::to_string(&colors) {
                                let _ = bg_state.db.put(&format!("colors:{}", id), json.as_bytes());
                            }
                        }
                    }
                }
            });
        });
    });

    Ok(())
}

#[tauri::command]
pub async fn cancel_import(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.cancelled.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn delete_items(
    state: State<'_, Arc<AppState>>,
    ids: Vec<String>,
) -> Result<(), String> {
    SdkBridge::delete_items(&*state, &ids).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_all_items(
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    SdkBridge::clear_all_items(&*state).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_items(
    state: State<'_, Arc<AppState>>,
    dest_path: String,
) -> Result<usize, String> {
    let dest = std::path::Path::new(&dest_path);
    if !dest.is_dir() {
        std::fs::create_dir_all(dest).map_err(|e| format!("无法创建目录: {}", e))?;
    }
    SdkBridge::export_all_items(&*state, dest)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_thumbnail_path(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Option<String>, String> {
    Ok(SdkBridge::get_thumbnail_path(&*state, &id).await)
}

#[tauri::command]
pub async fn get_item_colors(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Vec<String>, String> {
    Ok(SdkBridge::get_item_colors(&*state, &id).await)
}
