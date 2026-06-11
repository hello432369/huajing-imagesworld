use std::sync::Arc;
use anyhow::Result;
use std::path::Path;

use crate::AppState;
use crate::models::*;

pub struct SdkBridge;

impl SdkBridge {
    pub async fn query_items(state: &AppState, query: &ItemQuery) -> Result<ItemQueryResult> {
        let offset = query.offset.unwrap_or(0) as usize;
        let limit = query.limit.unwrap_or(60).min(200) as usize;

        let mut all: Vec<Item> = Vec::new();
        for result in state.db.scan_prefix("item:") {
            let (_, value) = result?;
            if let Ok(item) = serde_json::from_slice::<Item>(&value) {
                all.push(item);
            }
        }
        all.sort_by(|a, b| {
            let ia: u64 = a.id.parse().unwrap_or(0);
            let ib: u64 = b.id.parse().unwrap_or(0);
            ia.cmp(&ib)
        });

        let total = all.len() as u64;
        let items: Vec<Item> = all.into_iter().skip(offset).take(limit).collect();
        Ok(ItemQueryResult { items, total, offset: offset as u64, limit: limit as u64 })
    }

    pub fn import_single_file_fast(state: &Arc<AppState>, path_str: String) -> Result<Item> {
        use std::sync::atomic::Ordering;
        let path = Path::new(&path_str);
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len();
        let (width, height) = crate::engine::Engine::extract_image_size(path)?;

        let idx = state.next_id.fetch_add(1, Ordering::SeqCst);
        let now = chrono::Utc::now().to_rfc3339();
        let mime = crate::engine::Engine::detect_mime_type(path);

        let item = Item {
            id: idx.to_string(),
            file_name,
            file_path: path.to_string_lossy().to_string(),
            file_size,
            width, height,
            mime_type: mime,
            content_hash: None,
            imported_at: now,
        };

        state.db.put(&format!("item:{}", idx), &serde_json::to_vec(&item)?)?;

        Ok(item)
    }

    /// Generate thumbnails and colors for items that don't have them yet.
    /// Returns (generated_count, total_needed).
    pub fn generate_missing_thumbnails(state: &AppState) -> (usize, usize) {
        let mut items_to_process: Vec<String> = Vec::new();
        for result in state.db.scan_prefix("item:") {
            if let Ok((key, value)) = result {
                if let Ok(item) = serde_json::from_slice::<Item>(&value) {
                    let id = item.id.clone();
                    let thumb_path = state.thumb_dir.join(format!("{}.jpg", id));
                    if !thumb_path.exists() {
                        items_to_process.push(id);
                    }
                }
            }
        }

        let total = items_to_process.len();
        let mut generated = 0usize;

        for id in &items_to_process {
            let item_key = format!("item:{}", id);
            if let Ok(Some(data)) = state.db.get(&item_key) {
                if let Ok(item) = serde_json::from_slice::<Item>(&data) {
                    let src = Path::new(&item.file_path);
                    if src.exists() {
                        // Generate thumbnail
                        if let Some((thumb_bytes, _, _, ext)) = crate::engine::Engine::generate_thumbnail(src) {
                            let thumb_path = state.thumb_dir.join(format!("{}.{}", id, ext));
                            let _ = std::fs::write(&thumb_path, &thumb_bytes);
                        }
                        // Extract colors
                        let colors = crate::engine::Engine::extract_colors(src, 9);
                        if !colors.is_empty() {
                            if let Ok(json) = serde_json::to_string(&colors) {
                                let _ = state.db.put(&format!("colors:{}", id), json.as_bytes());
                            }
                        }
                    }
                }
            }
            generated += 1;
        }

        (generated, total)
    }

    pub async fn get_thumbnail_path(state: &AppState, id: &str) -> Option<String> {
        for ext in &["jpg", "png"] {
            let thumb_path = state.thumb_dir.join(format!("{}.{}", id, ext));
            if thumb_path.exists() {
                return Some(thumb_path.to_string_lossy().to_string());
            }
        }
        None
    }

    pub async fn get_item_colors(state: &AppState, id: &str) -> Vec<String> {
        let key = format!("colors:{}", id);
        // Return cached if available
        if let Ok(Some(data)) = state.db.get(&key) {
            if let Ok(colors) = serde_json::from_slice::<Vec<String>>(&data) {
                if !colors.is_empty() {
                    return colors;
                }
            }
        }
        // Extract on demand from original file
        let item_key = format!("item:{}", id);
        if let Ok(Some(item_data)) = state.db.get(&item_key) {
            if let Ok(item) = serde_json::from_slice::<crate::models::Item>(&item_data) {
                let src = std::path::Path::new(&item.file_path);
                if src.exists() {
                    let colors = crate::engine::Engine::extract_colors(src, 9);
                    if !colors.is_empty() {
                        if let Ok(json) = serde_json::to_string(&colors) {
                            let _ = state.db.put(&key, json.as_bytes());
                        }
                        return colors;
                    }
                }
            }
        }
        vec![]
    }

    pub async fn delete_items(state: &AppState, ids: &[String]) -> Result<()> {
        for id_str in ids {
            state.db.delete(&format!("item:{}", id_str))?;
            state.db.delete(&format!("colors:{}", id_str))?;
            let thumb_path = state.thumb_dir.join(format!("{}.jpg", id_str));
            let _ = std::fs::remove_file(&thumb_path);
        }
        Ok(())
    }

    pub async fn export_all_items(state: &AppState, dest_dir: &Path) -> Result<usize> {
        let mut count: usize = 0;
        for result in state.db.scan_prefix("item:") {
            let (key, value) = result?;
            let mut item: Item = serde_json::from_slice(&value)?;

            let src = Path::new(&item.file_path);
            if !src.exists() {
                continue;
            }

            let file_name = src.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
            let dest_path = resolve_unique_path(dest_dir, file_name);

            std::fs::rename(src, &dest_path)
                .or_else(|_| {
                    std::fs::copy(src, &dest_path)?;
                    std::fs::remove_file(src)?;
                    Ok::<_, anyhow::Error>(())
                })?;

            item.file_path = dest_path.to_string_lossy().to_string();
            state.db.put(&key, &serde_json::to_vec(&item)?)?;

            count += 1;
        }
        Ok(count)
    }

    pub async fn clear_all_items(state: &AppState) -> Result<()> {
        let mut all_keys: Vec<String> = Vec::new();
        for result in state.db.scan_prefix("item:") {
            if let Ok((key, _)) = result { all_keys.push(key); }
        }
        for result in state.db.scan_prefix("colors:") {
            if let Ok((key, _)) = result { all_keys.push(key); }
        }
        for key in &all_keys {
            let _ = state.db.delete(key);
        }
        if state.thumb_dir.exists() {
            let _ = std::fs::remove_dir_all(&state.thumb_dir);
            let _ = std::fs::create_dir_all(&state.thumb_dir);
        }
        Ok(())
    }
}

fn resolve_unique_path(dir: &std::path::Path, filename: &str) -> std::path::PathBuf {
    let candidate = dir.join(filename);
    if !candidate.exists() {
        return candidate;
    }
    let stem = std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();
    for i in 1..9999 {
        let new_name = format!("{}_{}{}", stem, i, ext);
        let candidate = dir.join(&new_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    dir.join(format!("{}_{}{}", stem, ts, ext))
}
