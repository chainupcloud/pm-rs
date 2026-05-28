//! Generic WebSocket infrastructure shared by the CLOB channels.
//!
//! This module wraps [`tokio_tungstenite`] with three specific behaviors:
//!
//! 1. Text-only framing — both the market and user channels exchange JSON-text
//!    frames (and the literal text strings `"PING"` / `"PONG"`). Binary frames
//!    are dropped with a warning.
//! 2. A heartbeat loop sending the **text** `"PING"` payload every
//!    [`WsConfig::ping_interval`] (default 10 s, matching the server's PONG
//!    response cadence — see `services/clob-service/internal/wsservice/`).
//! 3. Exponential backoff reconnect — the higher-level
//!    [`crate::clob::ws::ClobWebSocketClient`] uses this to re-emit any active
//!    subscriptions after reconnecting.
//!
//! The CLOB-specific subscription mechanics live in [`crate::clob::ws`].

pub mod config;
pub mod connection;
pub mod error;

pub use config::WsConfig;
pub use connection::{WsConnection, WsEvent};
pub use error::WsError;
