# Phase 3a — Gamma client (notes)

Branch: `feat/phase3-gamma`. SDK + CLI implementation of the
`gamma-service` REST API. Source of truth: `pm-cup2026/services/gamma-service`
(openapi + Go handlers + `internal/models/models.go`).

## Files added

### `clob-client/src/gamma/`

| File | Purpose |
|------|---------|
| `mod.rs` | Module entrypoint; re-exports `GammaClient`. |
| `client.rs` | `GammaClient` (one method per endpoint) + private `get_json` + 3 query-encoding unit tests. |
| `types/mod.rs` | Public re-export of `request::*` and `response::*`. |
| `types/request.rs` | Query-parameter structs (`ListEventsRequest`, `ListTagsRequest`, `ListSeriesRequest`, `ListCommentsRequest`, `CommentsByUserAddressRequest`, `RelatedTagsRequest`, `GetSeriesRequest`, `ListEventCreatorsRequest`, `ListCurationEventsRequest`, `SearchRequest`). All optional fields are `Option<T>` and skip-when-`None`. |
| `types/response.rs` | Response types mirroring `gamma-service/internal/models/models.go`: `Event`, `Market` (+ `Adjudication`, `NextStep`), `Tag`, `RelatedTag`, `SearchTag`, `Series`, `SeriesSummary`, `Comment` (+ `CommentProfile`, `CommentPosition`, `Reaction`), `Count`, `Profile`, `PublicProfile`, `PublicProfileUser`, `EventCreator`, `CurationEvent`, `Category`, `Chat`, `Template`, `Collection` (stub), `ImageOptimization`, `Pagination`, `Search`, `SportType` + `SportStage` + `SportTypesResponse`, `PublicInfo`, `Agreement`, `AgreementsResponse`, `HealthResponse`. |

### CLI

- `cli/src/gamma_commands.rs` — single-file owner of the `Gamma` subtree (clap definitions + dispatch + table/JSON renderers). No other Phase-2-owned CLI files were materially touched.

### Tests

| File | Type | Purpose |
|------|------|---------|
| `clob-client/tests/gamma_http.rs` | `httpmock` integration | 18 tests: one per endpoint family, asserting HTTP method + path + query-string assembly. Offline. |
| `clob-client/tests/gamma_smoke.rs` | live network, `#[ignore]` | `list_events(limit=2)` against `https://gamma-api.hermestrade.xyz`. Run with `cargo test --workspace -- --ignored gamma_smoke`. |

### Docs

- `docs/gamma.md` — endpoint matrix, Polymarket-comparison, request-construction guide, smoke-test instructions.
- `docs/diff-vs-polymarket-v1.md` — Gamma row updated to "implemented in Phase 3a".

## Shared-file edits (surgical, additive only)

- `clob-client/src/lib.rs` — `pub mod gamma;` + `pub use gamma::GammaClient;` (additive).
- `clob-client/src/client.rs` — added `Client::gamma() -> Result<GammaClient>` accessor. No refactor of `Client` / `Inner`. Reuses the existing `inner.http` via clone, the existing `Endpoints::gamma` URL, and returns `Error::Validation` when unset.
- `clob-client/Cargo.toml` — added `httpmock` dev-dep.
- `Cargo.toml` (workspace) — added `httpmock` workspace dep.
- `cli/src/main.rs` — declared `mod gamma_commands;`.
- `cli/src/cli.rs` — added `Gamma(GammaArgs)` variant to `Command`.
- `cli/src/commands.rs` — added one match arm dispatching to `gamma_commands::run`.
- `cli/src/output.rs` — relaxed `print_json<T: Serialize + ?Sized>` so slices (`&[T]`) can be JSON-rendered (needed by Gamma list renderers; one-line additive change).

Diff with Phase-2 hot files is intentionally minimal to reduce merge friction.

## Endpoint coverage

Every `gamma-service` route present in `services/gamma-service/internal/handlers/router.go`
that is not auth- or write-restricted is implemented. Specifically:

- System: `/health`, `/public-info`, `/agreements`, `/config/sport-types`
- Tags: `/tags`, `/tags/{id}`, `/tags/slug/{slug}`, `/tags/{id}/related-tags`, `/tags/{id}/related-tags/tags`, `/tags/slug/{slug}/related-tags`, `/tags/slug/{slug}/related-tags/tags`
- Events: `/events`, `/events/{id}`, `/events/slug/{slug}`, `/events/{id}/tags`, `/events/creators`, `/events/creators/{id}`, `/curation/events`
- Markets: `/markets/{id}`, `/markets/slug/{slug}`, `/markets/{id}/tags`, `POST /markets/information`
- Series: `/series`, `/series/{id}`, `/series/{id}/comments/count`, `/series-summary/{id}`, `/series-summary/slug/{slug}`
- Comments: `/comments`, `/comments/{id}`, `/comments/user_address/{addr}`
- Profiles (read): `/public-profile`, `/profiles/user_address/{addr}`
- Search: `/public-search`

### CLI subcommand tree

```
pm gamma health
pm gamma public-info
pm gamma agreements
pm gamma sport-types
pm gamma search <query> [--limit-per-type N] [--page N] [--search-tags BOOL] [--search-profiles BOOL]
pm gamma events list      [--limit N] [--offset N] [--order F] [--ascending] [--slug S] [--tag-id N] [--active|--closed|--archived|--featured BOOL]
pm gamma events get       <id|slug>
pm gamma events tags      <id>
pm gamma events creators  [--limit N] [--offset N] [--order F] [--ascending] [--creator-name X] [--creator-handle X]
pm gamma events creator   <id>
pm gamma markets get      <id|slug> [--include-tag]
pm gamma markets tags     <id>
pm gamma markets information --body '<json>' | --body-file PATH
pm gamma tags list        [--limit N] [--offset N] [--order F] [--ascending] [--is-carousel BOOL]
pm gamma tags get         <id|slug>
pm gamma tags related     <id|slug> [--full] [--status S] [--omit-empty]
pm gamma series list      [--limit N] [--offset N] [--order F] [--ascending] [--recurrence R] [--closed BOOL] [--exclude-events]
pm gamma series get       <id> [--exclude-events] [--include-chat]
pm gamma series summary   <id|slug>
pm gamma series comments-count <id>
pm gamma comments list    [--parent-entity-type T] [--parent-entity-id N] [--limit N] [--offset N] [--order F] [--ascending]
pm gamma comments get     <id>
pm gamma comments by-user <addr> [--limit N] [--offset N] [--order F] [--ascending]
pm gamma profiles public  <addr>
pm gamma profiles get     <addr>
pm gamma curation events  [--featured-level N]
```

## Out of scope (deliberate)

- `POST /profiles` — Bearer-JWT-authenticated; Phase 2 owns the auth flow.
- `POST /disputes/evidence` — write endpoint; defer to Phase 3b (or a dedicated dispute module).
- `/games*`, `/sports-events`, `/disputes/evidence` — present in the router but not in the openapi; tenant-specific shapes for the user-dapp. SDK-side wrapping is straightforward but produced no caller demand for Phase 3a. Document in `docs/gamma.md` so a Phase 3b ticket can pick them up.
- Polymarket-only endpoints (`/teams`, `/sports`, `/sports/market-types`) — not present in the `gamma-service`. Dropped from the CLI per the task brief.
- Gamma streaming — REST-only; no streaming variant.

## Test summary

`cargo test --workspace` (excluding `#[ignore]`):

```
clob-client lib tests          13 passed
clob-client::gamma_http        18 passed
clob-client::gamma_smoke        0 passed (1 ignored)
clob-client::golden_signer      3 passed
doctests                        2 passed
```

`cargo build --release` succeeds.

The `#[ignore]`d `gamma_smoke_list_markets_against_hermestrade` test passes
when run manually against the live `https://gamma-api.hermestrade.xyz` deployment.

## Known limitations / open questions

1. **`POST /markets/information`** body is `serde_json::Value` rather than a typed `MarketsInformationBody`. The server accepts a free-form JSON shape and the field semantics differ from Polymarket; we documented the accepted keys in the SDK doc-comment but did not produce a typed wrapper to keep this PR small. A typed `MarketsInformationBody` is a one-evening follow-up if a caller wants it.
2. **`Event.category`** is currently typed as `Option<String>` to match the Go `*string` shape, but the live server appears to also return integer-stringified values (e.g. `"0"`). serde-`Option<String>` deserialises both fine; if a future server change emits a bare integer we will need a `serde_with::PickFirst<...>` shim.
3. **Auth headers**: `GammaClient` adds no auth headers. The Gamma write endpoints (`POST /auth/login`, `POST /auth/refresh`, `POST /profiles`) require Bearer JWT — Phase 2 will wire that through the shared `Credentials` plumbing. The current sub-client only exposes read endpoints, which are unauthenticated.
4. **CLI auto-detection of numeric vs slug** uses `chars().all(is_ascii_digit)`. This is correct for the current data shape (all IDs are pure integers) but would mis-detect a hypothetical numeric slug. The server also offers an explicit `/slug/{slug}` path so callers can disambiguate by passing a slug that contains a hyphen.
5. **`Adjudication.payoutVector`** is the JSON-array string `[1,0]` or `[0,1]` — we surface it as `Option<String>` and leave parsing to the caller. A typed `Vec<i32>` would only need a `serde_json::from_str` helper similar to `Market::parsed_clob_token_ids`.
