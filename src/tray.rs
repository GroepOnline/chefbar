//! Systeemtray (ksni) — pure Rust, geen Ayatana/AppIndicator dependency.
//!
//! Tray draait in zijn eigen thread en stuurt alleen UI-commando's door een
//! mpsc-kanaal die de glib-mainloop om de zoveel tijd leegt (idle dispatch).

use crate::models::Snapshot;
use glib::ControlFlow;
use ksni::menu::StandardItem;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};

/// UI-commando's van tray/ipc naar de GTK-thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiCommand {
    TogglePanel,
    ShowPanel,
    Refresh,
    Doctor,
    Quit,
}

/// Glib-idle-bridge: leegt het commando-kanaal op de UI-thread.
pub fn start_command_bridge(
    rx: std::sync::mpsc::Receiver<UiCommand>,
    dispatcher: Arc<dyn Fn(UiCommand)>,
) {
    glib::timeout_add_local(std::time::Duration::from_millis(60), move || {
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
        }
    }

    fn send(&self, cmd: UiCommand) {
        let _ = self.tx.send(cmd);
    }
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
        vec![
            ksni::MenuItem::Standard(StandardItem::<Self> {
                label: "Openen".into(),
                icon_name: "utilities-system-monitor-symbolic".into(),
                activate: Box::new(|tray: &mut Self| tray.send(UiCommand::ShowPanel)),
                ..Default::default()
            }),
            ksni::MenuItem::Standard(StandardItem::<Self> {
                label: "Ververs".into(),
                icon_name: "view-refresh-symbolic".into(),
                activate: Box::new(|tray: &mut Self| tray.send(UiCommand::Refresh)),
                ..Default::default()
            }),
            ksni::MenuItem::Standard(StandardItem::<Self> {
                label: "Doctor".into(),
                icon_name: "diagnostics-symbolic".into(),
                activate: Box::new(|tray: &mut Self| tray.send(UiCommand::Doctor)),
                ..Default::default()
            }),
            ksni::MenuItem::Separator,
            ksni::MenuItem::Standard(StandardItem::<Self> {
                label: "Afsluiten".into(),
                icon_name: "application-exit-symbolic".into(),
                activate: Box::new(|tray: &mut Self| tray.send(UiCommand::Quit)),
                ..Default::default()
            }),
        ]
    }
}

/// Programmatisch gegenereerd 22x22 ARGB-pictogram; dot-kleur volgt de
/// tray-status (parity met de Python-indicator, alleen data, geen assets).
fn tray_icon_for(state: &str) -> ksni::Icon {
    const SIZE: usize = 22;
    let bg: (u8, u8, u8) = (0x16, 0x18, 0x1C);
    let accent: (u8, u8, u8) = match state {
        "offline" | "fout" => (0xF8, 0x51, 0x49), // red
        "hulp" => (0xD9, 0xA0, 0x38),              // amber
        "bezig" => (0x4F, 0x8D, 0xFF),             // accent
        _ => (0x3F, 0xB9, 0x50),                   // green
    };
    let mut pixels: Vec<u8> = Vec::with_capacity(SIZE * SIZE * 4);

    for y in 0..SIZE {
        for x in 0..SIZE {
            // Afgeronde rechthoek (pill 5px radius).
            let cx = x as f64 - 10.5;
            let cy = y as f64 - 10.5;
            let rx = (cx.abs() - 7.5).max(0.0);
            let ry = (cy.abs() - 7.5).max(0.0);
            let outside = (rx * rx + ry * ry).sqrt() > 5.0;
            let (r, g, b, a) = if outside {
                (0, 0, 0, 0)
            } else {
                // Accent-dot linksonder, status-dot rechtsboven.
                let dx = (x as f64 - 6.5).hypot(y as f64 - 15.0);
                let gx2 = (x as f64 - 15.0).hypot(y as f64 - 6.5);
                if dx <= 3.2 || gx2 <= 2.5 {
                    (accent.0, accent.1, accent.2, 255)
                } else {
                    (bg.0, bg.1, bg.2, 255)
                }
            };
            // ARGB32, netwerk-byte-volgorde (ksni contract).
            pixels.extend_from_slice(&[a, r, g, b]);
        }
    }
    ksni::Icon {
        width: SIZE as i32,
        height: SIZE as i32,
        data: pixels,
    }
}

pub fn tray_icon() -> ksni::Icon {
    tray_icon_for("stil")
}
