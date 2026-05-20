//! HTTP client for chainup `relayer-service` (Safe meta-tx submission).
//!
//! Constructed via [`crate::Client::relayer`]. Authentication is by JWT Bearer token —
//! attach via [`RelayerClient::with_token`] (or use [`crate::Client::jwt_login`] to fetch
//! one and have it auto-attached). For the alternative API-Key flow, use
//! [`RelayerClient::with_api_key`].

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use reqwest::{Client as HttpClient, Method};
use url::Url;

use crate::error::{Error, Result};
use crate::relayer::types::{RelayerTransaction, SubmitRequest, SubmitResponse};

/// Sub-client for the chainup `relayer-service`. Shares the underlying [`reqwest::Client`]
/// with the parent [`crate::Client`] for connection pooling.
#[derive(Clone, Debug)]
pub struct RelayerClient {
    http: HttpClient,
    base: Url,
    /// Authorization header value (`Bearer <jwt>` or `RELAYER_API_KEY ...`). The relayer
    /// rejects unauthenticated `/submit` requests.
    auth_headers: HeaderMap,
}

impl RelayerClient {
    /// Construct directly. Most callers should use [`crate::Client::relayer`] instead.
    #[must_use]
    pub fn new(http: HttpClient, base: Url) -> Self {
        Self {
            http,
            base,
            auth_headers: HeaderMap::new(),
        }
    }

    /// Base URL of this client (e.g. `https://relayer-api.hermestrade.xyz/`).
    #[must_use]
    pub fn base(&self) -> &Url {
        &self.base
    }

    /// Attach a JWT for `Authorization: Bearer <jwt>`. Replaces any previous credential.
    /// The JWT comes from [`crate::Client::jwt_login`] (i.e. gamma-service `/auth/login`).
    #[must_use]
    pub fn with_token(mut self, jwt: &str) -> Self {
        let mut headers = HeaderMap::new();
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {jwt}")) {
            headers.insert(AUTHORIZATION, val);
        }
        self.auth_headers = headers;
        self
    }

    /// Attach API-Key credentials (alternative to JWT). The relayer accepts a custom
    /// header pair `RELAYER_API_KEY` + `RELAYER_API_KEY_ADDRESS` for server-side ops.
    #[must_use]
    pub fn with_api_key(mut self, key: &str, address: &str) -> Self {
        let mut headers = HeaderMap::new();
        if let Ok(val) = HeaderValue::from_str(key)
            && let Ok(name) = reqwest::header::HeaderName::from_bytes(b"RELAYER_API_KEY")
        {
            headers.insert(name, val);
        }
        if let Ok(val) = HeaderValue::from_str(address)
            && let Ok(name) = reqwest::header::HeaderName::from_bytes(b"RELAYER_API_KEY_ADDRESS")
        {
            headers.insert(name, val);
        }
        self.auth_headers = headers;
        self
    }

    /// `POST /submit` — submit a Safe meta-tx (single op or MultiSend). Returns the
    /// `transactionID` for polling immediately; the relayer broadcasts asynchronously.
    pub async fn submit(&self, req: &SubmitRequest) -> Result<SubmitResponse> {
        self.post_json("submit", req).await
    }

    /// `GET /transaction?id=<tx_id>` — fetch the current lifecycle state. Use
    /// [`TransactionState::is_terminal`] to know when to stop polling.
    pub async fn transaction(&self, tx_id: &str) -> Result<RelayerTransaction> {
        let url = self.url("transaction")?;
        let mut url = url;
        url.set_query(Some(&format!("id={}", urlencoding(tx_id))));
        self.get_json_url(url).await
    }

    /// Poll `transaction` at the given interval until the state is terminal or the timeout
    /// elapses. Returns the last observed state.
    pub async fn poll_until_terminal(
        &self,
        tx_id: &str,
        interval: std::time::Duration,
        timeout: std::time::Duration,
    ) -> Result<RelayerTransaction> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let tx = self.transaction(tx_id).await?;
            if tx.state.is_terminal() {
                return Ok(tx);
            }
            if tokio::time::Instant::now() >= deadline {
                return Ok(tx);
            }
            tokio::time::sleep(interval).await;
        }
    }

    // ─── HTTP helpers ──────────────────────────────────────────────────────

    fn url(&self, path: &str) -> Result<Url> {
        let p = path.trim_start_matches('/');
        Ok(self.base.join(p)?)
    }

    async fn post_json<Q, R>(&self, path: &str, body: &Q) -> Result<R>
    where
        Q: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let url = self.url(path)?;
        let mut req = self.http.request(Method::POST, url).json(body);
        for (k, v) in &self.auth_headers {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let bytes = resp.bytes().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::api(
                status,
                "POST",
                path,
                String::from_utf8_lossy(&bytes).into_owned(),
            ));
        }
        serde_json::from_slice(&bytes)
            .map_err(|e| Error::Validation(format!("decoding {path}: {e}")))
    }

    async fn get_json_url<R>(&self, url: Url) -> Result<R>
    where
        R: serde::de::DeserializeOwned,
    {
        let mut req = self.http.request(Method::GET, url.clone());
        for (k, v) in &self.auth_headers {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let bytes = resp.bytes().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::api(
                status,
                "GET",
                url.path(),
                String::from_utf8_lossy(&bytes).into_owned(),
            ));
        }
        serde_json::from_slice(&bytes)
            .map_err(|e| Error::Validation(format!("decoding {}: {e}", url.path())))
    }
}

/// Minimal URL-encoding for query-string values. Sufficient for relayer tx ids (UUIDs).
fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}
