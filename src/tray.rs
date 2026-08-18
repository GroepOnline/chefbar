//! Systeemtray (ksni) — pure Rust, geen Ayatana/AppIndicator dependency.
//!
//! Tray draait in zijn eigen thread en stuurt alleen UI-commando's door een
//! mpsc-kanaal die de glib-mainloop om de zoveel tijd leegt (idle dispatch).

use crate::models::Snapshot;
use gtk::glib::ControlFlow;
use ksni::menu::StandardItem;
use std::collections::HashSet;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};

static UI_TX: Mutex<Option<Sender<UiCommand>>> = Mutex::new(None);

/// Register the in-process UI command sender (tray / ipc / palette).
pub fn register_command_tx(tx: Sender<UiCommand>) {
    *UI_TX
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(tx);
}

/// Queue a UI command on the GTK dispatcher. False when no sender is registered
/// yet (tests, early startup).
pub fn send_ui(cmd: UiCommand) -> bool {
    let Ok(guard) = UI_TX.lock() else {
        return false;
    };
    let Some(tx) = guard.as_ref() else {
        return false;
    };
    tx.send(cmd).is_ok()
}

/// UI-commando's van tray/ipc naar de GTK-thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    TogglePanel,
    ShowPanel,
    Refresh,
    Doctor,
    Quit,
    /// Open een URL via de executor (sessies, Linear, policy-checked).
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
    /// Desktop-IPC (`desktop start|stop`) is een no-op: geen lokale webtop.
    DesktopAction(String),
    /// Demp of ont-demp één agent in de watcher/inbox.
    ToggleMute(String),
    /// Forceer de tray-glyph-state (testhook: stil/bezig/hulp/fout/offline)
    /// voor live verificatie op een echt GNOME-panel (brief W3).
    ForceState(String),
    /// Focus een specifiek domein in het panel (sidebar-nav via IPC/palette).
    FocusDomain(String),
    /// Toggle de command-palette-overlay binnen het panel (Super+Shift+Space).
    TogglePalette,
    /// Open de inbox-zone in het panel.
    OpenInbox,
    /// Preview de detail-drawer met de eerste actie (visual-shot/CI path).
    DrawerPreview,
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
#[allow(dead_code)]
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

/// Domein-definitie voor FocusDomain-tray-menu en group-dots.
fn tray_domains() -> Vec<(&'static str, &'static str)> {
    vec![
        ("inbox", "Inbox"),
        ("fleet", "Fleet"),
        ("herdr", "Herdr"),
        ("control", "Control"),
        ("vault", "Vault"),
        ("share", "Share"),
        ("tasks", "Taken"),
        ("containers", "Containers"),
        ("secrets", "Secrets"),
        ("kater", "Kater"),
        ("health", "Health"),
    ]
}

#[allow(dead_code)]
fn domain_label(id: &str) -> &'static str {
    match id {
        "inbox" => "Inbox",
        "fleet" => "Fleet",
        "herdr" => "Herdr",
        "control" => "Control",
        "vault" => "Vault",
        "share" => "Share",
        "tasks" | "taken" => "Taken",
        "commerce" | "accounts" | "providers" => "Accounts",
        "containers" => "Containers",
        "secrets" => "Secrets",
        "kater" => "Kater",
        "health" => "Health",
        _ => "Onbekend",
    }
}

/// Bouw één regel met group-dots voorgelegd: e.g. "Fleet · ● Herdr ● Vault"
/// De caller bepaalt n; deze helper voegt per-domain een kleine dot toe
/// op basis van harness-kleur (harness.rs) waar beschikbaar.
fn tray_status_suffix(inbox_n: usize, health_level: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    if inbox_n > 0 {
        if inbox_n == 1 {
            parts.push("1 om aandacht".to_string());
        } else {
            parts.push(format!("{inbox_n} om aandacht"));
        }
    }
    // Group-dots: kleine coloured dots per harness-group met een live-like hint.
    // In pure tray-code hebben we geen HarnessKind import nodig; we gebruiken
    // vaste kleuren (zie harness.rs colors voor fleet/commerce/sync/eval).
    // Voor nu: één dot per health/level als subtiele indicator.
    if health_level == "warn" {
        parts.push("●".to_string());
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" · {}", parts.join(" · "))
    }
}

/// Publieke helper voor tests: map tray-status naar (state, line) met inbox-count.
pub fn tray_status_for(snapshot: &Snapshot) -> (String, String) {
    let (mut state, mut line) = snapshot.tray_state();
    let n = snapshot
        .suggestions
        .iter()
        .filter(|sg| sg.fresh(crate::models::SUGGESTION_TTL_SECONDS))
        .count();
    let suffix = tray_status_suffix(n, &snapshot.health.level);
    if !suffix.is_empty() {
        if state == "stil" && n > 0 {
            state = "hulp".into();
        }
        line.push_str(&suffix);
    }
    (state, line)
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
    let (n, level) = shared
        .read()
        .map(|s| {
            let n = s
                .suggestions
                .iter()
                .filter(|sg| sg.fresh(crate::models::SUGGESTION_TTL_SECONDS))
                .count();
            (n, s.health.level.clone())
        })
        .unwrap_or((0, String::new()));
    let suffix = tray_status_suffix(n, &level);
    if !suffix.is_empty() {
        if state == "stil" && n > 0 {
            state = "hulp".into();
        }
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
        "ChefApp".into()
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
            title: "ChefApp".into(),
            description: line,
            icon_name: "chefbar".into(),
            icon_pixmap: vec![self.icon.clone()],
        }
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let mut items: Vec<ksni::MenuItem<Self>> = Vec::new();

        // Kopieer menu-invoer binnen een korte read-lock; callbacks houden
        // daarna geen snapshot-guard vast terwijl het menu wordt opgebouwd.
        let (events, inbox_n, mute_agents) = self
            .shared
            .read()
            .map(|snapshot| {
                let inbox_n = snapshot
                    .suggestions
                    .iter()
                    .filter(|sg| sg.fresh(crate::models::SUGGESTION_TTL_SECONDS))
                    .count();
                let mute_agents: Vec<(String, String, String)> = snapshot
                    .agents
                    .iter()
                    .map(|agent| {
                        (
                            agent.key.clone(),
                            agent.agent.clone(),
                            agent.workspace.clone(),
                        )
                    })
                    .collect();
                (snapshot.events.clone(), inbox_n, mute_agents)
            })
            .unwrap_or_default();
        let sessions = crate::sessions::load_tray_events(&events);
        // Inbox-count regel bovenaan indien non-empty: "3 om aandacht".
        if inbox_n > 0 {
            let label = if inbox_n == 1 {
                "1 om aandacht".to_string()
            } else {
                format!("{inbox_n} om aandacht")
            };
            items.push(ksni::MenuItem::Standard(StandardItem::<Self> {
                label,
                icon_name: "mail-unread-symbolic".into(),
                activate: Box::new(|tray: &mut Self| tray.send(UiCommand::OpenInbox)),
                ..Default::default()
            }));
            items.push(ksni::MenuItem::Separator);
        }
        for (shown, session) in sessions.iter().take(6).enumerate() {
            if shown >= 5 {
                break;
            }
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
            items.push(ksni::MenuItem::Standard(StandardItem::<Self> {
                label: format!("{label}  [{stamp}]"),
                icon_name: "system-run-symbolic".into(),
                activate: Box::new(move |tray: &mut Self| {
                    tray.send(UiCommand::FocusAgent(focus.clone()));
                }),
                ..Default::default()
            }));
        }
        if !items.is_empty() {
            items.push(ksni::MenuItem::Separator);
        }

        // Account-submenu (zelfde data als Vault Accounts).
        let mut account_items: Vec<ksni::MenuItem<Self>> = Vec::new();
        if let Ok(snap) = self.shared.read() {
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
                    let source = row.source.clone();
                    let driver = row.driver.clone();
                    let account_id = acc_id.to_string();
                    account_items.push(ksni::MenuItem::Standard(StandardItem::<Self> {
                        label: format!("Werk als {label}"),
                        icon_name: "avatar-default-symbolic".into(),
                        activate: Box::new(move |tray: &mut Self| {
                            tray.send(UiCommand::SwitchAccount {
                                account_id: account_id.clone(),
                                source: source.clone(),
                                driver: driver.clone(),
                            });
                        }),
                        ..Default::default()
                    }));
                }
            }
        }
        if account_items.is_empty() {
            items.push(ksni::MenuItem::Standard(StandardItem::<Self> {
                label: "Account: niks om te wisselen".into(),
                enabled: false,
                ..Default::default()
            }));
        } else {
            items.push(ksni::MenuItem::SubMenu(ksni::menu::SubMenu::<Self> {
                label: "Account wisselen".into(),
                icon_name: "avatar-default-symbolic".into(),
                submenu: account_items,
                ..Default::default()
            }));
        }

        // Per-agent mute: de state-poller filtert deze keys vóór toast/inbox.
        let mutes = crate::mutes::load();
        let mut mute_items: Vec<ksni::MenuItem<Self>> = Vec::new();
        let mut shown_keys: HashSet<String> = HashSet::new();
        for (key, agent, workspace) in mute_agents {
            let label = format!("{agent} · {workspace}");
            let checked = mutes.contains(key.as_str());
            shown_keys.insert(key.clone());
            mute_items.push(ksni::MenuItem::Checkmark(
                ksni::menu::CheckmarkItem::<Self> {
                    label,
                    checked,
                    activate: Box::new(move |tray: &mut Self| {
                        tray.send(UiCommand::ToggleMute(key.clone()));
                    }),
                    ..Default::default()
                },
            ));
        }
        // Gedempte agents die niet (meer) in de snapshot staan, blijven zo
        // dempbaar via het menu — anders kan een verdwenen agent nooit meer
        // worden gedemd (de-mute blijft mogelijk via dezelfde toggle).
        for key in mutes.iter() {
            if shown_keys.contains(key) {
                continue;
            }
            let key = key.clone();
            let label = format!("{key} · (niet actief)");
            mute_items.push(ksni::MenuItem::Checkmark(
                ksni::menu::CheckmarkItem::<Self> {
                    label,
                    checked: true,
                    activate: Box::new(move |tray: &mut Self| {
                        tray.send(UiCommand::ToggleMute(key.clone()));
                    }),
                    ..Default::default()
                },
            ));
        }
        if mute_items.is_empty() {
            items.push(ksni::MenuItem::Standard(StandardItem::<Self> {
                label: "Meldingen: geen agents actief".into(),
                enabled: false,
                ..Default::default()
            }));
        } else {
            items.push(ksni::MenuItem::SubMenu(ksni::menu::SubMenu::<Self> {
                label: "Demp agenten".into(),
                icon_name: "notification-disabled-symbolic".into(),
                submenu: mute_items,
                ..Default::default()
            }));
        }
        items.push(ksni::MenuItem::Separator);

        // Domein-nav: FocusDomain per domein (group-dots impliciet via labels).
        {
            let mut domain_items: Vec<ksni::MenuItem<Self>> = Vec::new();
            for (id, label) in tray_domains() {
                let domain = id.to_string();
                domain_items.push(ksni::MenuItem::Standard(StandardItem::<Self> {
                    label: label.to_string(),
                    icon_name: "go-next-symbolic".into(),
                    activate: Box::new(move |tray: &mut Self| {
                        tray.send(UiCommand::FocusDomain(domain.clone()));
                    }),
                    ..Default::default()
                }));
            }
            items.push(ksni::MenuItem::SubMenu(ksni::menu::SubMenu::<Self> {
                label: "Ga naar domein".into(),
                icon_name: "view-grid-symbolic".into(),
                submenu: domain_items,
                ..Default::default()
            }));
            items.push(ksni::MenuItem::Separator);
        }

        // Notificaties pauzeren + rustige uren + meelopen vanaf login.
        items.push(ksni::MenuItem::Standard(StandardItem::<Self> {
            label: "Notificaties pauzeren (1u)".into(),
            icon_name: "notification-disabled-symbolic".into(),
            activate: Box::new(|tray: &mut Self| tray.send(UiCommand::PauseNotifications)),
            ..Default::default()
        }));
        if let Some(window) = crate::quiet::quiet_window() {
            let active = crate::quiet::in_quiet_hours(&window);
            items.push(ksni::MenuItem::Standard(StandardItem::<Self> {
                label: format!(
                    "Rustige uren {} · {}",
                    crate::quiet::window_label(&window),
                    if active { "actief" } else { "stil" }
                ),
                enabled: false,
                ..Default::default()
            }));
        }
        let autostart = crate::tray::autostart_enabled();
        items.push(ksni::MenuItem::Checkmark(
            ksni::menu::CheckmarkItem::<Self> {
                label: "Meelopen vanaf login".into(),
                checked: autostart,
                activate: Box::new(|tray: &mut Self| tray.send(UiCommand::ToggleAutostart)),
                ..Default::default()
            },
        ));
        items.push(ksni::MenuItem::Separator);

        items.push(ksni::MenuItem::Standard(StandardItem::<Self> {
            label: "Ververs".into(),
            icon_name: "view-refresh-symbolic".into(),
            activate: Box::new(|tray: &mut Self| tray.send(UiCommand::Refresh)),
            ..Default::default()
        }));
        items.push(ksni::MenuItem::Standard(StandardItem::<Self> {
            label: "Doctor".into(),
            icon_name: "diagnostics-symbolic".into(),
            activate: Box::new(|tray: &mut Self| tray.send(UiCommand::Doctor)),
            ..Default::default()
        }));
        items.push(ksni::MenuItem::Standard(StandardItem::<Self> {
            label: "Afsluiten".into(),
            icon_name: "application-exit-symbolic".into(),
            activate: Box::new(|tray: &mut Self| tray.send(UiCommand::Quit)),
            ..Default::default()
        }));
        items
    }
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
