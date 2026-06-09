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

        // Generate thumbnail in background
        if let Some((thumb_bytes, _, _)) = crate::engine::Engine::generate_thumbnail(path) {
            let thumb_path = state.thumb_dir.join(format!("{}.jpg", idx));
            let _ = std::fs::write(&thumb_path, &thumb_bytes);
        }

        // Extract and store colors
        let colors = crate::engine::Engine::extract_colors(path, 9);
        if !colors.is_empty() {
            if let Ok(json) = serde_json::to_string(&colors) {
                let _ = state.db.put(&format!("colors:{}", idx), json.as_bytes());
            }
        }

        Ok(item)
    }

    pub async fn get_thumbnail_path(state: &AppState, id: &str) -> Option<String> {
        let thumb_path = state.thumb_dir.join(format!("{}.jpg", id));
        if thumb_path.exists() {
            Some(thumb_path.to_string_lossy().to_string())
        } else {
            None
        }
    }

    pub async fn get_item_colors(state: &AppState, id: &str) -> Vec<String> {
        let key = format!("colors:{}", id);
        match state.db.get(&key) {
            Ok(Some(data)) => {
                serde_json::from_slice::<Vec<String>>(&data).unwrap_or_default()
            }
            _ => vec![],
        }
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
