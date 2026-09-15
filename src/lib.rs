//! ChefBar — Rust remake van de ChefGroep-assistent.
//!
//! Eén poll-actor (state.rs) voedt een gedeeld snapshot; tray (ksni), panel
//! (gtk-rs) en command-bar delen dat beeld. Acties zijn declaratieve data
//! (actions.rs), uitvoer loopt door één executor met policy-clients.

pub mod actions;
pub mod aliases;
pub mod auth;
pub mod brain;
pub mod chat;
pub mod config;
pub mod css;
pub mod doctor;
pub mod frecency;
pub mod harness;
pub mod http;
pub(crate) mod icons;
pub mod ipc;
pub mod log;
pub mod models;
pub mod motion;
pub mod mutes;
pub mod notify;
pub mod ops_cli;
pub mod palette;
pub mod panel;
pub mod panel_state;
pub mod policy;
pub mod quiet;
pub mod sessions;
pub mod state;
#[cfg(test)]
pub(crate) mod test_env;
pub mod tray;
pub mod vault_bridge;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const RAW_BUILD_SHA: Option<&str> = option_env!("CHEFBAR_BUILD_SHA");

/// Injected by release CI. Missing or empty is `unknown`, never a guessed SHA.
pub const BUILD_SHA: &str = match RAW_BUILD_SHA {
    Some("") | None => "unknown",
    Some(sha) => sha,
};

/// Visible runtime identity: Cargo version plus build SHA.
pub fn identity() -> String {
    format!("{VERSION} ({BUILD_SHA})")
}

/// Thuis-map, één centrale plek (port van HOME in de Python-app).
pub fn home_dir() -> std::path::PathBuf {
    dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
}

#[cfg(test)]
mod identity_tests {
    #[test]
    fn identity_uses_cargo_version_and_never_invents_a_sha() {
        let line = crate::identity();
        assert!(
            line.starts_with(crate::VERSION),
            "identity {line:?} must start with {}",
            crate::VERSION
        );
        assert!(
            line.contains(crate::BUILD_SHA),
            "identity {line:?} must include BUILD_SHA"
        );
        assert!(
            !crate::BUILD_SHA.is_empty(),
            "missing build SHA must be the literal unknown, not empty"
        );
    }
}
