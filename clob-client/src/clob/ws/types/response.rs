//! Inbound event payloads pushed by the chainup CLOB WebSocket server.
//!
//! Two top-level enums correspond to the two channels:
//!
//! - [`MarketEvent`] — variants for `book` / `price_change` /
//!   `last_trade_price` / `tick_size_change` / `best_bid_ask` / `new_market` /
//!   `market_resolved`. The last three are only pushed when the subscriber
//!   set `custom_feature_enabled = true` on the initial subscribe envelope.
//! - [`UserEvent`] — variants for `order` and `trade`. Both are auth-scoped
//!   to the API-key owner.
//!
//! Field names match the AsyncAPI byte-for-byte; integer timestamps use
//! [`Timestamp`] which transparently accepts JSON numbers and quoted-string
//! values (both are seen in the wild — see `pm-sdk-go pkg/ws/types.go`).

use serde::{Deserialize, Serialize};

/// Timestamp wrapper accepting both `1700000000` and `"1700000000"`.
///
/// The chainup server marshals timestamps as JSON numbers, but a handful of
/// MM-lazy paths (and downstream relays) re-emit them quoted; both flavours
/// must round-trip cleanly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Timestamp(pub i64);

impl Timestamp {
    #[must_use]
    pub const fn new(v: i64) -> Self {
        Self(v)
    }

    #[must_use]
    pub const fn as_i64(self) -> i64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Timestamp;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("integer or string-encoded integer timestamp")
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Timestamp, E> {
                Ok(Timestamp(v))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Timestamp, E> {
                Ok(Timestamp(v as i64))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Timestamp, E> {
                if v.is_empty() {
                    return Ok(Timestamp(0));
                }
                if let Ok(n) = v.parse::<i64>() {
                    return Ok(Timestamp(n));
                }
                // chainup WS occasionally emits RFC3339 (e.g. `2026-05-19T19:17:39Z`).
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(v) {
                    return Ok(Timestamp(dt.timestamp()));
                }
                Err(E::custom(format!("invalid timestamp '{v}': not an integer or RFC3339 string")))
            }
        }
        d.deserialize_any(Visitor)
    }
}

/// `OrderLevel` schema entry shared by `book` snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrderLevel {
    pub price: String,
    pub size: String,
}

/// Side enum used by every event that carries one. The chainup wire format
/// uses uppercase `BUY` / `SELL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Top-level inbound enum for the `/ws/market` channel. Dispatch is keyed on
/// `event_type`; the actual payload sits inside the wire `data: {...}` field — the
/// asyncapi spec shows a flat shape, but production chainup nests it. We use serde's
/// `tag + content` adjacent encoding to read the nested form. Top-level `asset_id`
/// (which the server echoes outside `data` on book / price-change frames) is ignored
/// because the same value is repeated inside the nested payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event_type", content = "data", rename_all = "snake_case")]
pub enum MarketEvent {
    Book(BookEvent),
    PriceChange(PriceChangeEvent),
    LastTradePrice(LastTradePriceEvent),
    TickSizeChange(TickSizeChangeEvent),
    BestBidAsk(BestBidAskEvent),
    NewMarket(NewMarketEvent),
    MarketResolved(MarketResolvedEvent),
}

/// `book` event payload (`asyncapi-market.json::components.messages.book`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BookEvent {
    pub asset_id: String,
    pub market: String,
    #[serde(default)]
    pub bids: Vec<OrderLevel>,
    #[serde(default)]
    pub asks: Vec<OrderLevel>,
    #[serde(default)]
    pub timestamp: Timestamp,
    #[serde(default)]
    pub hash: String,
}

/// `price_change` event payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriceChangeEvent {
    pub market: String,
    #[serde(default)]
    pub price_changes: Vec<PriceChangeEntry>,
    #[serde(default)]
    pub timestamp: Timestamp,
}

/// Single entry inside [`PriceChangeEvent::price_changes`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriceChangeEntry {
    pub asset_id: String,
    pub price: String,
    /// `"0"` indicates that price level was removed.
    pub size: String,
    pub side: OrderSide,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub best_bid: String,
    #[serde(default)]
    pub best_ask: String,
}

/// `last_trade_price` event payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LastTradePriceEvent {
    pub asset_id: String,
    pub market: String,
    pub price: String,
    pub size: String,
    #[serde(default)]
    pub fee_rate_bps: String,
    pub side: OrderSide,
    #[serde(default)]
    pub timestamp: Timestamp,
    /// On-chain settlement tx hash; empty string for synthetic trades pushed
    /// via the internal `POST /self-trade` endpoint.
    #[serde(default)]
    pub transaction_hash: String,
}

/// `tick_size_change` event payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TickSizeChangeEvent {
    pub asset_id: String,
    pub market: String,
    pub old_tick_size: String,
    pub new_tick_size: String,
    #[serde(default)]
    pub timestamp: Timestamp,
}

/// `best_bid_ask` event payload (requires `custom_feature_enabled`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BestBidAskEvent {
    pub asset_id: String,
    pub market: String,
    pub best_bid: String,
    pub best_ask: String,
    #[serde(default)]
    pub spread: String,
    #[serde(default)]
    pub timestamp: Timestamp,
}

/// `new_market` event payload (requires `custom_feature_enabled`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NewMarketEvent {
    pub id: String,
    pub question: String,
    pub market: String,
    pub slug: String,
    #[serde(default, rename = "assets_ids")]
    pub assets_ids: Vec<String>,
    #[serde(default)]
    pub outcomes: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub timestamp: Timestamp,
}

/// `market_resolved` event payload (requires `custom_feature_enabled`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketResolvedEvent {
    pub id: String,
    pub market: String,
    #[serde(default, rename = "assets_ids")]
    pub assets_ids: Vec<String>,
    pub winning_asset_id: String,
    pub winning_outcome: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub timestamp: Timestamp,
}

// ─── user channel ───────────────────────────────────────────────────────────

/// Top-level inbound enum for the `/ws/user` channel. Like [`MarketEvent`], live chainup
/// nests the per-event payload inside `data: {...}` (with `owner` and `condition_id`
/// echoed at the top level alongside `event_type`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event_type", content = "data", rename_all = "snake_case")]
pub enum UserEvent {
    Order(OrderEvent),
    Trade(TradeEvent),
}

/// Sub-type of an order event (the `type` field on the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderSubType {
    Placement,
    Update,
    Cancellation,
}

/// Order state transitions. The REST `/orders` endpoint uses the long prefixed form
/// (`ORDER_STATUS_LIVE`); the `/ws/user` channel uses the short lowercase form (`live`).
/// Some live frames switch to short UPPERCASE (`LIVE`). Accept all three via serde aliases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    #[serde(rename = "ORDER_STATUS_LIVE", alias = "live", alias = "LIVE")]
    Live,
    #[serde(rename = "ORDER_STATUS_MATCHED", alias = "matched", alias = "MATCHED")]
    Matched,
    #[serde(
        rename = "ORDER_STATUS_CANCELED",
        alias = "canceled",
        alias = "cancelled",
        alias = "CANCELED",
        alias = "CANCELLED"
    )]
    Canceled,
    #[serde(
        rename = "ORDER_STATUS_CANCELED_MARKET_RESOLVED",
        alias = "canceled_market_resolved",
        alias = "CANCELED_MARKET_RESOLVED"
    )]
    CanceledMarketResolved,
    #[serde(
        rename = "ORDER_STATUS_SYSTEM_CLEARED",
        alias = "system_cleared",
        alias = "SYSTEM_CLEARED"
    )]
    SystemCleared,
    #[serde(rename = "ORDER_STATUS_INVALID", alias = "invalid", alias = "INVALID")]
    Invalid,
}

/// Trade-lifecycle status. The REST `/trades` endpoint uses the long prefixed form
/// (`TRADE_STATUS_MATCHED`); the `/ws/user` channel uses the short UPPERCASE form
/// (`MATCHED`). Accept lowercase too, in case the server normalizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeStatus {
    #[serde(rename = "TRADE_STATUS_MATCHED", alias = "matched", alias = "MATCHED")]
    Matched,
    #[serde(rename = "TRADE_STATUS_MINED", alias = "mined", alias = "MINED")]
    Mined,
    #[serde(rename = "TRADE_STATUS_CONFIRMED", alias = "confirmed", alias = "CONFIRMED")]
    Confirmed,
    #[serde(rename = "TRADE_STATUS_RETRYING", alias = "retrying", alias = "RETRYING")]
    Retrying,
    #[serde(rename = "TRADE_STATUS_FAILED", alias = "failed", alias = "FAILED")]
    Failed,
}

/// Which side of the trade the user appears on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TraderSide {
    Taker,
    Maker,
}

/// `order` event payload, sent inside `data: {...}` on the user channel.
///
/// Live chainup uses an extremely lean envelope — only `id` and `status` are guaranteed.
/// Placement events fill `asset_id` / `side` / `original_size` / `price` / `type`;
/// cancellation events typically arrive with just `{id, status}`. Every field is
/// defaulted / optional so both shapes decode cleanly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrderEvent {
    pub id: String,
    pub status: OrderStatus,
    #[serde(default)]
    pub asset_id: String,
    #[serde(default)]
    pub side: Option<OrderSide>,
    #[serde(default)]
    pub price: String,
    #[serde(default)]
    pub original_size: String,
    /// Chainup wire field `type` carries the order type (`GTC`/`GTD`/`FOK`/`FAK`).
    #[serde(default, rename = "type")]
    pub order_type: String,
    /// MM-lazy persistence flag, serialized as the string `"true"` / `"false"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lazy: Option<String>,
    #[serde(default)]
    pub size_matched: String,
    /// Optional fields the asyncapi spec lists but chainup currently omits — kept defaulted
    /// for forward-compat and so legacy fixture tests still decode.
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub market: String,
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub maker_address: String,
    #[serde(default)]
    pub expiration: i64,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub associate_trades: Option<Vec<String>>,
    #[serde(default)]
    pub timestamp: Timestamp,
}

/// `trade` event payload, sent inside `data: {...}` on the user channel.
///
/// Like [`OrderEvent`], live chainup emits a lean envelope. The fields guaranteed by
/// observation are `id`, `asset_id`, `side`, `size`, `price`, and `status`. The asyncapi
/// spec lists many more (maker_orders, trader_side, transaction_hash, …) which chainup
/// either omits or fills only on later lifecycle frames (mined / confirmed). Every
/// optional field has a serde default so both shapes decode cleanly.
///
/// Chainup adds two fields not in the spec: `match_type` (`MATCH` / `MINT` / `MERGE` for
/// negRisk minting) and `order_id` (the order this trade settled into — a synonym for
/// `taker_order_id` accepted via alias).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TradeEvent {
    pub id: String,
    pub status: TradeStatus,
    #[serde(rename = "type", default = "default_trade_type")]
    pub sub_type: String,
    #[serde(default, alias = "order_id")]
    pub taker_order_id: String,
    #[serde(default)]
    pub market: String,
    #[serde(default)]
    pub asset_id: String,
    #[serde(default)]
    pub side: Option<OrderSide>,
    #[serde(default)]
    pub size: String,
    #[serde(default)]
    pub price: String,
    #[serde(default)]
    pub fee_rate_bps: String,
    /// Settlement type. `MATCH` = bilateral fill, `MINT` = mint complementary token
    /// (negRisk maker side), `MERGE` = burn complementary pair. Empty when chainup omits.
    #[serde(default)]
    pub match_type: String,
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub maker_address: String,
    #[serde(default)]
    pub transaction_hash: String,
    #[serde(default)]
    pub bucket_index: i64,
    #[serde(default)]
    pub matchtime: i64,
    #[serde(default)]
    pub last_update: i64,
    #[serde(default)]
    pub trader_side: Option<TraderSide>,
    #[serde(default)]
    pub maker_orders: Vec<MakerOrderFill>,
    #[serde(default)]
    pub timestamp: Timestamp,
}

fn default_trade_type() -> String {
    "TRADE".into()
}

/// `MakerOrderFill` schema — appears inside [`TradeEvent::maker_orders`] when
/// the user appears on the taker side of the trade.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MakerOrderFill {
    pub order_id: String,
    pub owner: String,
    pub maker_address: String,
    pub matched_amount: String,
    pub price: String,
    pub fee_rate_bps: String,
    pub asset_id: String,
    pub outcome: String,
    pub side: OrderSide,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_accepts_number_and_quoted_string() {
        let n: Timestamp = serde_json::from_str("1700000000").unwrap();
        assert_eq!(n.as_i64(), 1700000000);
        let s: Timestamp = serde_json::from_str("\"1700000000\"").unwrap();
        assert_eq!(s.as_i64(), 1700000000);
        let empty: Timestamp = serde_json::from_str("\"\"").unwrap();
        assert_eq!(empty.as_i64(), 0);
    }

    #[test]
    fn book_event_decodes() {
        let raw = r#"{
            "event_type": "book",
            "asset_id": "1234",
            "data": {
                "market": "0xabc",
                "asset_id": "1234",
                "bids": [{"price": "0.4", "size": "10"}],
                "asks": [{"price": "0.5", "size": "20"}],
                "timestamp": 1700000000,
                "hash": "0xdead"
            }
        }"#;
        let ev: MarketEvent = serde_json::from_str(raw).unwrap();
        let MarketEvent::Book(b) = ev else { panic!("wrong variant") };
        assert_eq!(b.asset_id, "1234");
        assert_eq!(b.bids[0].size, "10");
    }

    #[test]
    fn price_change_event_decodes() {
        let raw = r#"{
            "event_type": "price_change",
            "data": {
                "market": "0xabc",
                "price_changes": [{
                    "asset_id": "1234",
                    "price": "0.4",
                    "size": "0",
                    "side": "BUY",
                    "hash": "h",
                    "best_bid": "0.39",
                    "best_ask": "0.41"
                }],
                "timestamp": 1700000000
            }
        }"#;
        let ev: MarketEvent = serde_json::from_str(raw).unwrap();
        let MarketEvent::PriceChange(p) = ev else { panic!("wrong variant") };
        assert_eq!(p.price_changes[0].side, OrderSide::Buy);
        assert_eq!(p.price_changes[0].size, "0");
    }

    #[test]
    fn last_trade_price_event_decodes() {
        let raw = r#"{
            "event_type": "last_trade_price",
            "data": {
                "asset_id": "1234",
                "market": "0xabc",
                "price": "0.5",
                "size": "1",
                "fee_rate_bps": "10",
                "side": "SELL",
                "timestamp": 1700000001,
                "transaction_hash": ""
            }
        }"#;
        let ev: MarketEvent = serde_json::from_str(raw).unwrap();
        let MarketEvent::LastTradePrice(lt) = ev else { panic!("wrong variant") };
        assert_eq!(lt.side, OrderSide::Sell);
        assert_eq!(lt.transaction_hash, "");
    }

    #[test]
    fn order_event_decodes_live_placement_shape() {
        // The shape chainup actually sends on the `/ws/user` channel: lean payload, wire
        // `type` carries the order type, status is lowercase.
        let raw = r#"{
            "event_type": "order",
            "owner": "owner-uuid",
            "condition_id": "0xcid",
            "data": {
                "id": "0xorderhash",
                "asset_id": "1234",
                "lazy": "false",
                "original_size": "10",
                "price": "0.5",
                "side": "BUY",
                "status": "live",
                "type": "GTC"
            }
        }"#;
        let ev: UserEvent = serde_json::from_str(raw).unwrap();
        let UserEvent::Order(o) = ev else { panic!("wrong variant") };
        assert_eq!(o.status, OrderStatus::Live);
        assert_eq!(o.order_type, "GTC");
        assert_eq!(o.lazy.as_deref(), Some("false"));
    }

    #[test]
    fn order_event_decodes_cancellation_via_status() {
        let raw = r#"{
            "event_type": "order",
            "data": {
                "id": "0x",
                "asset_id": "1234",
                "side": "SELL",
                "original_size": "10",
                "size_matched": "5",
                "price": "0.5",
                "type": "GTC",
                "status": "canceled"
            }
        }"#;
        let ev: UserEvent = serde_json::from_str(raw).unwrap();
        let UserEvent::Order(o) = ev else { panic!("wrong variant") };
        assert!(o.lazy.is_none());
        assert_eq!(o.status, OrderStatus::Canceled);
    }

    #[test]
    fn trade_event_decodes() {
        let raw = r#"{
            "event_type": "trade",
            "data": {
                "type": "TRADE",
                "id": "t-uuid",
                "taker_order_id": "0xhash",
                "market": "0xcid",
                "asset_id": "1234",
                "side": "BUY",
                "size": "1",
                "price": "0.5",
                "fee_rate_bps": "10",
                "status": "TRADE_STATUS_MATCHED",
                "outcome": "Yes",
                "owner": "o",
                "maker_address": "0xsafe",
                "transaction_hash": "",
                "bucket_index": 0,
                "matchtime": 1700000000,
                "last_update": 1700000000,
                "trader_side": "TAKER",
                "maker_orders": [
                    {
                        "order_id": "0xmaker",
                        "owner": "om",
                        "maker_address": "0xms",
                        "matched_amount": "1",
                        "price": "0.5",
                        "fee_rate_bps": "10",
                        "asset_id": "1234",
                        "outcome": "Yes",
                        "side": "SELL"
                    }
                ],
                "timestamp": 1700000000
            }
        }"#;
        let ev: UserEvent = serde_json::from_str(raw).unwrap();
        let UserEvent::Trade(t) = ev else { panic!("wrong variant") };
        assert_eq!(t.status, TradeStatus::Matched);
        assert_eq!(t.trader_side, Some(TraderSide::Taker));
        assert_eq!(t.maker_orders.len(), 1);
        assert_eq!(t.maker_orders[0].side, OrderSide::Sell);
    }

    #[test]
    fn tick_size_change_event_decodes() {
        let raw = r#"{
            "event_type": "tick_size_change",
            "data": {
                "asset_id": "1234",
                "market": "0xcid",
                "old_tick_size": "0.01",
                "new_tick_size": "0.001",
                "timestamp": 1700000000
            }
        }"#;
        let ev: MarketEvent = serde_json::from_str(raw).unwrap();
        let MarketEvent::TickSizeChange(tsc) = ev else { panic!("wrong variant") };
        assert_eq!(tsc.old_tick_size, "0.01");
        assert_eq!(tsc.new_tick_size, "0.001");
    }

    #[test]
    fn best_bid_ask_event_decodes() {
        let raw = r#"{
            "event_type": "best_bid_ask",
            "data": {
                "asset_id": "1234",
                "market": "0xcid",
                "best_bid": "0.49",
                "best_ask": "0.51",
                "spread": "0.02",
                "timestamp": 1700000000
            }
        }"#;
        let ev: MarketEvent = serde_json::from_str(raw).unwrap();
        assert!(matches!(ev, MarketEvent::BestBidAsk(_)));
    }

    #[test]
    fn new_market_event_decodes() {
        let raw = r#"{
            "event_type": "new_market",
            "data": {
                "id": "m1",
                "question": "Q?",
                "market": "0xcid",
                "slug": "q",
                "assets_ids": ["1", "2"],
                "outcomes": ["Yes", "No"],
                "tags": ["sport"],
                "timestamp": 1700000000
            }
        }"#;
        let ev: MarketEvent = serde_json::from_str(raw).unwrap();
        let MarketEvent::NewMarket(nm) = ev else { panic!("wrong variant") };
        assert_eq!(nm.assets_ids, vec!["1", "2"]);
        assert_eq!(nm.outcomes, vec!["Yes", "No"]);
    }

    #[test]
    fn market_resolved_event_decodes() {
        let raw = r#"{
            "event_type": "market_resolved",
            "data": {
                "id": "m1",
                "market": "0xcid",
                "assets_ids": ["1", "2"],
                "winning_asset_id": "1",
                "winning_outcome": "Yes",
                "tags": [],
                "timestamp": 1700000001
            }
        }"#;
        let ev: MarketEvent = serde_json::from_str(raw).unwrap();
        let MarketEvent::MarketResolved(mr) = ev else { panic!("wrong variant") };
        assert_eq!(mr.winning_outcome, "Yes");
    }
}
