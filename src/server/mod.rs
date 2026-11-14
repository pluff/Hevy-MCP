//! rmcp-based MCP server (migration scaffold; tools are ported in later parts).

use std::sync::Arc;

use dashmap::DashMap;
use rmcp::{
    ErrorData, RoleServer,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ErrorCode},
    service::RequestContext,
    tool, tool_handler, tool_router,
};

use crate::hevy::HevyClient;
use crate::router::formatters::format_workout;
use crate::router::handlers::exercise_history::format_exercise_history;
use crate::router::handlers::workouts::{format_workout_events, format_workouts_page};

pub mod args;

use args::{
    CreateWorkoutParams, ExerciseHistoryParams, PaginationParams, UpdateWorkoutParams,
    WorkoutEventsParams, WorkoutIdParams,
};

/// Text result for a successful tool call.
fn text_result(text: String) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

/// Tool-level error result (`isError: true`) for Hevy API failures.
fn api_error(context: &str, err: impl std::fmt::Display) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::error(vec![ContentBlock::text(format!(
        "{}: {}",
        context, err
    ))]))
}

/// Hevy API key resolved from the OAuth bearer token. The HTTP layer (Part 4) must insert it
/// into the request extensions (`http::request::Parts::extensions`) before the MCP service runs.
#[derive(Clone, Debug)]
pub struct HevyApiKey(pub String);

#[derive(Clone)]
pub struct HevyServer {
    client_cache: Arc<DashMap<String, Arc<HevyClient>>>,
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

#[tool_router]
impl HevyServer {
    #[tool(
        description = "List workouts from Hevy with pagination. Returns workout summaries including title, date, and exercises."
    )]
    async fn get_workouts(
        &self,
        Parameters(p): Parameters<PaginationParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client.get_workouts(p.page, p.page_size).await {
            Ok(r) => text_result(format_workouts_page(&r)),
            Err(e) => api_error("Error fetching workouts", e),
        }
    }

    #[tool(description = "Get the total number of workouts on the account.")]
    async fn get_workouts_count(
        &self,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client.get_workouts_count().await {
            Ok(r) => text_result(format!("Workout count: {}", r.workout_count)),
            Err(e) => api_error("Error fetching workout count", e),
        }
    }

    #[tool(
        description = "List workout update and delete events since a given date, with pagination."
    )]
    async fn get_workout_events(
        &self,
        Parameters(p): Parameters<WorkoutEventsParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client
            .get_workout_events(p.page, p.page_size, p.since)
            .await
        {
            Ok(r) => text_result(format_workout_events(&r)),
            Err(e) => api_error("Error fetching workout events", e),
        }
    }

    #[tool(
        description = "Get a single workout by its ID. Returns full workout details including all exercises and sets."
    )]
    async fn get_workout(
        &self,
        Parameters(p): Parameters<WorkoutIdParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client.get_workout(&p.id).await {
            Ok(w) => text_result(format_workout(&w)),
            Err(e) => api_error("Error fetching workout", e),
        }
    }

    #[tool(
        description = "Create a new workout in Hevy. Requires title, start_time, end_time (ISO 8601), and exercises with sets."
    )]
    async fn create_workout(
        &self,
        Parameters(p): Parameters<CreateWorkoutParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client.create_workout(p.into()).await {
            Ok(w) => text_result(format!(
                "Workout created successfully!\n\n{}",
                format_workout(&w)
            )),
            Err(e) => api_error("Error creating Workout", e),
        }
    }

    #[tool(
        description = "Update an existing workout by ID. Replaces all workout data with the provided values."
    )]
    async fn update_workout(
        &self,
        Parameters(p): Parameters<UpdateWorkoutParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        let (id, input) = p.into_parts();
        match client.update_workout(&id, input).await {
            Ok(w) => text_result(format!(
                "Workout updated successfully!\n\n{}",
                format_workout(&w)
            )),
            Err(e) => api_error("Error updating Workout", e),
        }
    }

    #[tool(description = "Get exercise history for a specific exercise template.")]
    async fn get_exercise_history(
        &self,
        Parameters(p): Parameters<ExerciseHistoryParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client
            .get_exercise_history(&p.exercise_template_id, p.start_date, p.end_date)
            .await
        {
            Ok(r) => text_result(format_exercise_history(&r)),
            Err(e) => api_error("Error fetching exercise history", e),
        }
    }
}

#[tool_handler(
    router = self.tool_router,
    name = "hevy-mcp-server",
    instructions = "Hevy MCP Server - Interact with the Hevy fitness API. \
        Available operations include managing workouts, routines, \
        exercise templates, and folders. Requires a valid Hevy API key."
)]
impl rmcp::ServerHandler for HevyServer {}
