//! rmcp-based MCP server (migration scaffold; tools are ported in later parts).

use std::sync::Arc;

use dashmap::DashMap;
use rmcp::{
    ErrorData, RoleServer, handler::server::router::tool::ToolRouter, model::ErrorCode,
    service::RequestContext, tool_handler, tool_router,
};

use crate::hevy::HevyClient;

/// Hevy API key resolved from the OAuth bearer token. The HTTP layer (Part 4) must insert it
/// into the request extensions (`http::request::Parts::extensions`) before the MCP service runs.
#[derive(Clone, Debug)]
pub struct HevyApiKey(pub String);

#[derive(Clone)]
pub struct HevyServer {
    client_cache: Arc<DashMap<String, Arc<HevyClient>>>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl Default for HevyServer {
    fn default() -> Self {
        Self::new()
    }
}

impl HevyServer {
    pub fn new() -> Self {
        Self {
            client_cache: Arc::new(DashMap::new()),
            tool_router: Self::tool_router(),
        }
    }

    pub fn get_or_create_client(&self, api_key: &str) -> Arc<HevyClient> {
        self.client_cache
            .entry(api_key.to_string())
            .or_insert_with(|| Arc::new(HevyClient::new(api_key)))
            .clone()
    }

    /// Extracts the Hevy API key from the request context and returns the cached client.
    #[allow(dead_code)]
    pub(crate) fn client_for(
        &self,
        ctx: &RequestContext<RoleServer>,
    ) -> Result<Arc<HevyClient>, ErrorData> {
        let key = ctx
            .extensions
            .get::<http::request::Parts>()
            .and_then(|parts| parts.extensions.get::<HevyApiKey>())
            .ok_or_else(|| {
                ErrorData::new(ErrorCode::INVALID_REQUEST, "Missing Hevy API key", None)
            })?;
        Ok(self.get_or_create_client(&key.0))
    }
}

#[tool_router(allow_empty)]
impl HevyServer {}

#[tool_handler(
    router = self.tool_router,
    name = "hevy-mcp-server",
    instructions = "Hevy MCP Server - Interact with the Hevy fitness API. \
        Available operations include managing workouts, routines, \
        exercise templates, and folders. Requires a valid Hevy API key."
)]
impl rmcp::ServerHandler for HevyServer {}
