//! `data-service` REST client (portfolio / trades / activity / leaderboards).
//!
//! `data-service` is a separate microservice from CLOB / Gamma; it lives at
//! `data-api.<tenant>` (e.g. `https://data-api.hermestrade.xyz`). Construct a
//! [`DataClient`] from a parent [`crate::Client`] via [`crate::Client::data`].
//!
//! Counterpart of polymarket's `data-api.polymarket.com`. Field-level divergences (i18n
//! fields, wrapped leaderboard envelope, extra `fee` field on `/trades`) are documented in
//! `pm-cup2026/services/data-service/docs/polymarket-divergence-register.md` — they are
//! intentional product decisions, not bugs.

pub mod client;
pub mod types;

pub use client::DataClient;
