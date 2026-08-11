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

/// Programmatisch gegenereerd 22x22 ARGB-pictogram: de CG-statuslijn —
/// een verticale lijn met drie segmentmarkeringen (spec: chefbar-tray.md).
/// States via vorm + badge, nooit alleen kleur:
/// stil = lijn in outline, bezig = gevuld middensegment, hulp = ember-dot
/// rechtsboven, fout = !-badge, offline = gestreepte lijn.
fn tray_icon_for(state: &str) -> ksni::Icon {
    const SIZE: usize = 22;
    // Kleuren uit de Huly-tokenlijst (pixmap kan niet meekleuren met het
    // panel-thema; lichtgrijs leest op donker én licht).
    const LINE: (u8, u8, u8) = (0xC8, 0xCA, 0xD0);
    const IRIS: (u8, u8, u8) = (0x56, 0x83, 0xDA);
    const EMBER: (u8, u8, u8) = (0xFF, 0x89, 0x64);
    const RED: (u8, u8, u8) = (0xFF, 0x4D, 0x4D);

    let mut px = vec![0u8; SIZE * SIZE * 4];
    let rect = |px: &mut Vec<u8>, x0: usize, y0: usize, w: usize, h: usize, c: (u8, u8, u8), a: u8| {
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
        "stil" => 150,
        _ => 235,
    };
    // Verticale lijn x=9..11; offline = gestreept (dashes met gaten).
    if state == "offline" {
        for (y0, h) in [(4usize, 3usize), (9, 3), (14, 3)] {
            rect(&mut px, 9, y0, 2, h, LINE, alpha_line);
        }
    } else {
        rect(&mut px, 9, 4, 2, 13, LINE, alpha_line);
    }
    // Drie segmentmarkeringen (ticks over de lijn).
    for y in [5usize, 10, 15] {
        rect(&mut px, 8, y, 4, 2, LINE, alpha_line);
    }
    match state {
        "bezig" => {
            // Gevuld middensegment in Iris.
            rect(&mut px, 7, 9, 6, 4, IRIS, 255);
        }
        "hulp" => {
            // Gevuld topsegment + Ember brand-dot rechtsboven.
            rect(&mut px, 7, 4, 6, 4, IRIS, 255);
            disc(&mut px, 16.0, 6.0, 3.0, EMBER, 255);
        }
        "fout" => {
            // !-badge rechts: staaf + dot in rood.
            rect(&mut px, 15, 4, 2, 6, RED, 255);
            disc(&mut px, 16.0, 13.0, 1.6, RED, 255);
            rect(&mut px, 7, 14, 6, 4, RED, 255);
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
