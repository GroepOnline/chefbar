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
    ToggleBar,
    Refresh,
    Doctor,
    Quit,
}

/// Glib-idle-bridge: leegt het commando-kanaal op de UI-thread.
pub fn start_command_bridge(rx: std::sync::mpsc::Receiver<UiCommand>, dispatcher: Arc<dyn Fn(UiCommand)>) {
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
}

impl ChefTray {
    pub fn new(shared: Arc<RwLock<Snapshot>>, tx: Sender<UiCommand>) -> Self {
        Self {
            shared,
            tx,
            icon: tray_icon(),
        }
    }

    fn send(&self, cmd: UiCommand) {
        let _ = self.tx.send(cmd);
    }
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
        let (state, line) = self.shared.read().map(|s| s.tray_state()).unwrap_or_default();
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
                label: "ChefBar-dashboard".into(),
                icon_name: "utilities-system-monitor-symbolic".into(),
                activate: Box::new(|tray: &mut Self| tray.send(UiCommand::TogglePanel)),
                ..Default::default()
            }),
            ksni::MenuItem::Standard(StandardItem::<Self> {
                label: "Opdrachtbalk".into(),
                icon_name: "system-search-symbolic".into(),
                activate: Box::new(|tray: &mut Self| tray.send(UiCommand::ToggleBar)),
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

/// Programmatisch gegenereerd 22x22 RGBA-pictogram: donkere pill met
/// accent-dot + groene status-dot (alleen data, geen assets nodig).
fn tray_icon() -> ksni::Icon {
    const SIZE: usize = 22;
    let bg: (u8, u8, u8) = (0x16, 0x18, 0x1C);
    let accent: (u8, u8, u8) = (0x4F, 0x8D, 0xFF);
    let green: (u8, u8, u8) = (0x3F, 0xB9, 0x50);
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
                if dx <= 3.2 {
                    (accent.0, accent.1, accent.2, 255)
                } else if gx2 <= 2.5 {
                    (green.0, green.1, green.2, 255)
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

/// Snapshot-handle voor de tray: kleine clone-waardige struct.
pub fn snapshot_handle(snapshot: &Arc<RwLock<Snapshot>>) -> Arc<RwLock<Snapshot>> {
    snapshot.clone()
}

/// Mutex-free status-update voor testbare tooltips.
pub fn status_line(snapshot: &Arc<RwLock<Snapshot>>) -> String {
    snapshot
        .read()
        .map(|s| s.tray_state().1)
        .unwrap_or_else(|_| "ChefGroep".into())
}

pub struct StatusCache {
    pub line: Mutex<String>,
}

impl StatusCache {
    pub fn new() -> Self {
        Self {
            line: Mutex::new(String::new()),
        }
    }
}