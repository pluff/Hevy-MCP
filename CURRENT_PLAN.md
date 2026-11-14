# Plan: migrate from `mcp-server`/`mcp-spec` to `rmcp`

Official Rust MCP SDK: https://github.com/modelcontextprotocol/rust-sdk (crate `rmcp`).
Use `#[tool_router]`, `#[tool]`, `#[tool_handler]`, `Parameters<T>`, `schemars::JsonSchema`,
`transport::streamable_http_server::StreamableHttpService`. Check latest rmcp version and docs
(docs.rs/rmcp, repo `examples/servers`) before coding; do not guess API.

## Current state
- `src/router/` — `mcp_server::Router` impl (`RequestRouter`), hand-written JSON schemas (`tools/`),
  untyped arg parsing (`args/`), handlers (`handlers/`), text formatters (`formatters.rs`).
- `src/http/` — hand-rolled JSON-RPC over axum 0.7: `handler.rs` (initialize/session/dispatch),
  `session.rs` (session map + api_key), `oauth.rs` (OAuth 2.1 AS + bearer → Hevy API key, keep),
  `logging.rs` (keep), `mod.rs` (routes).
- `src/hevy/` — Hevy REST client + types (keep as is).
- Auth: Hevy API key arrives as OAuth bearer token on every request; `HevyRouter` caches `HevyClient` per key.

## Target
- `HevyServer` (rmcp `ServerHandler`) with `#[tool_router]`; tool args = typed `schemars` structs.
- `StreamableHttpService` mounted in axum; bearer middleware validates token, puts Hevy API key
  into request extensions; tool handlers read it via `RequestContext`/`http::request::Parts` extension.
- Own session/JSON-RPC code deleted. OAuth, logging, `/health`, well-known routes, env vars
  (`HEVY_HTTP_ADDR`, `PORT`, `HEVY_MCP_PATH`, `HEVY_SESSION_TIMEOUT_SECS`, `LOG_LEVEL`) keep working.
- Tool names, descriptions, input schemas and output text stay behaviour-compatible.

## Rules for every part
- One part = one commit. Commit message: conventional style, short. **NO Co-Authored-By / no attribution lines of any kind.**
- `cargo build` and `cargo clippy` must pass at the end of each part (and `cargo test` if tests exist).
- Do not touch files outside the part's scope. Mark finished part below with `[x]`.
- Axum version: rmcp may need a different axum major than the 0.7 used now; resolve in Part 1/4 (bump axum if needed, fix oauth/logging accordingly).

## Parts

- [x] **Part 1 — Baseline + rmcp scaffold.**
  Snapshot current `tools/list` output (names, descriptions, inputSchema) into `tests/fixtures/tools_baseline.json`
  (write a small throwaway test/bin to dump it from the old router). Add `rmcp` (features: `server`, `macros`,
  `transport-streamable-http-server`, + whatever needed) and `schemars`. Create `src/server/mod.rs` with `HevyServer`
  (holds `Arc<HevyRouter>`-style client cache, `ToolRouter<Self>`), `ServerHandler` impl (name, instructions,
  tools capability) and API-key extraction helper from request context. Zero tools yet. Old code stays working.

- [x] **Part 2 — Workout tools + shared arg types.**
  Port `get_workouts`, `get_workouts_count`, `get_workout_events`, `get_workout`, `create_workout`, `update_workout`,
  `get_exercise_history` to `#[tool]` methods with typed `Parameters<...>` structs (derive `Deserialize, JsonSchema`),
  reusing `hevy::types` and existing formatters. Descriptions identical to old ones. Errors → `rmcp::ErrorData`/tool error result.

- [x] **Part 3 — Routine, folder, template tools.**
  Port `get_routines`, `get_routine`, `create_routine`, `update_routine`, `get_routine_folders`, `get_routine_folder`,
  `create_routine_folder`, `get_exercise_templates`, `get_exercise_template`, `create_exercise_template`.
  Add test comparing generated tool list (names + required fields) against `tools_baseline.json`; document any intentional schema diffs.

- [x] **Part 4 — HTTP transport swap.**
  Replace `http/handler.rs` + `http/session.rs` with `StreamableHttpService` (`LocalSessionManager`, session timeout from
  `HEVY_SESSION_TIMEOUT_SECS` via config, path from `HEVY_MCP_PATH`). Bearer middleware: use `oauth.bearer_key`, 401 via
  `oauth.unauthorized`, inject key into request extensions. Keep oauth routes, `/health`, logging layer. Switch `main.rs`/`lib.rs` to `HevyServer`.
  Smoke test with curl: initialize → tools/list → tools/call (invalid key path ok), 401 without token, DELETE session.

- [ ] **Part 5 — Remove legacy + docs.**
  Delete `src/router/`, `mcp-server`/`mcp-spec` deps and any unused deps (`bytes`, `tower-service`, `uuid`, `dashmap` if unused),
  `HevyRouter` leftovers. Update README (architecture, rmcp), Dockerfile/CI if build changed, `cargo clippy -- -D warnings`,
  `cargo test`. Delete `CURRENT_PLAN.md` in this final commit.
