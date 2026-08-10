//! HTTP-client met policy-gates: geen redirects, alleen toegestane origins.

use crate::auth;
use crate::policy::EndpointPolicy;
use std::time::Duration;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct Client {
    base: String,
    policy: EndpointPolicy,
    timeout: Duration,
}

impl Client {
    pub fn new(base: &str, policy: EndpointPolicy) -> Self {
        Self {
            base: base.trim_end_matches('/').to_string(),
            policy,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    fn url(&self, path: &str) -> Result<String, String> {
        let url_path = if path.starts_with('/') { path.to_string() } else { format!("/{path}") };
        self.policy.safe_join(&self.base, &url_path)
    }

    fn agent(&self) -> ureq::Agent {
        ureq::AgentBuilder::new()
            .timeout(self.timeout)
            .redirects(0) // bearer-tokens volgen nooit redirects
            .build()
    }

    pub fn get_json(&self, path: &str) -> Result<serde_json::Value, ApiError> {
        let url = self.url(path)?;
        let agent = self.agent();
        let mut request = agent.get(&url);
        for (name, value) in auth::get_headers(false) {
            request = request.set(&name, &value);
        }
        run(request.call())
    }

    pub fn post_json(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value, ApiError> {
        self.post_json_headers(path, body, &[])
    }

    pub fn post_json_headers(
        &self,
        path: &str,
        body: &serde_json::Value,
        extra: &[(String, String)],
    ) -> Result<serde_json::Value, ApiError> {
        let url = self.url(path)?;
        let agent = self.agent();
        let mut request = agent.post(&url);
        for (name, value) in auth::get_headers(true) {
            request = request.set(&name, &value);
        }
        for (name, value) in extra {
            request = request.set(name, value);
        }
        run(request.send_json(body))
    }

    pub fn delete_json(&self, path: &str) -> Result<serde_json::Value, ApiError> {
        let url = self.url(path)?;
        let agent = self.agent();
        let mut request = agent.delete(&url);
        for (name, value) in auth::get_headers(false) {
            request = request.set(&name, &value);
        }
        run(request.call())
    }
}

#[derive(Debug)]
pub enum ApiError {
    Blocked(String),
    Http(u16, String),
    Transport(String),
}

impl From<String> for ApiError {
    fn from(reason: String) -> Self {
        ApiError::Blocked(reason)
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Blocked(msg) => write!(f, "geblokkeerd: {msg}"),
            ApiError::Http(code, detail) => write!(f, "HTTP {code}: {detail}"),
            ApiError::Transport(msg) => write!(f, "{msg}"),
        }
    }
}

fn run(response: Result<ureq::Response, ureq::Error>) -> Result<serde_json::Value, ApiError> {
    match response {
        Ok(resp) => resp
            .into_json()
            .map_err(|err| ApiError::Transport(format!("JSON-parse faalde: {err}"))),
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            let detail = body.chars().take(200).collect::<String>();
            Err(ApiError::Http(code, detail))
        }
        Err(ureq::Error::Transport(err)) => Err(ApiError::Transport(err.to_string())),
    }
}