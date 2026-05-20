# pm-cli (`pm`)

Terminal client for ChainUp's [`pm-cup2026`](https://github.com/chainupcloud/pm-cup2026) prediction-market platform. Browse markets, place orders, manage positions — counterpart of Polymarket's [`polymarket-cli`](https://github.com/Polymarket/polymarket-cli), with feature parity for everything chainup's backend exposes (see [Non-goals](#non-goals) for what it deliberately skips).

```bash
$ pm --tenant hermestrade.xyz time
2026-05-20T02:25:10Z

$ pm --tenant hermestrade.xyz book 3404...0576
asks:
  0.75 × 5
bids:
  0.68 × 10
```

## Install

### Build from source

```bash
git clone https://github.com/chainupcloud/pm-rs.git
cd pm-rs
cargo build --release
install -m 0755 target/release/pm ~/.local/bin/pm
```

Requires Rust 1.80+ (`rust-toolchain.toml` pins the exact version used by the CI build).

## Quick start

### Read-only — no wallet needed

```bash
# Point at a tenant — clob-api / gamma-api / clob-ws are derived automatically.
pm --tenant hermestrade.xyz ok                  # server health
pm --tenant hermestrade.xyz time
pm --tenant hermestrade.xyz endpoints           # show derived URLs + chain id
pm --tenant hermestrade.xyz book   <TOKEN_ID>
pm --tenant hermestrade.xyz midpoint <TOKEN_ID>
pm --tenant hermestrade.xyz gamma events get how-many-fed-rate-cuts-in-2026-pm-406282
```

Or supply the CLOB URL directly (useful for non-canonical hostnames or local dev):

```bash
pm --clob-endpoint https://clob-api.predict.prax1s.xyz time
```

JSON for scripts:

```bash
pm --tenant hermestrade.xyz -o json book 3404...0576 | jq '.bids[0]'
```

### Trading — wallet + L2 credentials

```bash
# 1. Wallet — pick one
pm wallet create                                # generates a fresh EOA, stores 0600
pm wallet import 0xYOURKEY                      # or import an existing one
pm wallet set-safe 0xYOUR_SAFE_ADDRESS          # required when signature-type = gnosis-safe
pm wallet show                                  # eoa + safe + source

# 2. Create an L2 API key (writes credentials.json mode 0600)
pm auth create-key --output json > credentials.json

# 3. Trade
export PM_TENANT=hermestrade.xyz
export PM_CHAIN_ID=143
export PM_SCOPE_ID=0x1811a132dd725e2c40475aa52df39025b36544f7a70825968e32b28da2196e95
export PM_CREDENTIALS_FILE=$PWD/credentials.json

pm balance --asset-type collateral
pm order create --token 3404...0576 --side buy --price 0.10 --size 5 \
                --fee-rate-bps 20 --maker 0xYOUR_SAFE
pm order list
pm order cancel <ORDER_ID>
```

The first run prompts auto-pick a sensible config dir (`~/.config/pm` on Linux, mode 0700). Override with `--config-dir` or `PM_CONFIG_DIR`.

## Configuration

### Resolution order

Every connection flag (`--tenant`, `--clob-endpoint`, `--chain-id`, `--scope-id`, `--private-key`, …) resolves in this order:

1. CLI flag — wins.
2. Env var — `PM_TENANT`, `PM_CLOB_ENDPOINT`, `PM_CHAIN_ID`, `PM_SCOPE_ID`, `PM_PRIVATE_KEY`, `PM_SIGNATURE_TYPE`, `PM_EXCHANGE_ADDRESS`, `PM_CONFIG_DIR`, `PM_CREDENTIALS_FILE`, `PM_OUTPUT`.
3. Stored config — `<config-dir>/config.toml` (written by `pm wallet …`).

Empty values are treated as unset.

### Signature types

| Value | Type | Use when |
|-------|------|----------|
| `eoa` | 0 — direct EOA signing | Funds held in the same EOA that signs. Polymarket-style trading wallet. |
| `proxy` | 1 — Polymarket proxy wallet | Legacy / interop. |
| `gnosis-safe` (**default**) | 2 — 1-of-1 Gnosis Safe | **chainup default.** EOA signs; the Safe is the `maker` and holds the funds. |

The default is `gnosis-safe`. Persist a different choice with `pm wallet create --signature-type eoa`, or override per-invocation via `--signature-type <eoa|proxy|gnosis-safe>` / `PM_SIGNATURE_TYPE`.

### `scopeId` — multi-tenant isolation

`scopeId` is a `bytes32` value embedded in every signed `ClobAuth` and `Order`. Two clients on the same EOA but different scopes derive different L2 keys and never share order state. Fetch the right one with:

```bash
# From the server (returns the canonical scope for your tenant)
curl https://clob-api.<tenant>/auth/nonce | jq -r .scopeId

# Or via the CLI
pm auth nonce | grep scopeId
```

Set it via flag, env var, or `pm wallet create --scope-id 0x…`.

### Network config (`approve check`)

`pm approve check` needs to know which contracts to query. Pass a YAML file (one ships at [`examples/networks/monad-hermestrade.yaml`](../examples/networks/monad-hermestrade.yaml)):

```bash
pm approve check --network-config examples/networks/monad-hermestrade.yaml
```

The YAML schema is the same one used by the backend deploy tooling, so you can point at the file the tenant ops team already maintains.

## Commands

### Market data — public, no auth

| Command | What it does |
|---------|--------------|
| `pm ok` / `pm time` | Server health + clock |
| `pm endpoints` | Show the derived clob / gamma / ws URLs + chain id |
| `pm midpoint <TOKEN>` | Single-token midpoint |
| `pm price <TOKEN> --side buy` | Last price (one side) |
| `pm spread <TOKEN>` | Best-bid / best-ask + spread |
| `pm book <TOKEN>` | Top-of-book depth |
| `pm tick-size <TOKEN>` | Active tick size |
| `pm fee-rate <TOKEN>` | Fee rate bps |
| `pm last-trade <TOKEN>` | Last trade price |
| `pm price-history <TOKEN> --interval 1h \| 6h \| 1d \| 1w \| 1m \| all` | Historical price points |
| `pm midpoints t1 t2 ...` | Batch (≤ 500 tokens) |
| `pm prices t1:buy t2:sell ...` | Batch — per-token side selectable |
| `pm spreads t1 t2 ...` | Batch spreads |
| `pm books t1:buy t2:sell ...` | Batch books |
| `pm last-trades t1 t2 ...` | Batch last trades |

### Gamma — event / market discovery

```bash
pm gamma events list --limit 10
pm gamma events get how-many-fed-rate-cuts-in-2026-pm-406282
pm gamma events tags 291
pm gamma markets get <CONDITION_ID>            # or slug
pm gamma profiles get <SAFE_ADDRESS>
pm gamma tags list
```

Chainup's Gamma is REST-only; there is no streaming variant.

### Wallet

```bash
pm wallet create [--force]                    # random EOA, mode 0600
pm wallet import 0xHEXKEY
pm wallet address                             # print EOA only
pm wallet show                                # eoa + safe + source
pm wallet reset                               # delete config
pm wallet set-safe 0xSAFE                     # store Safe address (gnosis-safe mode)
pm wallet detect-safe                         # ask the server for the Safe linked to the API key
```

### Authentication (L1 + L2 API keys)

```bash
pm auth nonce                                 # nonce + scopeId for the current EOA
pm auth derive-key                            # deterministic L2 key derivation (no server write)
pm auth create-key                            # POST /auth/api-key
pm auth list-keys
pm auth delete-key <UUID> [--nonce N]
```

### Trading

```bash
# Place a limit order (default GTC)
pm order create --token <T> --side buy --price 0.10 --size 5 \
                --fee-rate-bps 20 --maker <SAFE>

# postOnly / GTD
pm order create --token <T> --side buy --price 0.10 --size 5 \
                --fee-rate-bps 20 --maker <SAFE> --post-only
pm order create --token <T> --side buy --price 0.10 --size 5 \
                --fee-rate-bps 20 --maker <SAFE> \
                --order-type gtd --expiration $(( $(date +%s) + 600 ))

# Market order (BUY only — amount denominated in USDW; chainup runs the book walk)
pm order create --token <T> --side buy --amount 3.75 --price 0.75 \
                --fee-rate-bps 20 --maker <SAFE> --market

# Batch place
pm order post-batch --tokens t1,t2 --prices 0.10,0.05 --sizes 5,5 \
                    --side buy --fee-rate-bps 20 --maker <SAFE>

# Manage
pm order list
pm order get <ID>
pm order cancel <ID>
pm order cancel-many <ID1>,<ID2>,...
pm order cancel-all
pm order replace --orders-file replace.json   # atomic cancel + re-place

# Dry-run anywhere — prints the signed envelope, does NOT post
pm order create ... --dry-run -o json
```

#### Lot size + minimum size

- **Minimum order size: 5 shares.** Smaller orders return `ORDER_SIZE_TOO_SMALL`.
- **Lot size: 0.01.** For market orders, `amount / price` must round to a multiple of 0.01.

#### Fee model (live finding)

Fees are deducted **in shares on the receiving side**, not in USDW. A BUY 5 × 0.09 with `--fee-rate-bps 20`:
- USDW spent: 5 × 0.09 = 0.45 (exact)
- Tokens received: 5 - 0.01 = 4.99 (fee in shares)

### Trades + balance

```bash
pm trade                                      # your trade history
pm trade --token <T>
pm balance --asset-type collateral
pm balance --asset-type conditional --token <T>
pm balance --update                           # force refresh from the server
pm fee-rate                                   # account-level fee tier
pm heartbeat                                  # server-side liveness ping
```

### Approval helpers

```bash
pm approve check --network-config examples/networks/monad-hermestrade.yaml
```

Reads `IERC20.allowance(owner, target)` + `IERC1155.isApprovedForAll(owner, target)` for every spender in the YAML. Read-only — no on-chain writes. The `set` flow is deferred while the Safe `execTransaction` path is finalized (see [Non-goals](#non-goals)).

### WebSocket

```bash
pm ws ping                                    # connectivity check
pm ws book <TOKEN>                            # one-shot book snapshot via WS
pm ws book-watch <TOKEN>                      # stream book updates
pm ws user                                    # stream your order + trade events
pm ws user --markets cond1,cond2              # filter to specific condition ids
```

Connection state survives transient disconnects — the SDK auto-reconnects and replays the subscription.

## Common workflows

### Browse markets without a wallet

```bash
pm --tenant hermestrade.xyz gamma events list --limit 5
pm --tenant hermestrade.xyz book 3404...0576
pm --tenant hermestrade.xyz price-history 3404...0576 --interval 1d
```

### From zero to first order

```bash
# Pick wallet + chain config once
pm wallet create --signature-type gnosis-safe --chain-id 143 \
                 --scope-id 0x1811a132...196e95
pm wallet set-safe 0xYOUR_SAFE                  # the Safe controlled by your EOA

# Verify the Safe is funded + approved
pm balance --asset-type collateral
pm approve check --network-config examples/networks/monad-hermestrade.yaml

# Fire your first order
pm order create --token 3404...0576 --side buy --price 0.10 --size 5 \
                --fee-rate-bps 20 --maker 0xYOUR_SAFE
```

### Place + cancel cycle (no fill)

```bash
ID=$(pm order create --token <T> --side buy --price 0.10 --size 5 \
                     --fee-rate-bps 20 --maker <SAFE> -o json | jq -r .orderID)
pm order get $ID
pm order cancel $ID
```

### Cross-spread fill (real trade, real money)

```bash
# Yes book — best ASK 0.09 × 10
ID=$(pm order create --token <YES_TOKEN> --side buy --price 0.09 --size 5 \
                     --fee-rate-bps 20 --maker <SAFE> -o json | jq -r .orderID)
# Order will return with status="matched" and a tradeIDs[] populated.
pm trade
pm balance --asset-type conditional --token <YES_TOKEN>
```

### Monitor your trades over WS

```bash
# Terminal A — start the user channel before placing the order
pm ws user

# Terminal B — fire the order
pm order create ...
# Terminal A prints the matching trade + lifecycle order events as they arrive.
```

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `ORDER_SIZE_TOO_SMALL: limit order requires share >= 5` | Order size below the 5-share minimum. | Increase to ≥ 5, even if the per-share price is low. |
| `size 0.66… has 28 decimals; chainup lot size is 2` | Market `--amount / --price` didn't round to 0.01. | Pick `amount` so `amount / price` is a multiple of 0.01. |
| `unknown variant 'MATCHED' / 'cancelled'` from `pm ws user` | Pre-`60904cc` build. | `git pull && cargo build`. |
| `proxy_wallet` differs between API keys | Server returns the proxy from the first key created with a given scope. | Use `pm wallet set-safe <addr>` manually or filter by `--api-key` in code. |
| TLS handshake panic on startup | rustls 0.23 missing crypto provider. | Already fixed in `ee4eec2`. Pull latest. |
| `/heartbeat` returns empty body | Known minor: chainup may return `{}` rather than `{status: ok}`. Functional, just visually empty. | — |

## Non-goals

Commands intentionally omitted because chainup's backend doesn't expose the underlying endpoint:

- **Market browsing** — `markets list / get / sampling-markets / simplified-markets`. Chainup pushes discovery through Gamma instead (`pm gamma events …`).
- **Polymarket rewards** — `rewards list / earnings / reward-percentages / current-rewards / orders-scoring`. Chainup tenants run their own incentive logic.
- **Notifications + account state** — `notifications / closed-only-mode / account-status / geoblock / neg-risk` (the neg-risk flag is embedded in the `/book` response).
- **`bridge`, `data`, `rtds`, `rfq`, `ctf split / merge / redeem`** — Polymarket-proprietary or front-end-handled.
- **`upgrade`, `shell`, `setup`** — on the roadmap (see `.local/TODO.md`), not yet shipped.
- **`approve set`** — deferred behind the Safe `execTransaction` broadcast question.

## Output formats

```bash
pm --tenant ... -o table  ...       # default — human-readable
pm --tenant ... -o json   ...       # machine-readable; pipe through jq
```

Or set `PM_OUTPUT=json` once and forget about it.

## License

Apache-2.0. See `LICENSE` at the repo root.
