use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    serve as axum_serve,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
};
use thiserror::Error;
use tokio::net::TcpListener;
use tracing::info;

use crate::server::{HevyApiKey, HevyServer};

mod logging;
mod oauth;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) oauth: Arc<oauth::OAuth>,
}

async fn health() -> Response {
    (StatusCode::OK, "ok").into_response()
}

/// Validates the OAuth bearer token and injects the resolved Hevy API key into the request
/// extensions, where `HevyServer::client_for` finds it via the request `Parts`.
async fn require_bearer(State(state): State<AppState>, mut req: Request, next: Next) -> Response {
    match state.oauth.bearer_key(req.headers()) {
        Some(Ok(key)) => {
            req.extensions_mut().insert(HevyApiKey(key));
            next.run(req).await
        }
        Some(Err(())) => state.oauth.unauthorized(req.headers(), true),
        None => state.oauth.unauthorized(req.headers(), false),
    }
}

pub async fn serve(addr: SocketAddr) -> Result<(), HttpError> {
    let path = match env::var("HEVY_MCP_PATH") {
        Ok(p) if p.starts_with('/') => p,
        Ok(p) => format!("/{p}"),
        Err(_) => "/".to_string(),
    };

    // rmcp validates the Host header against `allowed_hosts` (DNS-rebinding protection) and
    // defaults to loopback only, which would reject the deployed hostname. Every MCP request
    // here requires an OAuth bearer token (not ambient browser credentials), so rebinding is
    // not a threat: allow any host unless HEVY_ALLOWED_HOSTS (comma-separated) is set.
    // Stateless mode with plain JSON responses: no Mcp-Session-Id, every POST returns
    // `application/json` (no SSE priming event). Some clients (e.g. Gemini) cannot parse an
    // SSE reply to POST and stall after `initialize`. `json_response` only takes effect
    // when `legacy_session_mode` is false. The tools here never emit server-initiated
    // messages, so nothing needs a session.
    let mut config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true);
    config = match env::var("HEVY_ALLOWED_HOSTS") {
        Ok(hosts) if !hosts.trim().is_empty() => config.with_allowed_hosts(
            hosts
                .split(',')
                .map(str::trim)
                .filter(|h| !h.is_empty())
                .map(String::from),
        ),
        _ => config.disable_allowed_hosts(),
    };

    let mcp_service = StreamableHttpService::new(
        || Ok(HevyServer::new()),
        Arc::new(NeverSessionManager::default()),
        config,
    );

    let state = AppState {
        oauth: Arc::new(oauth::OAuth::from_env(&path)),
    };

    let mcp_routes = Router::new()
        .route_service(&path, mcp_service)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_bearer,
        ));

    let app = Router::new()
        .route("/health", get(health))
        .route(
            "/.well-known/oauth-protected-resource",
            get(oauth::protected_resource),
        )
        .route(
            "/.well-known/oauth-protected-resource/*rest",
            get(oauth::protected_resource),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(oauth::authorization_server),
        )
        .route("/register", post(oauth::register))
        .route("/authorize", get(oauth::authorize_get).post(oauth::authorize_post))
        .route("/token", post(oauth::token))
        .with_state(state)
        .merge(mcp_routes)
        .layer(axum::middleware::from_fn(logging::log_requests));

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|err| HttpError::Bind(err.to_string()))?;
    info!("HTTP MCP server listening on {}", addr);
    axum_serve(listener, app)
        .await
        .map_err(|err| HttpError::Serve(err.to_string()))?;
    Ok(())
}

#[derive(Error, Debug)]
pub enum HttpError {
    #[error("failed to bind HTTP listener: {0}")]
    Bind(String),
    #[error("HTTP server error: {0}")]
    Serve(String),
}
