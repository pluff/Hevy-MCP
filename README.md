# Hevy MCP Server

MCP server for the [Hevy](https://hevy.com) fitness API, with HTTP transport and session management. Entirely written in Rust for low memory management. Also, proper async support!!!

## Quick start with Docker

```bash
docker pull ghcr.io/zelosleone/hevy-mcp-server:latest
docker run -e OAUTH_SECRET=long-random-string -p 5000:5000 ghcr.io/zelosleone/hevy-mcp-server:latest
```

## Requirements

- Hevy PRO subscription
- Hevy API key from https://hevy.com/settings?developer

## Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `HEVY_HTTP_ADDR` | No | `127.0.0.1:3000` (source) / `0.0.0.0:5000` (Docker) | Socket address to bind the HTTP server |
| `HEVY_MCP_PATH` | No | `/` | HTTP route path for the MCP endpoint. A leading `/` is added automatically if missing. |
| `HEVY_SESSION_TIMEOUT_SECS` | No | `3600` | Seconds of inactivity before a session expires |

## Authentication

Each client supplies its own Hevy API key; there is no server-wide key.

### OAuth 2.1

When no key is supplied, the server answers `401` with a `WWW-Authenticate` header pointing at `/.well-known/oauth-protected-resource`. MCP clients (e.g. Gemini) then run the standard flow: discovery, dynamic client registration (`/register`), PKCE authorization (`/authorize` shows a page where the user pastes their Hevy API key), and `/token`. Tokens are stateless encrypted blobs, so nothing is stored.

| Variable | Description |
|----------|-------------|
| `OAUTH_SECRET` | Encryption secret for codes/tokens. Set it, or tokens are invalidated on every restart. |
| `PUBLIC_HOST` | Public hostname without scheme (e.g. `hevy.example.com`). The scheme is always `https`. Taken from the `Host`/`X-Forwarded-Host` header if unset. |
| `OAUTH_CLIENT_ID` / `OAUTH_CLIENT_SECRET` | Optional pre-registered client for platforms that ask for a client id/secret instead of dynamic registration. |
| `OAUTH_REDIRECT_URIS` | Optional comma-separated redirect URI allowlist for that static client. |

Only OAuth access tokens (`Authorization: Bearer <token>`) are accepted.

## HTTP transport

The server exposes two endpoints on the configured path (default `/`):

### `POST {path}` -- MCP requests

Send JSON-RPC requests and notifications. The first request must be an `initialize` call. The response includes an `Mcp-Session-Id` header that must be sent on all subsequent requests.

```
POST /
Content-Type: application/json
Mcp-Session-Id: <session-id>
```

### Session lifecycle

- A new session is created on each `initialize` request and a UUID is returned via the `Mcp-Session-Id` response header.
- All non-initialize requests must include the `Mcp-Session-Id` header; requests without it are rejected.
- Sessions are automatically cleaned up after `HEVY_SESSION_TIMEOUT_SECS` seconds of inactivity (default 3600). A background task checks for expired sessions every 60 seconds.
- Sessions can also be deleted explicitly via the `DELETE` endpoint.

## Available tools

### Workouts
- `get_workouts` -- List workouts with pagination. Params: `page` (1-indexed), `page_size` (max 10).
- `get_workouts_count` -- Get the total number of workouts on the account.
- `get_workout_events` -- List workout update/delete events since a timestamp. Params: `page`, `page_size` (max 10), `since` (ISO 8601).
- `get_workout` -- Get a single workout by ID. Returns full details including exercises and sets.
- `create_workout` -- Create a workout. Required: `title`, `start_time`, `end_time` (ISO 8601), `exercises` with `sets`. Optional: `is_private`, `description`.
- `update_workout` -- Replace an existing workout by ID. Same fields as `create_workout` plus `id`.

### Routines
- `get_routines` -- List routines with pagination. Params: `page`, `page_size` (max 10).
- `get_routine` -- Get a single routine by ID, including exercises and set templates.
- `create_routine` -- Create a routine (workout template). Required: `title`, `folder_id`, `exercises` with `sets`. Optional: `notes`.
- `update_routine` -- Replace an existing routine by ID. Same fields as `create_routine` plus `id`.

### Exercise Templates
- `get_exercise_templates` -- List exercise templates from the Hevy library. Params: `page`, `page_size` (max 100).
- `get_exercise_template` -- Get an exercise template by ID, including muscle groups and equipment.
- `create_exercise_template` -- Create a custom exercise template. Required: `title`, `exercise_type`, `equipment_category`, `muscle_group`. Optional: `other_muscles`.

### Routine Folders
- `get_routine_folders` -- List routine folders with pagination. Params: `page`, `page_size` (max 10).
- `get_routine_folder` -- Get a routine folder by ID.
- `create_routine_folder` -- Create a folder to organize routines. Required: `title`.

### Exercise History
- `get_exercise_history` -- Get workout history for an exercise template. Required: `exercise_template_id`. Optional: `start_date`, `end_date` (ISO 8601).

## From source

```bash
cargo build --release
./target/release/hevy-mcp-server
```

The server binds to `127.0.0.1:3000` by default. Override with `HEVY_HTTP_ADDR`:

```bash
HEVY_HTTP_ADDR=0.0.0.0:8080 ./target/release/hevy-mcp-server
```

## License

MIT License - See [LICENSE](LICENSE) for details.

## References

- https://api.hevyapp.com/docs/
