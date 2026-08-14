//! Crate-wide lock for tests that mutate control-chat / panel-state env vars.
//!
//! `cargo test` runs modules in parallel in one process. Chat, harness, and
//! any other test that touches `CHEFBAR_CONTROL_AGENT`,
//! `CHEFBAR_CONTROL_PANE`, or `CHEFBAR_PANEL_STATE` must share this guard so
//! one test cannot restore another test's saved value.

use std::sync::{Mutex, MutexGuard};

static ENV_LOCK: Mutex<()> = Mutex::new(());

const KEYS: &[&str] = &[
    "CHEFBAR_CONTROL_AGENT",
    "CHEFBAR_CONTROL_PANE",
    "CHEFBAR_PANEL_STATE",
];

pub(crate) struct EnvGuard {
    _lock: MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    pub(crate) fn acquire() -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let saved = KEYS
            .iter()
            .map(|k| (*k, std::env::var(k).ok()))
            .collect::<Vec<_>>();
        for k in KEYS {
            std::env::remove_var(k);
        }
        Self { _lock: lock, saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in &self.saved {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    }
}
