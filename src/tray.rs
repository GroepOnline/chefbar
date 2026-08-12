//! Systeemtray (ksni) — pure Rust, geen Ayatana/AppIndicator dependency.
//!
//! Tray draait in zijn eigen thread en stuurt alleen UI-commando's door een
//! mpsc-kanaal die de glib-mainloop om de zoveel tijd leegt (idle dispatch).

use crate::models::Snapshot;
use gtk::glib::ControlFlow;
use ksni::menu::StandardItem;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};

/// UI-commando's van tray/ipc naar de GTK-thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    TogglePanel,
    ShowPanel,
    Refresh,
    Doctor,
    Quit,
    /// Open een URL (Thuis/Ploeg/desktop/ops) via de executor.
    OpenUrl(String),
    /// Focus een agent-werkstroom (terminal-id uit de event-line).
    FocusAgent(String),
    /// Account wisselen (zelfde payload als de panel-actie).
    SwitchAccount {
        account_id: String,
        source: String,
        driver: Option<String>,
    },
    /// Notificaties pauzeren via joep-notify (1u default).
    PauseNotifications,
    /// Meelopen vanaf login aan/uit (autostart-desktop-bestand).
    ToggleAutostart,
    /// Desktop webtop starten/stoppen via de executor.
    DesktopAction(String),
    /// Forceer de tray-glyph-state (testhook: stil/bezig/hulp/fout/offline)
    /// voor live verificatie op een echt GNOME-panel (brief W3).
    ForceState(String),
}

/// Glib-idle-bridge: leegt het commando-kanaal op de UI-thread.
pub fn start_command_bridge(
    rx: std::sync::mpsc::Receiver<UiCommand>,
    dispatcher: Arc<dyn Fn(UiCommand)>,
) {
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(60), move || {
        while let Ok(cmd) = rx.try_recv() {
            dispatcher(cmd);
        }
        ControlFlow::Continue
    });
}

pub struct ChefTray {
    shared: Arc<RwLock<Snapshot>>,
    tx: Sender<UiCommand>,
    icon: ksni::Icon,
    /// Laatst gezonden statuslijn, geüpdatet via Handle::update.
    status_line: Mutex<String>,
    /// Door `--ipc state <x>` geforceerde glyph (testhook, W3). None = live.
    forced_state: Mutex<Option<(String, std::time::Instant)>>,
}

impl ChefTray {
    pub fn new(shared: Arc<RwLock<Snapshot>>, tx: Sender<UiCommand>) -> Self {
        let (state, line) = shared
            .read()
            .map(|s| s.tray_state())
            .unwrap_or_else(|_| ("stil".into(), "ChefGroep".into()));
        Self {
            shared,
            tx,
            icon: tray_icon_for(&state),
            status_line: Mutex::new(line),
            forced_state: Mutex::new(None),
        }
    }

    fn send(&self, cmd: UiCommand) {
        let _ = self.tx.send(cmd);
    }
}

/// Forceer de tray-glyph voor live verificatie (10s, daarna live-status weer).
pub fn force_state(state: &str) {
    let Some(slot) = TRAY_HANDLE.get() else {
        return;
    };
    let guard = slot.lock().unwrap();
    let Some(handle) = guard.as_ref() else {
        return;
    };
    let state = state.to_string();
    handle.update(move |tray| {
        *tray.forced_state.lock().unwrap() = Some((state.clone(), std::time::Instant::now()));
        let line = format!("ChefGroep · test-glyph [{state}]");
        tray.icon = tray_icon_for(&state);
        *tray.status_line.lock().unwrap() = line;
    });
}

/// Update-handle die de poll-actor elke cyclus laat bijwerken (tooltip + icon).
static TRAY_HANDLE: std::sync::OnceLock<Mutex<Option<ksni::Handle<ChefTray>>>> =
    std::sync::OnceLock::new();

pub fn register_handle(handle: ksni::Handle<ChefTray>) {
    let slot = TRAY_HANDLE.get_or_init(|| Mutex::new(None));
    *slot.lock().unwrap() = Some(handle);
}

/// Tel verse meldingen voor de badge in de tray-statuslijn.
fn inbox_count(shared: &Arc<RwLock<Snapshot>>) -> usize {
    shared
        .read()
        .map(|s| {
            s.suggestions
                .iter()
                .filter(|sg| sg.fresh(crate::models::SUGGESTION_TTL_SECONDS))
                .count()
        })
        .unwrap_or(0)
}

/// Pauseer niet-critische notificaties 1 uur via joep-notify (brief: pauzeren
/// verloopt vanzelf; helper dropt non-critical, geen crash zonder daemon).
pub fn pause_notifications() {
    let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string());
    if let Some(home) = home {
        let _ = std::process::Command::new(format!("{home}/.local/bin/joep-notify"))
            .args(["pause", "1h"])
            .spawn()
            .map(|mut c| c.wait());
    }
}

/// ChefBar als systemd user-service bij login? (`chefbar.service` enabled).
pub fn autostart_enabled() -> bool {
    std::process::Command::new("systemctl")
        .args(["--user", "is-enabled", "chefbar.service"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Toggle "meelopen vanaf login" (chefbar.service enable/disable).
pub fn toggle_autostart() {
    let verb = if autostart_enabled() {
        "disable"
    } else {
        "enable"
    };
    let _ = std::process::Command::new("systemctl")
        .args(["--user", verb, "chefbar.service"])
        .status();
}
/// Fetch de laatste snapshot en werk de tray bij (parity: tooltip + icon per
/// status, zoals indicator.py dat per poll deed).
pub fn update_from(shared: &Arc<RwLock<Snapshot>>) {
    let Some(slot) = TRAY_HANDLE.get() else {
        return;
    };
    let guard = slot.lock().unwrap();
    let Some(handle) = guard.as_ref() else {
        return;
    };
    let (mut state, mut line) = shared
        .read()
        .map(|s| s.tray_state())
        .unwrap_or_else(|_| ("offline".into(), "ChefGroep".into()));
    let n = inbox_count(shared);
    if n > 0 {
        if state == "stil" {
            state = "hulp".into();
        }
        let suffix = if n == 1 {
            " · 1 melding".to_string()
        } else {
            format!(" · {n} meldingen")
        };
        line.push_str(&suffix);
    }
    let icon = tray_icon_for(&state);
    handle.update(|tray| {
        // Testhook-glyph: maximaal 10s vasthouden, daarna terug naar live.
        let forced = tray.forced_state.lock().unwrap();
        if let Some((forced_state, at)) = forced.as_ref() {
            if at.elapsed() < std::time::Duration::from_secs(10) {
                tray.icon = tray_icon_for(forced_state);
                *tray.status_line.lock().unwrap() =
                    format!("ChefGroep · test-glyph [{forced_state}]");
                return;
            }
        }
        tray.icon = icon;
        *tray.status_line.lock().unwrap() = line;
    });
}

impl ksni::Tray for ChefTray {
    fn id(&self) -> String {
        "chefbar".into()
    }
    fn title(&self) -> String {
        "ChefBar".into()
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![self.icon.clone()]
    }
    fn tool_tip(&self) -> ksni::ToolTip {
        let (state, line) = self
            .shared
            .read()
            .map(|s| s.tray_state())
            .unwrap_or_default();
        let _ = state;
        ksni::ToolTip {
            title: "ChefBar".into(),
            description: line,
            icon_name: "chefbar".into(),
            icon_pixmap: vec![self.icon.clone()],
        }
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        // Q3: alle menu-inhoud komt uit de pure builder (data, geen closures) —
        // deze methode is alleen nog de dunne ksni-adapter. De builder is
        // unit-testbaar zonder ksni/GTK (hier zat de E0382/E0597-breuk).
        let snap = self.shared.read().map(|s| s.clone()).unwrap_or_default();
        let profile = crate::config::global_profile();
        let specs = menu_items(&snap, profile, crate::tray::autostart_enabled());
        specs.into_iter().map(MenuItemSpec::into_ksni).collect()
    }
}

// ---------------------------------------------------------------------------
// Q3: pure tray-menu-builder — inhoud als data, geen ksni-types/closures.
// ---------------------------------------------------------------------------

/// Één tray-menu-regel als pure data (Q3). `menu()` vertaalt dit naar
/// ksni-items; de logica (welke rijen, welke commando's) is hier testbaar
/// zonder ksni-closures en zonder GTK.
#[derive(Debug, Clone, PartialEq)]
enum MenuItemSpec {
    Separator,
    /// Eén actie: klik → UiCommand naar de UI-thread.
    Action {
        label: String,
        icon: String,
        cmd: UiCommand,
    },
    /// Aanvinkbare rij (autostart).
    Checkmark {
        label: String,
        checked: bool,
        cmd: UiCommand,
    },
    /// Submenu met eigen rijen (account wisselen).
    Submenu {
        label: String,
        icon: String,
        items: Vec<MenuItemSpec>,
    },
    /// Uitgegrijsde info-rij (bijv. geen accounts om te wisselen).
    Disabled(String),
}

impl MenuItemSpec {
    /// Label voor tests/overzicht (separator krijgt een placeholder).
    #[cfg(test)]
    fn label(&self) -> &str {
        match self {
            MenuItemSpec::Separator => "──",
            MenuItemSpec::Action { label, .. }
            | MenuItemSpec::Checkmark { label, .. }
            | MenuItemSpec::Submenu { label, .. }
            | MenuItemSpec::Disabled(label) => label,
        }
    }

    /// Dunne ksni-adapter: één spec → ksni-menu-item. Geen beslislogica hier.
    fn into_ksni(self) -> ksni::MenuItem<ChefTray> {
        match self {
            MenuItemSpec::Separator => ksni::MenuItem::Separator,
            MenuItemSpec::Action { label, icon, cmd } => {
                ksni::MenuItem::Standard(StandardItem::<ChefTray> {
                    label,
                    icon_name: icon,
                    activate: Box::new(move |tray: &mut ChefTray| tray.send(cmd.clone())),
                    ..Default::default()
                })
            }
            MenuItemSpec::Checkmark {
                label,
                checked,
                cmd,
            } => ksni::MenuItem::Checkmark(ksni::menu::CheckmarkItem::<ChefTray> {
                label,
                checked,
                activate: Box::new(move |tray: &mut ChefTray| tray.send(cmd.clone())),
                ..Default::default()
            }),
            MenuItemSpec::Submenu { label, icon, items } => {
                let submenu: Vec<ksni::MenuItem<ChefTray>> =
                    items.into_iter().map(MenuItemSpec::into_ksni).collect();
                ksni::MenuItem::SubMenu(ksni::menu::SubMenu::<ChefTray> {
                    label,
                    icon_name: icon,
                    submenu,
                    ..Default::default()
                })
            }
            MenuItemSpec::Disabled(label) => ksni::MenuItem::Standard(StandardItem::<ChefTray> {
                label,
                enabled: false,
                ..Default::default()
            }),
        }
    }
}

/// Bouw de volledige tray-menu-inhoud als pure data (Q3). Eén plek met alle
/// beslislogica: eventregels → acties → accounts → desktop → notificaties →
/// systeemrijen. Geen I/O (autostart wordt als flag meegegeven).
fn menu_items(
    snap: &Snapshot,
    profile: &crate::config::EndpointProfile,
    autostart: bool,
) -> Vec<MenuItemSpec> {
    let mut items: Vec<MenuItemSpec> = Vec::new();

    // Live eventregels (max 3, nieuwste eerst) — klik → focus agent.
    let sessions = crate::sessions::load_ranked_sessions(&snap.events);
    for session in sessions.iter().take(3) {
        let stamp = match session.state.as_str() {
            "working" | "starting" => "BEZIG",
            "done" | "ok" => "KLAAR",
            "waiting" | "blocked" | "failed" => "JOUW",
            _ => "…",
        };
        let label = if session.title.len() > 38 {
            format!("{}…", &session.title[..38])
        } else {
            session.title.clone()
        };
        let focus = session
            .attach
            .focus
            .clone()
            .unwrap_or_else(|| session.id.clone());
        items.push(MenuItemSpec::Action {
            label: format!("{label}  [{stamp}]"),
            icon: "system-run-symbolic".into(),
            cmd: UiCommand::FocusAgent(focus),
        });
    }
    if !items.is_empty() {
        items.push(MenuItemSpec::Separator);
    }

    // Acties: Open Thuis / Open Ploeg.
    items.push(MenuItemSpec::Action {
        label: "Open Thuis".into(),
        icon: "go-home-symbolic".into(),
        cmd: UiCommand::OpenUrl(profile.dashboard.clone()),
    });
    items.push(MenuItemSpec::Action {
        label: "Open Ploeg".into(),
        icon: "x-office-document-symbolic".into(),
        cmd: UiCommand::OpenUrl(profile.ops_api.clone()),
    });
    items.push(MenuItemSpec::Separator);

    // Account-submenu (zelfde data als Vault Accounts).
    let mut account_items: Vec<MenuItemSpec> = Vec::new();
    for row in &snap.providers {
        for acc in &row.accounts {
            let acc_id = acc.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if Some(acc_id) == row.active_id.as_deref() {
                continue;
            }
            let label = acc
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or(acc_id)
                .to_string();
            account_items.push(MenuItemSpec::Action {
                label: format!("Werk als {label}"),
                icon: "avatar-default-symbolic".into(),
                cmd: UiCommand::SwitchAccount {
                    account_id: acc_id.to_string(),
                    source: row.source.clone(),
                    driver: row.driver.clone(),
                },
            });
        }
    }
    if account_items.is_empty() {
        items.push(MenuItemSpec::Disabled(
            "Account: niks om te wisselen".into(),
        ));
    } else {
        items.push(MenuItemSpec::Submenu {
            label: "Account wisselen".into(),
            icon: "avatar-default-symbolic".into(),
            items: account_items,
        });
    }

    // Desktop starten/stoppen.
    let desktop_running = snap.desktop.get("state").and_then(|v| v.as_str()) == Some("running");
    items.push(MenuItemSpec::Action {
        label: if desktop_running {
            "Desktop stoppen".into()
        } else {
            "Desktop starten".into()
        },
        icon: if desktop_running {
            "system-shutdown-symbolic".into()
        } else {
            "computer-symbolic".into()
        },
        cmd: UiCommand::DesktopAction(if desktop_running { "stop" } else { "start" }.into()),
    });
    items.push(MenuItemSpec::Separator);

    // Notificaties pauzeren + meelopen vanaf login.
    items.push(MenuItemSpec::Action {
        label: "Notificaties pauzeren (1u)".into(),
        icon: "notification-disabled-symbolic".into(),
        cmd: UiCommand::PauseNotifications,
    });
    items.push(MenuItemSpec::Checkmark {
        label: "Meelopen vanaf login".into(),
        checked: autostart,
        cmd: UiCommand::ToggleAutostart,
    });
    items.push(MenuItemSpec::Separator);

    items.push(MenuItemSpec::Action {
        label: "Ververs".into(),
        icon: "view-refresh-symbolic".into(),
        cmd: UiCommand::Refresh,
    });
    items.push(MenuItemSpec::Action {
        label: "Doctor".into(),
        icon: "diagnostics-symbolic".into(),
        cmd: UiCommand::Doctor,
    });
    items.push(MenuItemSpec::Action {
        label: "Afsluiten".into(),
        icon: "application-exit-symbolic".into(),
        cmd: UiCommand::Quit,
    });
    items
}

/// Het opgeloste thema ("dark"/"light") voor de tray-pixmap: een pixmap
/// kleurt niet mee met het panel-thema, dus het basislijntje moet contrasteren
/// met de tray-achtergrond (donker paneel -> lichte lijn, licht paneel ->
/// donkere lijn). Wordt éénmalig gezet vanuit main na css::detect_theme.
static THEME: std::sync::RwLock<&'static str> = std::sync::RwLock::new("dark");

pub fn set_theme(theme: &str) {
    let t: &'static str = if theme == crate::css::THEME_LIGHT {
        "light"
    } else {
        "dark"
    };
    *THEME.write().unwrap() = t;
}

/// Programmatisch gegenereerd 22x22 ARGB-pictogram: de CG-statuslijn —
/// een verticale lijn met drie segmentmarkeringen (spec: chefbar-tray.md).
/// States via vorm + badge, nooit alleen kleur:
/// stil = lijn in outline, bezig = gevuld middensegment, hulp = amber-dot
/// rechtsboven, fout = !-badge, offline = gestreepte lijn.
fn tray_icon_for(state: &str) -> ksni::Icon {
    const SIZE: usize = 22;
    // v2-tokenwaarden per tray-achtergrond (design-system tokens.css,
    // skin devin). Lijn = text-muted over de traykleur; statussen volgen
    // het v2-spectrum (accent / amber hold / rood).
    let light = *THEME.read().unwrap() == "light";
    let (line_c, accent_c, amber_c, red_c) = if light {
        (
            (0x73, 0x73, 0x73), // text-muted rgba(0,0,0,0.55) op lichte tray
            (0x31, 0x7C, 0xFF), // accent licht
            (0xBF, 0x5B, 0x00), // amber licht (hold)
            (0xCF, 0x22, 0x2E), // rood licht
        )
    } else {
        (
            (0x90, 0x8F, 0x8C), // text-muted over basalt-tray (#1B1A19)
            (0x5C, 0x97, 0xFF), // accent donker
            (0xD9, 0xA0, 0x38), // amber donker (hold)
            (0xF8, 0x51, 0x49), // rood donker
        )
    };

    let mut px = vec![0u8; SIZE * SIZE * 4];
    let rect =
        |px: &mut Vec<u8>, x0: usize, y0: usize, w: usize, h: usize, c: (u8, u8, u8), a: u8| {
            for y in y0..(y0 + h) {
                for x in x0..(x0 + w) {
                    if x < SIZE && y < SIZE {
                        let i = (y * SIZE + x) * 4;
                        px[i] = a;
                        px[i + 1] = c.0;
                        px[i + 2] = c.1;
                        px[i + 3] = c.2;
                    }
                }
            }
        };
    let disc = |px: &mut Vec<u8>, cx: f64, cy: f64, r: f64, c: (u8, u8, u8), a: u8| {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let d = (x as f64 + 0.5 - cx).hypot(y as f64 + 0.5 - cy);
                if d <= r {
                    let i = (y * SIZE + x) * 4;
                    px[i] = a;
                    px[i + 1] = c.0;
                    px[i + 2] = c.1;
                    px[i + 3] = c.2;
                }
            }
        }
    };

    let alpha_line: u8 = match state {
        "offline" => 70,
        "stil" => 170,
        _ => 235,
    };
    // Verticale lijn x=9..11; offline = gestreept (dashes met gaten).
    if state == "offline" {
        for (y0, h) in [(4usize, 3usize), (9, 3), (14, 3)] {
            rect(&mut px, 9, y0, 2, h, line_c, alpha_line);
        }
    } else {
        rect(&mut px, 9, 4, 2, 13, line_c, alpha_line);
    }
    // Drie segmentmarkeringen (ticks over de lijn).
    for y in [5usize, 10, 15] {
        rect(&mut px, 8, y, 4, 2, line_c, alpha_line);
    }
    match state {
        "bezig" => {
            // Gevuld middensegment in accent.
            rect(&mut px, 7, 9, 6, 4, accent_c, 255);
        }
        "hulp" => {
            // Gevuld topsegment + amber hold-dot rechtsboven.
            rect(&mut px, 7, 4, 6, 4, accent_c, 255);
            disc(&mut px, 16.0, 6.0, 3.0, amber_c, 255);
        }
        "fout" => {
            // !-badge rechts: staaf + dot in rood.
            rect(&mut px, 15, 4, 2, 6, red_c, 255);
            disc(&mut px, 16.0, 13.0, 1.6, red_c, 255);
            rect(&mut px, 7, 14, 6, 4, red_c, 255);
        }
        _ => {}
    }
    ksni::Icon {
        width: SIZE as i32,
        height: SIZE as i32,
        data: px,
    }
}

pub fn tray_icon() -> ksni::Icon {
    tray_icon_for("stil")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EndpointProfile;
    use crate::models::{ProviderRow, Snapshot};

    fn labels(items: &[MenuItemSpec]) -> Vec<&str> {
        items.iter().map(MenuItemSpec::label).collect()
    }

    #[test]
    fn basis_rijen_zijn_aanwezig() {
        let items = menu_items(&Snapshot::default(), &EndpointProfile::default(), false);
        let labels = labels(&items);
        for expected in [
            "Open Thuis",
            "Open Ploeg",
            "Notificaties pauzeren (1u)",
            "Meelopen vanaf login",
            "Ververs",
            "Doctor",
            "Afsluiten",
        ] {
            assert!(labels.contains(&expected), "rij ontbreekt: {expected}");
        }
        // Afsluiten is de laatste rij.
        assert_eq!(labels.last(), Some(&"Afsluiten"));
    }

    #[test]
    fn zonder_accounts_is_de_rij_disabled() {
        let items = menu_items(&Snapshot::default(), &EndpointProfile::default(), false);
        assert!(items.iter().any(|i| matches!(i, MenuItemSpec::Disabled(_))));
    }

    #[test]
    fn account_submenu_bouwt_switchacties_voor_niet_actieve_accounts() {
        let mut snap = Snapshot::default();
        snap.providers.push(ProviderRow {
            label: "Vault".into(),
            source: "vault".into(),
            active_id: Some("acc-1".into()),
            accounts: vec![
                serde_json::json!({"id": "acc-1", "label": "Hoofd"}),
                serde_json::json!({"id": "acc-2", "label": "Zakelijk"}),
            ],
            ..Default::default()
        });
        let items = menu_items(&snap, &EndpointProfile::default(), false);
        let submenu = items.iter().find_map(|i| match i {
            MenuItemSpec::Submenu { label, items, .. } if label == "Account wisselen" => {
                Some(items)
            }
            _ => None,
        });
        let submenu = submenu.expect("account-submenu aanwezig");
        // Alleen acc-2 (acc-1 is actief en wordt overgeslagen).
        assert_eq!(labels(submenu), vec!["Werk als Zakelijk"]);
        let cmds: Vec<&UiCommand> = submenu
            .iter()
            .filter_map(|i| match i {
                MenuItemSpec::Action { cmd, .. } => Some(cmd),
                _ => None,
            })
            .collect();
        assert_eq!(
            cmds,
            vec![&UiCommand::SwitchAccount {
                account_id: "acc-2".into(),
                source: "vault".into(),
                driver: None,
            }]
        );
    }

    #[test]
    fn desktop_actie_volgt_snapshot_state() {
        let mut snap = Snapshot::default();
        snap.desktop
            .insert("state".into(), serde_json::Value::String("running".into()));
        let items = menu_items(&snap, &EndpointProfile::default(), false);
        let desktop = items.iter().find_map(|i| match i {
            MenuItemSpec::Action { label, cmd, .. } if label.starts_with("Desktop") => {
                Some((label.as_str(), cmd.clone()))
            }
            _ => None,
        });
        assert_eq!(desktop.as_ref().map(|(l, _)| *l), Some("Desktop stoppen"));
        assert_eq!(
            desktop.map(|(_, cmd)| cmd),
            Some(UiCommand::DesktopAction("stop".into()))
        );
    }

    #[test]
    fn autostart_checkmark_reflecteert_flag() {
        let items = menu_items(&Snapshot::default(), &EndpointProfile::default(), true);
        assert!(items
            .iter()
            .any(|i| matches!(i, MenuItemSpec::Checkmark { checked: true, .. })));
        let items = menu_items(&Snapshot::default(), &EndpointProfile::default(), false);
        assert!(items
            .iter()
            .any(|i| matches!(i, MenuItemSpec::Checkmark { checked: false, .. })));
    }

    #[test]
    fn zonder_events_geen_focus_rijen() {
        let items = menu_items(&Snapshot::default(), &EndpointProfile::default(), false);
        assert!(!items.iter().any(|i| matches!(
            i,
            MenuItemSpec::Action {
                cmd: UiCommand::FocusAgent(_),
                ..
            }
        )));
    }
}
