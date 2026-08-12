//! Pure command-palette fuzzy ranking voor ChefApp.

use std::collections::HashMap;

use crate::aliases::expanded_terms as expand_query;

/// Actiebeschrijving: data, geen closures. Executie loopt in één executor.
#[derive(Debug, Clone)]
pub struct Action {
    pub title: String,
    pub meta: String,
    pub stamp: String,
    pub keywords: String,
    pub section: String,
    pub shortcut: String,
    pub needs_text: bool,
    pub destructive: bool,
    pub pinned: bool,
    pub run: crate::actions::RunSpec,
}

impl Action {
    pub fn matches(&self, query: &str) -> bool {
        fuzzy_score(query, self).is_some()
    }

    /// Stable lokale frecency-key; dezelfde titel met andere uitvoering blijft
    /// daardoor een afzonderlijke actie in de ranking.
    pub fn frecency_id(&self) -> String {
        format!("{}::{:?}", self.title, self.run)
    }
}

// ---------------------------------------------------------------------------
// Frecency
// ---------------------------------------------------------------------------

/// Frecency entry: (count, last_used_rfc3339).
/// Wordt gevuld uit `frecency.rs` (Lane A) of inline HashMap — geen harde dep.
/// `last_used` wordt getest op "binnen 24u" voor +60 boost.
pub fn apply_frecency_boost(action: &Action, frecency: &HashMap<String, (u32, String)>) -> i32 {
    if frecency.is_empty() {
        return 0;
    }
    let haystack = format!(
        "{} {} {} {}",
        action.title.to_lowercase(),
        action.meta.to_lowercase(),
        action.section.to_lowercase(),
        action.keywords.to_lowercase()
    );
    let action_id = action.frecency_id().to_lowercase();
    for (key, (_count, ts)) in frecency.iter() {
        let k = key.to_lowercase();
        if k.is_empty() {
            continue;
        }
        if k != action_id && !haystack.contains(&k) {
            continue;
        }
        if is_within_24h(ts) {
            return 60;
        }
    }
    0
}

fn is_within_24h(ts: &str) -> bool {
    if let Some(epoch) = parse_rfc3339_to_epoch(ts) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if epoch > now {
            // future timestamp (clock skew) — treat as recent if within 1h future
            return epoch.saturating_sub(now) < 3600;
        }
        return now.saturating_sub(epoch) < 86400;
    }
    false
}

fn parse_rfc3339_to_epoch(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Ok(epoch) = s.parse::<u64>() {
        return Some(epoch);
    }
    if s.len() < 10 {
        return None;
    }
    // Accept "YYYY-MM-DDTHH:MM:SSZ", "YYYY-MM-DD HH:MM:SS", with optional millis/timezone
    let (date_part, time_part) = if let Some(idx) = s.find('T') {
        (&s[..idx], &s[idx + 1..])
    } else {
        let idx = s.find(' ')?;
        (&s[..idx], &s[idx + 1..])
    };
    let date_parts: Vec<&str> = date_part.split('-').collect();
    if date_parts.len() != 3 {
        return None;
    }
    let y: i32 = date_parts[0].parse().ok()?;
    let m: u32 = date_parts[1].parse().ok()?;
    let d: u32 = date_parts[2].parse().ok()?;
    // time_part may be "HH:MM:SSZ" or "HH:MM:SS.xxxZ" or "HH:MM:SS+00:00"
    let time_clean = time_part
        .trim_end_matches('Z')
        .split('+')
        .next()
        .unwrap_or(time_part)
        .split('.')
        .next()
        .unwrap_or(time_part)
        .trim();
    let time_parts: Vec<&str> = time_clean.split(':').collect();
    if time_parts.len() < 2 {
        return None;
    }
    let hh: u32 = time_parts[0].parse().ok()?;
    let mm: u32 = time_parts[1].parse().ok()?;
    let ss: u32 = if time_parts.len() >= 3 {
        time_parts[2].parse().unwrap_or(0)
    } else {
        0
    };
    let days = days_from_civil(y, m, d)?;
    let epoch = (days as u64) * 86400 + (hh as u64) * 3600 + (mm as u64) * 60 + (ss as u64);
    Some(epoch)
}

fn days_from_civil(y: i32, m: u32, d: u32) -> Option<i64> {
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = y as i64;
    let m = m as i64;
    let d = d as i64;
    // Howard Hinnant civil_from_days inverse
    let y_adj = y - if m <= 2 { 1 } else { 0 };
    let era = (if y_adj >= 0 { y_adj } else { y_adj - 399 }) / 400;
    let yoe = y_adj - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

#[allow(dead_code)]
fn format_now_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d, hh, mm, ss) = civil_from_epoch(secs);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

#[allow(dead_code)]
fn civil_from_epoch(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    let days = (secs / 86400) as i64;
    let secs_of_day = (secs % 86400) as u32;
    let hh = secs_of_day / 3600;
    let mm = (secs_of_day % 3600) / 60;
    let ss = secs_of_day % 60;
    // days -> civil
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let y = y + if m <= 2 { 1 } else { 0 };
    (y as i32, m as u32, d as u32, hh, mm, ss)
}

// ---------------------------------------------------------------------------
// Ranking core
// ---------------------------------------------------------------------------

/// Rank ordered-character matches; exact words en prefixes winnen.
/// Query wordt eerst geëxpand via aliases zodat "cfg" ook "config" raakt.
pub fn fuzzy_score(query: &str, action: &Action) -> Option<i32> {
    let expanded = expand_query(query);
    // needle voor contains/prefix/gappy: join expanded terms met spatie
    // maar voor contains checken we alleen de volledige needle, niet per-token
    // (per-token zou "fl" al als contains tellen voor "fleet" — fout: dat is prefix).
    let needle_raw: String = query.split_whitespace().collect::<Vec<_>>().join(" ");
    if needle_raw.trim().is_empty() && expanded.is_empty() {
        return Some(0);
    }
    let haystack = format!(
        "{} {} {} {}",
        action.title.to_lowercase(),
        action.meta.to_lowercase(),
        action.section.to_lowercase(),
        action.keywords.to_lowercase()
    );
    // Tier 1 — contains: alleen volledige needle als substring, plus single-token alias.
    let needle_lower = needle_raw.to_lowercase();
    if !needle_lower.is_empty() && haystack.contains(&needle_lower) {
        return Some(1000 + (1000 - haystack.find(&needle_lower).unwrap_or(0) as i32).max(0));
    }
    // Single-token alias: "cfg" -> "config" — als alias-term in haystack, tel als contains.
    let orig_tokens: Vec<&str> = needle_lower.split_whitespace().collect();
    if orig_tokens.len() == 1 && expanded.len() > 1 {
        for term in expanded.iter() {
            if *term != needle_lower && haystack.contains(term) {
                return Some(1000 + (1000 - haystack.find(term).unwrap_or(0) as i32).max(0));
            }
        }
    }
    // Voor prefix/gappy gebruiken we de originele needle (niet geëxpande join)
    if needle_lower.trim().is_empty() {
        return Some(0);
    }
    let words: Vec<&str> = haystack.split_whitespace().collect();
    let tokens: Vec<&str> = needle_lower.split_whitespace().collect();
    // Prefix: elk token moet prefix van een woord zijn.
    if !tokens.is_empty()
        && tokens
            .iter()
            .all(|token| words.iter().any(|word| word.starts_with(token)))
    {
        return Some(700);
    }
    // Gappy: ordered character match op haystack met de originele needle
    let mut position: i64 = -1;
    let mut gaps: i64 = 0;
    for ch in needle_lower.chars() {
        let next = haystack
            .char_indices()
            .skip_while(|(idx, _)| *idx as i64 <= position)
            .find(|(_, c)| *c == ch)
            .map(|(idx, _)| idx as i64);
        let next = next?;
        if position >= 0 {
            gaps += next - position - 1;
        }
        position = next;
    }
    Some((500 - gaps).max(1) as i32)
}

/// Context die "dichtbij" definieert: recente sessies, lopende agents, het
/// harnas dat open staat, frecency en pinned. Ranking gebruikt dit als boost
/// bovenop de fuzzy-score — zoeken kiest wat je net aanraakte.
#[derive(Debug, Clone, Default)]
pub struct RankContext {
    /// Kleine set lowercase termen (sessie-titels, agent-namen, harnas-prefixen).
    pub boost_terms: Vec<String>,
    /// Actieve groep (sidebar-selectie) — match op section/keywords/title geeft +150.
    pub active_group: Option<String>,
    /// Frecency map: sleutel (lowercase titel/keyword) -> (count, last_used_rfc3339).
    /// Geen harde dep op `frecency.rs`; inline HashMap volstaat.
    pub frecency: HashMap<String, (u32, String)>,
}

impl RankContext {
    /// Laad de lokale frecency-store expliciet voor productie-ranking.
    /// `Default` blijft puur zodat tests en callers zonder I/O kunnen bouwen.
    pub fn local() -> Self {
        let frecency = crate::frecency::load()
            .into_iter()
            .map(|entry| (entry.id, (entry.open_count, entry.last_opened)))
            .collect();
        Self {
            frecency,
            ..Self::default()
        }
    }

    pub fn local_with_terms(boost_terms: Vec<String>) -> Self {
        Self {
            boost_terms,
            ..Self::local()
        }
    }

    /// Totale boost voor een actie: lopende agents (+150) + active_group (+150)
    /// + frecency (+60) + pinned (+80, verrekend in rank_actions_with).
    fn boost(&self, action: &Action) -> i32 {
        let mut total = 0;
        // lopende agents / boost_terms
        if !self.boost_terms.is_empty() {
            let haystack = format!(
                "{} {} {} {}",
                action.title.to_lowercase(),
                action.meta.to_lowercase(),
                action.section.to_lowercase(),
                action.keywords.to_lowercase()
            );
            if self
                .boost_terms
                .iter()
                .any(|term| !term.is_empty() && haystack.contains(term))
            {
                total += 150;
            }
        }
        // active_group
        if let Some(ref group) = self.active_group {
            let g = group.to_lowercase();
            if !g.is_empty() {
                let haystack = format!(
                    "{} {} {}",
                    action.section.to_lowercase(),
                    action.keywords.to_lowercase(),
                    action.title.to_lowercase()
                );
                if haystack.contains(&g) {
                    total += 150;
                }
            }
        }
        // frecency
        total += apply_frecency_boost(action, &self.frecency);
        total
    }
}

/// Tier-bewuste boost: boost wordt gecapped op 99 zodat een lagere tier
/// nooit een hogere tier kan overstijgen:
/// contains >=1000, prefix 700, gappy <=500+99=599.
/// Dus: gappy+99 < prefix (700) en prefix+99 < contains (1000).
fn boosted(score: i32, boost: i32) -> i32 {
    let capped = boost.clamp(0, 99);
    score + capped
}

pub fn rank_actions(actions: &[Action], query: &str, limit: usize) -> Vec<Action> {
    rank_actions_with(actions, query, limit, None)
}

pub fn rank_actions_with(
    actions: &[Action],
    query: &str,
    limit: usize,
    ctx: Option<&RankContext>,
) -> Vec<Action> {
    let mut ranked: Vec<(i32, usize, &Action)> = Vec::new();
    for (index, action) in actions.iter().enumerate() {
        if let Some(score) = fuzzy_score(query, action) {
            // Pinned +80, context boosts (running + active_group + frecency)
            let pinned = if action.pinned { 80 } else { 0 };
            let boost = ctx.map(|c| c.boost(action)).unwrap_or(0) + pinned;
            ranked.push((boosted(score, boost), index, action));
        }
    }
    ranked.sort_by(|a, b| {
        b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)) // stabiel, originele volgorde als tiebreak
    });
    ranked
        .into_iter()
        .take(limit)
        .map(|(_, _, a)| a.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::RunSpec;

    fn action(title: &str, meta: &str, keywords: &str) -> Action {
        Action {
            title: title.into(),
            meta: meta.into(),
            stamp: "STIL".into(),
            keywords: keywords.into(),
            section: "Acties".into(),
            shortcut: "↵".into(),
            needs_text: false,
            destructive: false,
            pinned: false,
            run: RunSpec::Noop,
        }
    }

    #[test]
    fn exact_phrase_wins() {
        let a = action(
            "Focus cursor agent",
            "herdr workspace",
            "focus herdr spring",
        );
        let b = action(
            "Open dashboard (Thuis)",
            "vault dashboard",
            "open dashboard thuis",
        );
        let score_a = fuzzy_score("focus", &a);
        let score_b = fuzzy_score("focus", &b);
        assert!(score_a.unwrap_or(0) > score_b.unwrap_or(0));
    }

    #[test]
    fn prefix_words_rank_above_gappy() {
        let a = action("Stuur naar agent", "herdr", "stuur send prompt");
        let b = action("Open ops", "joep-ops", "open ops overzicht");
        let score_a = fuzzy_score("stuur", &a);
        let score_b = fuzzy_score("stuur", &b);
        assert!(score_a.unwrap_or(0) > score_b.unwrap_or(0));
    }

    #[test]
    fn empty_query_matches_everything() {
        let a = action("X", "y", "");
        assert_eq!(fuzzy_score("", &a), Some(0));
    }

    #[test]
    fn rank_respects_limit_and_order() {
        let actions = vec![
            action("Alpha", "one", ""),
            action("Beta", "two", ""),
            action("Gamma", "three", ""),
        ];
        let ranked = rank_actions(&actions, "alpha", 2);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].title, "Alpha");
        let ranked2 = rank_actions(&actions, "a", 2);
        assert!(ranked2.len() <= 2);
    }

    #[test]
    fn context_boost_tilt_recente_sessie_omhoog() {
        let ver = action("Ververs status", "haal status op", "ververs refresh status");
        let deel = action(
            "Deel lokale bestanden",
            "push naar gedeelde map",
            "share sync push",
        );
        let actions = vec![ver.clone(), deel.clone()];
        let ctx = RankContext {
            boost_terms: vec!["share".into()],
            ..Default::default()
        };
        let ranked = rank_actions_with(&actions, "s", 10, Some(&ctx));
        assert_eq!(ranked[0].title, "Deel lokale bestanden");
        let plain = rank_actions(&actions, "s", 10);
        assert_eq!(plain[0].title, "Ververs status");
    }

    #[test]
    fn exact_match_wint_altijd_nog_van_boost() {
        let exact = action("Focus agent", "herdr", "focus herdr");
        let boosted = action("Fleet overzicht", "nodes", "fleet status");
        let actions = vec![boosted, exact];
        let ctx = RankContext {
            boost_terms: vec!["fleet".into()],
            ..Default::default()
        };
        let ranked = rank_actions_with(&actions, "focus", 10, Some(&ctx));
        assert_eq!(ranked[0].title, "Focus agent");
    }

    #[test]
    fn prefix_blijft_onder_contains_ondanks_boost() {
        let contains = action("fl ex dashboard", "status", "open");
        let prefix = action("fleet extra ops", "status", "run");
        assert!(fuzzy_score("fl ex", &contains).unwrap() >= 1000);
        assert_eq!(fuzzy_score("fl ex", &prefix), Some(700));
        let actions = vec![prefix, contains];
        let ctx = RankContext {
            boost_terms: vec!["fleet".into()],
            ..Default::default()
        };
        let ranked = rank_actions_with(&actions, "fl ex", 10, Some(&ctx));
        assert_eq!(ranked[0].title, "fl ex dashboard");
    }

    #[test]
    fn pinned_zweeft_boven_onpinned() {
        let mut pinned_a = action("Zzz laatste", "meta", "zzz");
        pinned_a.pinned = true;
        let plain_b = action("Zzz eerste", "meta", "zzz");
        let ranked = rank_actions(&[plain_b, pinned_a], "zzz", 10);
        assert_eq!(ranked[0].title, "Zzz laatste");
    }

    // ---- Nieuwe Lane D tests (tier-invariant, alias, frecency, determinisme) ----

    #[test]
    fn tier_invariant_contains_nooit_onder_prefix_met_max_boost() {
        // contains ("fl ex" als aaneengesloten substring) vs prefix (fl→fleet, ex→extra)
        // zelfs met alle boosts op prefix mag contains nooit verliezen.
        let contains = action("fl ex dashboard", "status", "open");
        let mut prefix = action("fleet extra ops", "status", "run fleet extra");
        // geef prefix alle boosts
        prefix.pinned = true;
        let mut frecency = HashMap::new();
        frecency.insert("fleet".into(), (5, format_now_rfc3339()));
        let ctx = RankContext {
            boost_terms: vec!["fleet".into()],
            active_group: Some("fleet".into()),
            frecency,
        };
        let ctx_empty = RankContext::default();
        let score_contains = fuzzy_score("fl ex", &contains).unwrap();
        let score_prefix = fuzzy_score("fl ex", &prefix).unwrap();
        assert!(
            score_contains >= 1000,
            "contains tier >=1000 got {score_contains}"
        );
        assert_eq!(score_prefix, 700);
        // boosted: prefix krijgt max boost capped 99 -> 799, contains 1000+ -> >=1000
        let boosted_contains = boosted(
            score_contains,
            ctx_empty.boost(&contains) + if contains.pinned { 80 } else { 0 },
        );
        let boosted_prefix = boosted(score_prefix, ctx.boost(&prefix) + 80);
        assert!(
            boosted_contains > boosted_prefix,
            "tier invariant broken: contains {boosted_contains} vs prefix {boosted_prefix}"
        );
        let actions = vec![prefix, contains.clone()];
        let ranked = rank_actions_with(&actions, "fl ex", 10, Some(&ctx));
        assert_eq!(ranked[0].title, "fl ex dashboard");
    }

    #[test]
    fn tier_invariant_prefix_nooit_onder_gappy_met_boost() {
        // prefix 700 vs gappy ~500: gappy met max boost mag nooit boven prefix komen.
        // "stu pro" is prefix voor "Stuur prompt" (tokens stu→stuur, pro→prompt) maar
        // niet als aaneengesloten substring, dus tier 700. Voor gappy-actie is het gappy
        // (ordered chars verspreid, geen prefix-match, wel gappy-match).
        let prefix = action("Stuur prompt", "herdr", "stuur prompt");
        let gappy = action(
            "Setup tunnel report overview",
            "meta",
            "setup tunnel report",
        );
        let score_prefix = fuzzy_score("stu pro", &prefix).unwrap();
        let score_gappy = fuzzy_score("stu pro", &gappy).unwrap();
        assert_eq!(
            score_prefix, 700,
            "prefix tier verwacht voor stu pro -> stuur prompt"
        );
        assert!(
            (1..700).contains(&score_gappy),
            "gappy score {score_gappy:?} moet 1..699 zijn"
        );
        let mut frecency = HashMap::new();
        frecency.insert("setup".into(), (3, format_now_rfc3339()));
        let ctx_gappy = RankContext {
            boost_terms: vec!["setup".into()],
            active_group: Some("setup".into()),
            frecency,
        };
        let mut gappy_pinned = gappy.clone();
        gappy_pinned.pinned = true;
        let boosted_prefix = boosted(score_prefix, 0);
        let boosted_gappy = boosted(score_gappy, ctx_gappy.boost(&gappy_pinned) + 80);
        assert!(
            boosted_prefix > boosted_gappy,
            "prefix {boosted_prefix} moet boven gappy {boosted_gappy} blijven"
        );
    }

    #[test]
    fn alias_expansion_cfg_raakt_config() {
        // query "cfg" moet via alias "config" matchen op een config-action
        let cfg_action = action("Open configuratie", "instellingen", "config configuratie");
        let other = action("Open dashboard", "overzicht", "dashboard open");
        // zonder alias zou "cfg" alleen gappy matchen op config (s c h r i...), met alias wordt het contains via "config"
        let score_cfg = fuzzy_score("cfg", &cfg_action).unwrap();
        let score_other = fuzzy_score("cfg", &other).unwrap_or(0);
        assert!(
            score_cfg > score_other,
            "alias expansion: cfg score {score_cfg} moet > other {score_other}"
        );
        assert!(
            score_cfg >= 1000,
            "cfg via alias config moet contains-tier halen, kreeg {score_cfg}"
        );
        // ook via rank
        let actions = vec![other, cfg_action];
        let ranked = rank_actions(&actions, "cfg", 10);
        assert_eq!(ranked[0].title, "Open configuratie");
        // reverse: "config" moet ook "cfg" alias raken als iemand "config" zoekt maar actie "cfg" keyword heeft?
        let cfg_keyword_action = action("cfg shortcut", "meta", "cfg");
        let score_rev = fuzzy_score("config", &cfg_keyword_action).unwrap();
        assert!(
            score_rev >= 1000,
            "reverse alias: config -> cfg kreeg {score_rev}"
        );
    }

    #[test]
    fn frecency_boost_recent_geeft_plus60_oud_geeft_nul() {
        let action_a = action("Fleet infra overzicht", "infra", "fleet infra");
        // recent frecency
        let mut recent_map = HashMap::new();
        recent_map.insert("fleet".into(), (3, format_now_rfc3339()));
        let boost_recent = apply_frecency_boost(&action_a, &recent_map);
        assert_eq!(boost_recent, 60);
        let mut id_map = HashMap::new();
        id_map.insert(action_a.frecency_id(), (1, format_now_rfc3339()));
        assert_eq!(apply_frecency_boost(&action_a, &id_map), 60);
        // oud frecency (>24u) — 2020
        let mut old_map = HashMap::new();
        old_map.insert("fleet".into(), (3, "2020-01-01T00:00:00Z".into()));
        let boost_old = apply_frecency_boost(&action_a, &old_map);
        assert_eq!(boost_old, 0);
        // geen match
        let mut nomatch = HashMap::new();
        nomatch.insert("unknown".into(), (3, format_now_rfc3339()));
        assert_eq!(apply_frecency_boost(&action_a, &nomatch), 0);
        // ranking: met recent frecency wint fleet boven andere gappy
        let other = action("Ververs status", "haal op", "ververs");
        let actions = vec![other.clone(), action_a.clone()];
        let ctx = RankContext {
            frecency: recent_map,
            ..Default::default()
        };
        let ranked = rank_actions_with(&actions, "f", 10, Some(&ctx));
        assert_eq!(ranked[0].title, "Fleet infra overzicht");
    }

    #[test]
    fn active_group_boost_tilt_binnen_tier() {
        let a = action("Inbox overzicht", "postvak", "inbox postvak");
        let mut b = action("Fleet overzicht", "infra", "fleet infra");
        b.section = "Fleet".into();
        let actions = vec![a.clone(), b.clone()];
        // query "overzicht" geeft beide contains-tier (gelijke base)
        let score_a = fuzzy_score("overzicht", &a).unwrap();
        let score_b = fuzzy_score("overzicht", &b).unwrap();
        assert!(score_a >= 1000 && score_b >= 1000);
        // zonder group: stabiele volgorde wint (a eerst)
        let plain = rank_actions(&actions, "overzicht", 10);
        assert_eq!(plain[0].title, "Inbox overzicht");
        // met active_group Fleet: b krijgt +150 (capped 99) en wint binnen tier
        let ctx = RankContext {
            active_group: Some("Fleet".into()),
            ..Default::default()
        };
        let ranked = rank_actions_with(&actions, "overzicht", 10, Some(&ctx));
        assert_eq!(ranked[0].title, "Fleet overzicht");
    }

    #[test]
    fn determinisme_rank_is_stabiel() {
        let actions = vec![
            action("Alpha", "one", "alpha"),
            action("Alfa copy", "one", "alpha"),
            action("Beta", "two", "beta"),
        ];
        let ranked1 = rank_actions(&actions, "alpha", 10);
        let ranked2 = rank_actions(&actions, "alpha", 10);
        assert_eq!(
            ranked1.iter().map(|a| &a.title).collect::<Vec<_>>(),
            ranked2.iter().map(|a| &a.title).collect::<Vec<_>>()
        );
        // met context ook deterministisch
        let ctx = RankContext {
            boost_terms: vec!["alpha".into()],
            active_group: Some("Acties".into()),
            frecency: {
                let mut m = HashMap::new();
                m.insert("alpha".into(), (1, format_now_rfc3339()));
                m
            },
        };
        let r1 = rank_actions_with(&actions, "alpha", 10, Some(&ctx));
        let r2 = rank_actions_with(&actions, "alpha", 10, Some(&ctx));
        assert_eq!(
            r1.iter().map(|a| &a.title).collect::<Vec<_>>(),
            r2.iter().map(|a| &a.title).collect::<Vec<_>>()
        );
    }

    #[test]
    fn frecency_boost_via_rankcontext_wordt_meegenomen() {
        let act = action("Dashboard openen", "overzicht", "dashboard");
        let mut map = HashMap::new();
        map.insert("dashboard".into(), (2, format_now_rfc3339()));
        let ctx = RankContext {
            frecency: map,
            ..Default::default()
        };
        let actions = vec![act.clone(), action("Andere actie", "meta", "andere")];
        let ranked = rank_actions_with(&actions, "dash", 10, Some(&ctx));
        assert_eq!(ranked[0].title, "Dashboard openen");
    }
}
