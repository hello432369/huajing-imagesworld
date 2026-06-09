use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_size: u64,
    pub width: u32,
    pub height: u32,
    pub mime_type: String,
    pub content_hash: Option<String>,
    pub imported_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemQueryResult {
    pub items: Vec<Item>,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub items: Vec<Item>,
    pub index: usize,
    pub total: usize,
    pub skipped: u64,
    pub cancelled: bool,
}
