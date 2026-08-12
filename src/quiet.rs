//! Do-not-disturb-schema (rustige uren): `CHEFBAR_QUIET="HH:MM-HH:MM"`.
//!
//! Eén venster per dag, overnight toegestaan (22:00-07:00). Tijdens rustige
//! uren dempt de toast-route niet-kritieke meldingen (KLAAR/HULP/LIMIET);
//! FOUT gaat altijd door. De inbox blijft gewoon gevuld — alleen de toast
//! zwijgt. De warden-laag blijft per veld: geen nieuw configbestand, alleen
//! env. Geen chrono-dep nodig: lokale tijd via `libc::localtime_r` (libc zit
//! al in de tree), de window-logica zelf is puur en unit-testbaar.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuietWindow {
    pub from_h: u32,
    pub from_m: u32,
    pub to_h: u32,
    pub to_m: u32,
}

/// Parse `CHEFBAR_QUIET="22:00-07:00"`. Ongeldig/leeg → None (rustige uren
/// uit). Tolerante parse: minuten mogen ontbreken (22-07 = 22:00-07:00).
pub fn quiet_window() -> Option<QuietWindow> {
    let raw = std::env::var("CHEFBAR_QUIET").ok()?;
    quiet_window_from(&raw)
}

/// Pure kern van `quiet_window` — testbaar zonder env.
pub fn quiet_window_from(raw: &str) -> Option<QuietWindow> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let (from, to) = raw.split_once('-')?;
    let from = parse_hhmm(from.trim())?;
    let to = parse_hhmm(to.trim())?;
    Some(QuietWindow {
        from_h: from.0,
        from_m: from.1,
        to_h: to.0,
        to_m: to.1,
    })
}

fn parse_hhmm(text: &str) -> Option<(u32, u32)> {
    let (h, m) = match text.split_once(':') {
        Some((h, m)) => (h, m),
        None => (text, "0"),
    };
    let h: u32 = h.trim().parse().ok()?;
    let m: u32 = m.trim().parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some((h, m))
}

/// In rustige uren? (pure logica, incl. overnight-wrap en grenzen —
/// `from` exact = actief, `to` exact = niet meer.)
pub fn in_quiet_hours_at(window: &QuietWindow, hour: u32, minute: u32) -> bool {
    let now = hour * 60 + minute;
    let from = window.from_h * 60 + window.from_m;
    let to = window.to_h * 60 + window.to_m;
    if from == to {
        // Leeg/heel-dag venster: nooit actief (config-fout, veilig uit).
        return false;
    }
    if from < to {
        now >= from && now < to
    } else {
        now >= from || now < to
    }
}

/// Locale tijd als (uur, minuut) via libc — geen chrono-dep.
pub fn local_hhmm() -> (u32, u32) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as libc::time_t)
        .unwrap_or(0);
    // Safe: localtime_r schrijft alleen naar de door ons aangewezen tm.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe {
        libc::localtime_r(&now, &mut tm);
    }
    (tm.tm_hour as u32, tm.tm_min as u32)
}

/// Is het nú rustige uren (als er een venster geconfigureerd is)?
pub fn in_quiet_hours(window: &QuietWindow) -> bool {
    let (h, m) = local_hhmm();
    in_quiet_hours_at(window, h, m)
}

/// Compact label voor menu/doctor: "22:00–07:00".
pub fn window_label(window: &QuietWindow) -> String {
    format!(
        "{:02}:{:02}–{:02}:{:02}",
        window.from_h, window.from_m, window.to_h, window.to_m
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(from_h: u32, from_m: u32, to_h: u32, to_m: u32) -> QuietWindow {
        QuietWindow {
            from_h,
            from_m,
            to_h,
            to_m,
        }
    }

    #[test]
    fn parse_geldig_en_overnight() {
        assert_eq!(quiet_window_from("22:00-07:00"), Some(w(22, 0, 7, 0)));
        assert_eq!(quiet_window_from(" 09:30-17:00 "), Some(w(9, 30, 17, 0)));
    }

    #[test]
    fn parse_zonder_minuten() {
        assert_eq!(quiet_window_from("22-07"), Some(w(22, 0, 7, 0)));
    }

    #[test]
    fn parse_ongeldig_is_uit() {
        assert_eq!(quiet_window_from(""), None);
        assert_eq!(quiet_window_from("22:00"), None); // geen '-'
        assert_eq!(quiet_window_from("25:00-07:00"), None); // uur > 23
        assert_eq!(quiet_window_from("22:00-07:61"), None); // minuut > 59
        assert_eq!(quiet_window_from("abc-def"), None);
    }

    #[test]
    fn dagvenster_actief_binnen_grenzen() {
        let window = w(9, 0, 17, 30);
        assert!(!in_quiet_hours_at(&window, 8, 59));
        assert!(in_quiet_hours_at(&window, 9, 0)); // from exact = actief
        assert!(in_quiet_hours_at(&window, 12, 0));
        assert!(in_quiet_hours_at(&window, 17, 29));
        assert!(!in_quiet_hours_at(&window, 17, 30)); // to exact = niet meer
    }

    #[test]
    fn overnight_venster_wrapt() {
        let window = w(22, 0, 7, 0);
        assert!(!in_quiet_hours_at(&window, 21, 59));
        assert!(in_quiet_hours_at(&window, 22, 0));
        assert!(in_quiet_hours_at(&window, 23, 59));
        assert!(in_quiet_hours_at(&window, 0, 30));
        assert!(in_quiet_hours_at(&window, 6, 59));
        assert!(!in_quiet_hours_at(&window, 7, 0));
    }

    #[test]
    fn gelijke_grenzen_is_uit() {
        // from == to: config-fout, veilig uit (nooit actief).
        assert!(!in_quiet_hours_at(&w(12, 0, 12, 0), 12, 0));
    }

    #[test]
    fn label_is_compact() {
        assert_eq!(window_label(&w(22, 0, 7, 0)), "22:00–07:00");
        assert_eq!(window_label(&w(9, 30, 17, 0)), "09:30–17:00");
    }
}
