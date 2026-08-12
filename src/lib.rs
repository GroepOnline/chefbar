//! ChefBar — Rust remake van de ChefGroep-assistent.
//!
//! Eén poll-actor (state.rs) voedt een gedeeld snapshot; tray (ksni), panel
//! (gtk-rs) en command-bar delen dat beeld. Acties zijn declaratieve data
//! (actions.rs), uitvoer loopt door één executor met policy-clients.

pub mod actions;
pub mod aliases;
pub mod auth;
pub mod config;
pub mod css;
pub mod doctor;
pub mod harness;
pub mod http;
pub mod ipc;
pub mod models;
pub mod motion;
pub mod notify;
pub mod ops_cli;
pub mod palette;
pub mod panel;
pub mod panel_state;
pub mod policy;
pub mod sessions;
pub mod state;
pub mod tray;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Thuis-map, één centrale plek (port van HOME in de Python-app).
pub fn home_dir() -> std::path::PathBuf {
    dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
}
