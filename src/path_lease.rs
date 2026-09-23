
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

pub const DEFAULT_LEASE_TTL_MS: u64 = 5 * 60 * 1000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    pub path: String,
    pub holder_id: String,
    pub acquired_at: u64,
    pub ttl_ms: u64,
}

impl Lease {
    pub fn is_expired(&self, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.acquired_at) >= self.ttl_ms
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseConflict {
    pub path: String,
    pub existing_holder: String,
    pub remaining_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathLeaseRegistry {
    pub leases: HashMap<String, Lease>,
}

impl PathLeaseRegistry {
    pub fn new() -> Self {
        Self {
            leases: HashMap::new(),
        }
    }

    pub fn normalize_path(path: &str) -> String {
        let p = Path::new(path);
        if let Ok(canon) = p.canonicalize() {
            canon.to_string_lossy().to_string()
        } else {
            p.to_string_lossy().to_string()
        }
    }

    pub fn acquire(
        &mut self,
        path: &str,
        holder_id: &str,
        ttl_ms: u64,
    ) -> Result<(), LeaseConflict> {
        let norm = Self::normalize_path(path);
        let now = current_time_ms();
        self.clean_expired_at(now);

        if let Some(existing) = self.leases.get(&norm) {
            if existing.holder_id != holder_id && !existing.is_expired(now) {
                let remaining = existing.ttl_ms.saturating_sub(now.saturating_sub(existing.acquired_at));
                return Err(LeaseConflict {
                    path: norm,
                    existing_holder: existing.holder_id.clone(),
                    remaining_ms: remaining,
                });
            }
        }

        self.leases.insert(
            norm.clone(),
            Lease {
                path: norm,
                holder_id: holder_id.to_string(),
                acquired_at: now,
                ttl_ms: if ttl_ms == 0 { DEFAULT_LEASE_TTL_MS } else { ttl_ms },
            },
        );

        Ok(())
    }

    pub fn release(&mut self, path: &str, holder_id: &str) -> bool {
        let norm = Self::normalize_path(path);
        if let Some(existing) = self.leases.get(&norm) {
            if existing.holder_id == holder_id {
                self.leases.remove(&norm);
                return true;
            }
        }
        false
    }

    pub fn release_all(&mut self, holder_id: &str) -> usize {
        let before = self.leases.len();
        self.leases.retain(|_, lease| lease.holder_id != holder_id);
        before.saturating_sub(self.leases.len())
    }

    pub fn get_lease(&self, path: &str) -> Option<Lease> {
        let norm = Self::normalize_path(path);
        let now = current_time_ms();
        if let Some(lease) = self.leases.get(&norm) {
            if !lease.is_expired(now) {
                return Some(lease.clone());
            }
        }
        None
    }

    pub fn clean_expired(&mut self) {
        let now = current_time_ms();
        self.clean_expired_at(now);
    }

    fn clean_expired_at(&mut self, now_ms: u64) {
        self.leases.retain(|_, lease| !lease.is_expired(now_ms));
    }

    pub fn load_from_file(path: &Path) -> Self {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(mut reg) = serde_json::from_str::<PathLeaseRegistry>(&data) {
                reg.clean_expired();
                return reg;
            }
        }
        Self::new()
    }

    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        crate::ipc::atomic_write(path, &json)
    }
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_and_release_single_lease() {
        let mut reg = PathLeaseRegistry::new();
        let path = "src/main.rs";

        assert!(reg.acquire(path, "worker-1", 10_000).is_ok());
        assert!(reg.get_lease(path).is_some());
        assert_eq!(reg.get_lease(path).unwrap().holder_id, "worker-1");

        let err = reg.acquire(path, "worker-2", 10_000).unwrap_err();
        assert_eq!(err.existing_holder, "worker-1");

        assert!(reg.acquire(path, "worker-1", 20_000).is_ok());

        assert!(!reg.release(path, "worker-2"));
        assert!(reg.get_lease(path).is_some());

        assert!(reg.release(path, "worker-1"));
        assert!(reg.get_lease(path).is_none());

        assert!(reg.acquire(path, "worker-2", 10_000).is_ok());
    }

    #[test]
    fn release_all_for_holder() {
        let mut reg = PathLeaseRegistry::new();
        reg.acquire("src/pane.rs", "worker-1", 10_000).unwrap();
        reg.acquire("src/tab_bar.rs", "worker-1", 10_000).unwrap();
        reg.acquire("src/main.rs", "worker-2", 10_000).unwrap();

        assert_eq!(reg.release_all("worker-1"), 2);
        assert!(reg.get_lease("src/pane.rs").is_none());
        assert!(reg.get_lease("src/tab_bar.rs").is_none());
        assert!(reg.get_lease("src/main.rs").is_some());
    }

    #[test]
    fn expired_leases_are_cleaned() {
        let mut reg = PathLeaseRegistry::new();
        let now = 10_000u64;

        reg.leases.insert(
            "src/old.rs".into(),
            Lease {
                path: "src/old.rs".into(),
                holder_id: "dead-worker".into(),
                acquired_at: now - 5000,
                ttl_ms: 2000,
            },
        );

        reg.clean_expired_at(now);
        assert!(reg.leases.is_empty());
    }
}

