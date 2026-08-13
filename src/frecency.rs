//! Frecency store — capped 64, TTL 30d, atomair JSON.
//!
//! Pure lokaal: `~/.local/share/chefbar/frecency.json`. Nooit naar server.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

const CAP: usize = 64;
const TTL_SECS: i64 = 30 * 24 * 3600;

#[derive(Debug, Clone)]
struct Cache {
    path: PathBuf,
    entries: Vec<FrecencyEntry>,
}

static CACHE: OnceLock<RwLock<Option<Cache>>> = OnceLock::new();

fn cache() -> &'static RwLock<Option<Cache>> {
    CACHE.get_or_init(|| RwLock::new(None))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrecencyEntry {
    pub id: String,
    pub last_opened: String,
    pub open_count: u32,
}

fn frecency_path() -> PathBuf {
    if let Ok(env) = std::env::var("CHEFBAR_FRECENCY_PATH") {
        if !env.trim().is_empty() {
            return PathBuf::from(env);
        }
    }
    crate::home_dir().join(".local/share/chefbar/frecency.json")
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn parse_secs(value: &str) -> Option<i64> {
    // probeer eerst als integer secs
    if let Ok(n) = value.trim().parse::<i64>() {
        return Some(n);
    }
    // probeer ISO8601-achtig: neem laatste : of T suffix
    // fallback: 0
    None
}

fn is_expired(entry: &FrecencyEntry, now: i64) -> bool {
    let Some(ts) = parse_secs(&entry.last_opened) else {
        // als niet parsebaar, behandel als oud maar niet direct expired
        // we kijken naar TTL alleen als we secs kunnen parsen; anders behoud
        return false;
    };
    now - ts > TTL_SECS
}

pub fn load() -> Vec<FrecencyEntry> {
    let path = frecency_path();
    if let Ok(guard) = cache().read() {
        if let Some(cached) = guard.as_ref().filter(|cached| cached.path == path) {
            return cached.entries.clone();
        }
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            if let Ok(mut guard) = cache().write() {
                *guard = Some(Cache {
                    path,
                    entries: Vec::new(),
                });
            }
            return Vec::new();
        }
    };
    let entries: Vec<FrecencyEntry> = serde_json::from_str(&text).unwrap_or_default();
    let now = now_secs();
    let mut filtered: Vec<FrecencyEntry> = entries
        .into_iter()
        .filter(|e| !is_expired(e, now))
        .collect();
    filtered.truncate(CAP);
    if let Ok(mut guard) = cache().write() {
        *guard = Some(Cache {
            path,
            entries: filtered.clone(),
        });
    }
    filtered
}

fn save(entries: &[FrecencyEntry]) {
    let path = frecency_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(entries).unwrap_or_else(|_| "[]".into());
    let tmp = path.with_extension("json.tmp");
    if fs::write(&tmp, json).is_ok() && fs::rename(&tmp, &path).is_ok() {
        if let Ok(mut guard) = cache().write() {
            *guard = Some(Cache {
                path,
                entries: entries.to_vec(),
            });
        }
    }
}

pub fn record(id: &str) {
    let mut entries = load();
    let now = now_secs().to_string();
    if let Some(existing) = entries.iter_mut().find(|e| e.id == id) {
        existing.last_opened = now;
        existing.open_count = existing.open_count.saturating_add(1);
    } else {
        entries.push(FrecencyEntry {
            id: id.to_string(),
            last_opened: now,
            open_count: 1,
        });
    }
    // sorteer: meest recent eerst, dan open_count desc, dan id
    entries.sort_by(|a, b| {
        b.last_opened
            .cmp(&a.last_opened)
            .then(b.open_count.cmp(&a.open_count))
            .then(a.id.cmp(&b.id))
    });
    entries.truncate(CAP);
    save(&entries);
}

pub fn top(n: usize) -> Vec<FrecencyEntry> {
    let mut entries = load();
    entries.truncate(n);
    entries
}

pub fn prune_expired() -> usize {
    let before = load();
    let now = now_secs();
    let pruned: Vec<FrecencyEntry> = before.into_iter().filter(|e| !is_expired(e, now)).collect();
    let removed = {
        let raw_before: Vec<FrecencyEntry> = {
            let path = frecency_path();
            fs::read_to_string(&path)
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default()
        };
        raw_before.len().saturating_sub(pruned.len())
    };
    // save pruned (load already filtered, but ensure file reflects it)
    let current = load();
    save(&current);
    let _ = pruned;
    removed
}

// helper voor tests: directe map-vorm
pub fn as_map() -> HashMap<String, FrecencyEntry> {
    load().into_iter().map(|e| (e.id.clone(), e)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "chefbar-frecency-test-{}-{}-{}.json",
            name,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn cap_is_enforced() {
        let _g = TEST_LOCK.lock().unwrap();
        let path = temp_path("cap");
        std::env::set_var("CHEFBAR_FRECENCY_PATH", &path);
        let _ = fs::remove_file(&path);
        for i in 0..70 {
            record(&format!("id-{i}"));
        }
        let entries = load();
        assert!(entries.len() <= 64, "capped 64, got {}", entries.len());
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.tmp"));
        std::env::remove_var("CHEFBAR_FRECENCY_PATH");
    }

    #[test]
    fn ttl_eviction() {
        let _g = TEST_LOCK.lock().unwrap();
        let path = temp_path("ttl");
        std::env::set_var("CHEFBAR_FRECENCY_PATH", &path);
        let old_secs = now_secs() - TTL_SECS - 1000;
        let fresh_secs = now_secs();
        let data = vec![
            FrecencyEntry {
                id: "old".into(),
                last_opened: old_secs.to_string(),
                open_count: 1,
            },
            FrecencyEntry {
                id: "fresh".into(),
                last_opened: fresh_secs.to_string(),
                open_count: 1,
            },
        ];
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&path, serde_json::to_string(&data).unwrap()).unwrap();
        let loaded = load();
        assert!(loaded.iter().any(|e| e.id == "fresh"));
        assert!(
            !loaded.iter().any(|e| e.id == "old"),
            "old should be evicted"
        );
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.tmp"));
        std::env::remove_var("CHEFBAR_FRECENCY_PATH");
    }

    #[test]
    fn tolerant_load_on_corrupt() {
        let _g = TEST_LOCK.lock().unwrap();
        let path = temp_path("corrupt");
        std::env::set_var("CHEFBAR_FRECENCY_PATH", &path);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&path, "not json [[[ ").unwrap();
        let loaded = load();
        assert!(loaded.is_empty());
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.tmp"));
        std::env::remove_var("CHEFBAR_FRECENCY_PATH");
    }

    #[test]
    fn record_bumps_count() {
        let _g = TEST_LOCK.lock().unwrap();
        let path = temp_path("bump");
        std::env::set_var("CHEFBAR_FRECENCY_PATH", &path);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.tmp"));
        record("same-id");
        record("same-id");
        let entries = load();
        let entry = entries.iter().find(|e| e.id == "same-id").unwrap();
        assert_eq!(entry.open_count, 2);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.tmp"));
        std::env::remove_var("CHEFBAR_FRECENCY_PATH");
    }
}
