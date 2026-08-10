//! Pure command-palette fuzzy ranking voor ChefBar.

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
}

/// Rank ordered-character matches; exact words en prefixes winnen.
pub fn fuzzy_score(query: &str, action: &Action) -> Option<i32> {
    let needle: String = query.split_whitespace().collect::<Vec<_>>().join(" ");
    if needle.is_empty() {
        return Some(0);
    }
    let haystack = format!(
        "{} {} {} {}",
        action.title.to_lowercase(),
        action.meta.to_lowercase(),
        action.section.to_lowercase(),
        action.keywords.to_lowercase()
    );
    let needle = needle.to_lowercase();
    if haystack.contains(&needle) {
        return Some(1000 - haystack.find(&needle).unwrap_or(0) as i32);
    }
    let words: Vec<&str> = haystack.split_whitespace().collect();
    let tokens: Vec<&str> = needle.split_whitespace().collect();
    if tokens
        .iter()
        .all(|token| words.iter().any(|word| word.starts_with(token)))
    {
        return Some(700);
    }
    let mut position: i64 = -1;
    let mut gaps: i64 = 0;
    for ch in needle.chars() {
        let next = haystack
            .char_indices()
            .skip_while(|(idx, _)| *idx as i64 <= position)
            .find(|(_, c)| *c == ch)
            .map(|(idx, _)| idx as i64);
        let Some(next) = next else {
            return None;
        };
        if position >= 0 {
            gaps += next - position - 1;
        }
        position = next;
    }
    Some((500 - gaps).max(1) as i32)
}

pub fn rank_actions(actions: &[Action], query: &str, limit: usize) -> Vec<Action> {
    let mut ranked: Vec<(i32, usize, &Action)> = Vec::new();
    for (index, action) in actions.iter().enumerate() {
        if let Some(score) = fuzzy_score(query, action) {
            ranked.push((score, index, action));
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
        // limiet snijdt ook bij breed matchen op een meer generieke query
        let ranked2 = rank_actions(&actions, "a", 2);
        assert!(ranked2.len() <= 2);
    }
}
