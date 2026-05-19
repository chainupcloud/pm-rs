//! Connection configuration for [`crate::ws::WsConnection`].

use std::time::Duration;

/// Tunable knobs for a single WebSocket connection / reconnect loop.
///
/// Defaults match the chainup `clob-service` server (`pingPeriod = 10s`,
/// `pongWait = 15s` — see `services/clob-service/internal/wsservice/market_channel.go`)
/// and the `pm-sdk-go` reference implementation (`maxBackoff = 30s`).
#[derive(Clone, Debug)]
pub struct WsConfig {
    /// Heartbeat interval. The client sends the text frame `"PING"` every
    /// `ping_interval` and the server replies with `"PONG"`. Set to
    /// [`Duration::ZERO`] to disable heartbeats (not recommended — the server
    /// closes the socket if no frame arrives within `pongWait`).
    pub ping_interval: Duration,
    /// Initial backoff for the first reconnect attempt. Subsequent attempts
    /// double the wait up to [`Self::max_backoff`].
    pub initial_backoff: Duration,
    /// Backoff cap for repeated reconnect attempts.
    pub max_backoff: Duration,
    /// Channel buffer used between the read task and the public stream. A
    /// larger buffer absorbs bursts at the cost of memory.
    pub channel_capacity: usize,
    /// Per-connection dial timeout. Reaching this aborts the dial and the
    /// reconnect loop schedules the next attempt.
    pub connect_timeout: Duration,
    /// If true, [`crate::ws::WsConnection`] surfaces a `WsEvent::Reconnecting`
    /// before every reconnect attempt. Useful for higher layers that want to
    /// invalidate caches.
    pub emit_reconnecting: bool,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            ping_interval: Duration::from_secs(10),
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
            channel_capacity: 256,
            connect_timeout: Duration::from_secs(10),
            emit_reconnecting: true,
        }
    }
}

impl WsConfig {
    /// Convenience: build a config with a custom ping interval.
    #[must_use]
    pub fn with_ping_interval(mut self, interval: Duration) -> Self {
        self.ping_interval = interval;
        self
    }

    /// Convenience: cap the reconnect backoff at `cap`.
    #[must_use]
    pub fn with_max_backoff(mut self, cap: Duration) -> Self {
        self.max_backoff = cap;
        self
    }

    /// Convenience: set the dial timeout.
    #[must_use]
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }
}
