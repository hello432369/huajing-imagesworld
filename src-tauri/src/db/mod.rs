use std::path::PathBuf;
use anyhow::Result;
use sled::Db;

pub struct MetadataDb {
    db: Db,
}

impl MetadataDb {
    pub fn new(path: PathBuf) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn put(&self, key: &str, value: &[u8]) -> Result<()> {
        self.db.insert(key.as_bytes(), value)?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.db.get(key.as_bytes())?.map(|v| v.to_vec()))
    }

    pub fn delete(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.db.remove(key.as_bytes())?.map(|v| v.to_vec()))
    }

    pub fn scan_prefix(&self, prefix: &str) -> impl Iterator<Item = Result<(String, Vec<u8>)>> {
        self.db
            .scan_prefix(prefix.as_bytes())
            .map(|r| r.map(|(k, v)| (String::from_utf8(k.to_vec()).unwrap_or_default(), v.to_vec())).map_err(anyhow::Error::from))
    }

    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }

    /// Scan all keys with the given prefix, find the maximum numeric suffix.
    /// e.g. for item:0, item:1, ..., item:99 — returns 99.
    pub fn scan_prefix_max_numeric(&self, prefix: &str) -> Option<u64> {
        let mut max: Option<u64> = None;
        for result in self.db.scan_prefix(prefix.as_bytes()) {
            if let Ok((key, _)) = result {
                let key_str = String::from_utf8(key.to_vec()).unwrap_or_default();
                if let Some(num_str) = key_str.strip_prefix(prefix) {
                    if let Ok(n) = num_str.parse::<u64>() {
                        if max.map_or(true, |m| n > m) {
                            max = Some(n);
                        }
                    }
                }
            }
        }
        max
    }
}
