# pm-rs auth flow (Phase 2.1)

This document describes the L1 (EIP-712) and L2 (HMAC-SHA256) authentication used by `pm-rs-clob-client` Phase 2.1, including the Safe-wallet derivation note.

Authoritative server-side references:
- `pm-cup2026/services/clob-service/internal/tradingapi/handlers/auth.go` — L1 verification.
- `pm-cup2026/services/clob-service/internal/tradingapi/middleware/auth.go` — L2 verification.

---

## 1. L1 — EIP-712 ClobAuth

Used by **API-key management**: `POST /auth/api-key`, `GET /auth/derive-api-key`, `DELETE /auth/api-key`. The body is empty for every L1 endpoint; the binding lives entirely in headers.

### Header set

| Header | Required | Format | Notes |
|--------|----------|--------|-------|
| `PRED_ADDRESS` | yes | `0x` + 40 hex chars | EOA address recovered by EIP-712 signature verification. |
| `PRED_NONCE` | yes | base-10 `u32` string | Sent literally, even when zero. |
| `PRED_TIMESTAMP` | yes | Unix seconds (string) | Server tolerance: **±5 minutes**. |
| `PRED_SIGNATURE` | yes | `0x` + 130 hex chars | 65-byte `r ‖ s ‖ v`. `v ∈ {0, 1}` — the server normalises `v ≥ 27` for compatibility. |
| `PRED_SCOPE_ID` | optional | `0x` + 64 hex chars | Multi-tenant scope. Omit when binding is empty / "no scope". |

### Sequence (create_or_derive_api_key)

```
client                                                                 server
  │                                                                       │
  ├─ build PRED_* headers from signer.sign_clob_auth(timestamp, nonce) ──▶│
  │  POST /auth/api-key       (empty body)                                │
  │                                                                       │
  │       200 + {apiKey, secret, passphrase}        ◀───────────────────  │
  │  (or 4xx — fall back to derive)                                       │
  │                                                                       │
  │   on 4xx:                                                             │
  ├─ same PRED_* headers ────────────────────────────────────────────────▶│
  │  GET  /auth/derive-api-key                                            │
  │                                                                       │
  │       200 + {apiKey, secret, passphrase}        ◀───────────────────  │
```

### EIP-712 ClobAuth struct (5 fields)

```
ClobAuth(address address,string timestamp,uint256 nonce,bytes32 scopeId,string message)
```

The `message` field is the constant `"This message attests that I control the given wallet"`. Domain is short-form: `name="ClobAuthDomain"`, `version="1"`, `chainId=<config>`, no `verifyingContract`.

`scopeId` is `bytes32`. Pass `ScopeId::ZERO` to opt out of tenant binding (in which case `PRED_SCOPE_ID` is **omitted** from the request, matching the server contract).

---

## 2. L2 — HMAC-SHA256

Used by **trading + read endpoints**: `/auth/api-keys`, `/balance-allowance[/update]`, all `/order(s)` paths, `/trades`, etc.

### Header set

| Header | Format | Source |
|--------|--------|--------|
| `PRED_API_KEY` | UUID | `Credentials::key` returned by L1. |
| `PRED_PASSPHRASE` | random ASCII | `Credentials::passphrase`. |
| `PRED_TIMESTAMP` | Unix seconds (string) | Server tolerance: **±30 s**. |
| `PRED_ADDRESS` | `0x` + 40 hex chars | EOA address — must equal the L1 signer that minted the key. |
| `PRED_SIGNATURE` | base64 (standard) of HMAC-SHA256 | See formula below. |

### HMAC formula

```
secret_bytes = base64_standard_decode(credentials.secret)   # or raw bytes on decode failure
message      = timestamp || method || path || body          # concatenated, no separators
signature    = base64_standard_encode( HMAC_SHA256(secret_bytes, message) )
```

Byte-level subtleties — these must match `middleware/auth.go::computeHMAC` exactly:

- **`path` is the URL path ONLY** — the query string is excluded. The server reads `c.Request.URL.Path` (Gin), which never contains `?...`. Signing path+query produces `401 invalid signature`.
- **Method is the upper-case verb** (`GET`, `POST`, ...). `Request::method().as_str()` returns the upper-case form, which matches the server.
- **Body is the raw UTF-8 bytes** that the server reads from the request, exactly as transmitted. For GET / DELETE this is the empty string.
- **base64 uses the STANDARD alphabet** (`+ /` instead of `- _`). Polymarket V1 uses URL-safe; `pm-cup2026` deliberately diverged.
- **Secret decode is permissive**: the server falls back to raw bytes when the secret is not valid base64. The SDK mirrors this in `auth::compute_l2_hmac`.

### Sequence (any L2 request)

```
client                                                              server
  │                                                                    │
  ├─ ts = now()                                                        │
  ├─ sig = base64( hmac_sha256(secret, ts || method || path || body) ) │
  ├─ method PATH?qs   PRED_API_KEY,PRED_PASSPHRASE,PRED_SIGNATURE, ──▶│
  │                   PRED_TIMESTAMP, PRED_ADDRESS                     │
  │  body...                                                           │
  │                                                                    │
  │       200 + JSON                                  ◀──────────────  │
  │  (or 401 on signature mismatch / timestamp drift / unknown key)    │
```

---

## 3. Safe-wallet derivation

`pm-cup2026` users transact through a **Gnosis Safe** (`signatureType = 2`):

- `maker` (the wallet holding USDC / outcome tokens) is the Safe address.
- `signer` (the L1 signer that produces EIP-712 signatures) is the Safe owner EOA.
- The Safe address is **deterministic**: CREATE2-derived by `SafeProxyFactory` with `salt = keccak256(abi.encode(signer, scopeId))`.

For Phase 2.1 the SDK does NOT compute the Safe address client-side. Two consequences:

1. `Client::balance_allowance` returns the **Safe wallet's** balance — the server derives the Safe address from `EOA + scopeId` and reads its on-chain balance. The SDK only sends `PRED_ADDRESS = EOA`; the server does the lookup.
2. `Client::api_keys` returns `proxy_wallet` (the Safe address) as part of the response so downstream callers know which address actually holds funds.

A future client-side `derive_safe_address(signer, scope_id, factory_contract)` helper is tracked under Phase 2.2.

---

## 4. Quick reference

| Operation | Method | Path | Auth | Headers |
|-----------|--------|------|------|---------|
| Create API key | POST | `/auth/api-key` | L1 | `PRED_ADDRESS / PRED_NONCE / PRED_SIGNATURE / PRED_TIMESTAMP [+ PRED_SCOPE_ID]` |
| Derive API key | GET | `/auth/derive-api-key` | L1 | same |
| Revoke API key | DELETE | `/auth/api-key` | L1 | same |
| List API keys | GET | `/auth/api-keys` | L2 | `PRED_API_KEY / PRED_PASSPHRASE / PRED_SIGNATURE / PRED_TIMESTAMP / PRED_ADDRESS` |
| Balance + allowance | GET | `/balance-allowance` | L2 | same |
| Force refresh | GET | `/balance-allowance/update` | L2 | same |
