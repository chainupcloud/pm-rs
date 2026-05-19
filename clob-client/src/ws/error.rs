//! Error type for the generic [`crate::ws`] layer.

use thiserror::Error;

/// Errors surfaced by [`crate::ws::WsConnection`] and the CLOB-specific clients
/// that wrap it.
///
/// Auth errors are intentionally **never** swallowed — see [`Self::Auth`].
#[derive(Debug, Error)]
pub enum WsError {
    /// Failure during the WebSocket upgrade or dial step. Includes URL-parse
    /// errors and TLS handshake failures.
    #[error("connect failed: {0}")]
    Connect(String),

    /// Underlying tungstenite transport error (frame parse, IO, etc.).
    #[error("transport: {0}")]
    Transport(String),

    /// Server replied to a `/ws/user` upgrade with an HTTP error status
    /// (most commonly 401 / 403). Carries the status code so callers can
    /// distinguish auth failures from transient network errors and bail out
    /// rather than reconnecting.
    #[error("auth rejected (http {status}): {message}")]
    Auth { status: u16, message: String },

    /// Server closed the user-channel socket immediately after the first
    /// frame with an error envelope (`{"error":"authentication failed"}` —
    /// see `wsservice/user_channel.go`). Reconnect loops must NOT swallow
    /// this case; the higher-level client treats it as terminal.
    #[error("user-channel authentication failed: {0}")]
    UserAuthRejected(String),

    /// JSON encode / decode failure on a wire frame.
    #[error("frame decode: {0}")]
    Decode(String),

    /// The connection task panicked or the heartbeat ticker outlived the
    /// reader — internal bug.
    #[error("internal: {0}")]
    Internal(String),

    /// User explicitly cancelled the subscription (Ctrl-C / drop).
    #[error("cancelled")]
    Cancelled,
}

impl WsError {
    /// True for errors that should bail out of the reconnect loop. Anything
    /// transient (transport / connect) is reconnectable.
    #[must_use]
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::Auth { .. } | Self::UserAuthRejected(_) | Self::Cancelled,
        )
    }
}

impl From<url::ParseError> for WsError {
    fn from(e: url::ParseError) -> Self {
        Self::Connect(format!("url parse: {e}"))
    }
}

impl From<serde_json::Error> for WsError {
    fn from(e: serde_json::Error) -> Self {
        Self::Decode(e.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for WsError {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        // tungstenite::Error::Http carries the failed upgrade response.
        if let tokio_tungstenite::tungstenite::Error::Http(resp) = &e {
            let status = resp.status().as_u16();
            let body = resp
                .body()
                .as_ref()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default();
            return Self::Auth { status, message: body };
        }
        Self::Transport(e.to_string())
    }
}

impl From<WsError> for crate::error::Error {
    fn from(e: WsError) -> Self {
        crate::error::Error::Validation(format!("websocket: {e}"))
    }
}
