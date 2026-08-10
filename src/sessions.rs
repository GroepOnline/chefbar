//! CHEF-shaped injectable sessions — één implementatie (geen duplicaten).
//!
//! Een session is een aanhechtbaar live werkobject, geen inbox-item. ChefBar
//! toont het contextueel en geeft controle door aan Kater, herdr, browser of
//! evidence.

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionActionKind {
    None_,
    Kater,
    Focus,
    Workspace,
    Browser,
    Evidence,
}

#[derive(Debug, Clone, Default)]
pub struct AttachPoints {
    pub focus: Option<String>,
    pub browser: Option<String>,
    pub workspace_url: Option<String>,
    pub kater_session_id: Option<String>,
    pub evidence_url: Option<String>,
}

impl AttachPoints {
    pub fn from_value(raw: Option<&Value>) -> Self {
        let raw = raw.unwrap_or(&Value::Null);
        let get = |key: &str| raw.get(key).and_then(|v| v.as_str()).map(String::from);
        Self {
            focus: get("focus"),
            browser: get("browser"),
            workspace_url: get("workspaceUrl"),
            kater_session_id: get("katerSessionId"),
            evidence_url: get("evidenceUrl"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub source: String,
    pub state: String,
    pub title: String,
    pub summary: String,
    pub updated_at: Option<String>,
    pub attach: AttachPoints,
}

impl Session {
    pub fn from_value(raw: &Value) -> Option<Self> {
        // payload-variant (connector events) of direct variant (/api/sessions)
        let payload = raw.get("payload").filter(|p| p.is_object()).unwrap_or(raw);
        let id = payload
            .get("id")
            .or_else(|| raw.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() {
            return None;
        }
        let get = |key: &str| {
            payload
                .get(key)
                .or_else(|| raw.get(key))
                .and_then(|v| v.as_str())
        };
        let source = get("source").unwrap_or("").to_lowercase();
        let state = get("state").unwrap_or("").to_lowercase();
        if source.is_empty() || state.is_empty() {
            return None;
        }
        let title = get("title").unwrap_or(&id).to_string();
        let summary = get("summary").unwrap_or("").to_string();
        let updated_at = get("updatedAt")
            .or_else(|| get("ts"))
            .map(String::from);
        let attach = AttachPoints::from_value(payload.get("attach"));
        Some(Self {
            id,
            source,
            state,
            title,
            summary,
            updated_at,
            attach,
        })
    }

    pub fn needs_attention(&self) -> bool {
        matches!(
            self.state.as_str(),
            "waiting" | "blocked" | "failed"
        )
    }

    pub fn primary_action(&self) -> SessionActionKind {
        if self.attach.kater_session_id.is_some() {
            SessionActionKind::Kater
        } else if self.attach.focus.is_some() {
            SessionActionKind::Focus
        } else if self.attach.workspace_url.is_some() {
            SessionActionKind::Workspace
        } else if self.attach.browser.is_some() {
            SessionActionKind::Browser
        } else if self.attach.evidence_url.is_some() {
            SessionActionKind::Evidence
        } else {
            SessionActionKind::None_
        }
    }
}

fn rank_priority(session: &Session) -> u8 {
    match session.state.as_str() {
        "blocked" => 0,
        "waiting" => 1,
        "failed" => 2,
        "working" => 3,
        "starting" => 4,
        "done" => 5,
        _ => 9,
    }
}

/// Sessions uit de vault-connector/API-feed, aandacht eerst.
pub fn load_ranked_sessions(events: &[Value]) -> Vec<Session> {
    let mut sessions: Vec<Session> = Vec::new();
    for raw in events {
        if let Some(session) = Session::from_value(raw) {
            sessions.push(session);
        }
    }
    sessions.sort_by_key(|s| (rank_priority(s), s.title.to_lowercase()));
    sessions.truncate(6);
    sessions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kater_is_preferred_attach_action() {
        let session = Session::from_value(&serde_json::json!({
            "id": "ses-1",
            "source": "opencodex",
            "state": "waiting",
            "title": "Review UDO",
            "attach": {
                "focus": "pane-1",
                "katerSessionId": "kater-42",
                "evidenceUrl": "https://kater.chefgroep.online/evidence/42"
            }
        }))
        .unwrap();
        assert!(session.needs_attention());
        assert_eq!(session.primary_action(), SessionActionKind::Kater);
    }

    #[test]
    fn ranked_sessions_prefer_attention() {
        let events = vec![
            serde_json::json!({"id": "a", "source": "kater", "state": "working", "title": "Busy"}),
            serde_json::json!({"id": "b", "source": "kater", "state": "waiting", "title": "Needs you"}),
        ];
        let ranked = load_ranked_sessions(&events);
        assert_eq!(ranked[0].id, "b");
        assert_eq!(ranked[1].id, "a");
    }

    #[test]
    fn connector_event_payload_is_parsed() {
        let event = serde_json::json!({
            "kind": "session",
            "payload": {"id": "s9", "source": "kater", "state": "working", "title": "X"}
        });
        let session = Session::from_value(&event).unwrap();
        assert_eq!(session.id, "s9");
        assert_eq!(session.source, "kater");
    }
}