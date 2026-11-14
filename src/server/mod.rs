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
use crate::hevy::types::{ExerciseTemplatesResponse, RoutineFoldersResponse, RoutinesResponse};
pub mod args;
mod formatters;

use formatters::{
    format_exercise_history, format_exercise_template, format_folder, format_routine,
    format_workout, format_workout_events, format_workouts_page,
};

use args::{
    CreateExerciseTemplateParams, CreateRoutineFolderParams, CreateRoutineParams,
    CreateWorkoutParams, ExerciseHistoryParams, FolderIdParams, PaginationParams, RoutineIdParams,
    TemplateIdParams, UpdateRoutineParams, UpdateWorkoutParams, WorkoutEventsParams,
    WorkoutIdParams,
};

/// Formats a paginated list as `Page X of Y` followed by `---`-separated items.
fn format_page<T>(
    page: u32,
    page_count: u32,
    items: &[T],
    empty: &str,
    fmt: impl Fn(&T) -> String,
) -> String {
    let mut output = format!("Page {} of {}\n\n", page, page_count);
    if items.is_empty() {
        output.push_str(empty);
        return output;
    }
    for item in items {
        output.push_str(&fmt(item));
        output.push_str("\n---\n\n");
    }
    output
}

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

#[tool_router(vis = "pub")]
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
    #[tool(
        description = "List workout routines from Hevy with pagination. Routines are workout templates you can start workouts from."
    )]
    async fn get_routines(
        &self,
        Parameters(p): Parameters<PaginationParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client.get_routines(p.page, p.page_size).await {
            Ok(r) => {
                let r: crate::hevy::PaginatedResponse<RoutinesResponse> = r;
                text_result(format_page(
                    r.page,
                    r.page_count,
                    &r.data.routines,
                    "No routines found.",
                    format_routine,
                ))
            }
            Err(e) => api_error("Error fetching routines", e),
        }
    }

    #[tool(
        description = "Get a single routine by its ID. Returns full routine details including all exercises and set templates."
    )]
    async fn get_routine(
        &self,
        Parameters(p): Parameters<RoutineIdParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client.get_routine(&p.id).await {
            Ok(r) => text_result(format_routine(&r)),
            Err(e) => api_error("Error fetching routine", e),
        }
    }

    #[tool(
        description = "Create a new workout routine in Hevy. Routines serve as templates for workouts."
    )]
    async fn create_routine(
        &self,
        Parameters(p): Parameters<CreateRoutineParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client.create_routine(p.into()).await {
            Ok(r) => text_result(format!(
                "Routine created successfully!\n\n{}",
                format_routine(&r)
            )),
            Err(e) => api_error("Error creating Routine", e),
        }
    }

    #[tool(
        description = "Update an existing routine by ID. Replaces all routine data with the provided values."
    )]
    async fn update_routine(
        &self,
        Parameters(p): Parameters<UpdateRoutineParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        let (id, input) = p.into_parts();
        match client.update_routine(&id, input).await {
            Ok(r) => text_result(format!(
                "Routine updated successfully!\n\n{}",
                format_routine(&r)
            )),
            Err(e) => api_error("Error updating Routine", e),
        }
    }

    #[tool(
        description = "List routine folders from Hevy. Folders help organize your workout routines."
    )]
    async fn get_routine_folders(
        &self,
        Parameters(p): Parameters<PaginationParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client.get_routine_folders(p.page, p.page_size).await {
            Ok(r) => {
                let r: crate::hevy::PaginatedResponse<RoutineFoldersResponse> = r;
                text_result(format_page(
                    r.page,
                    r.page_count,
                    &r.data.routine_folders,
                    "No routine folders found.",
                    format_folder,
                ))
            }
            Err(e) => api_error("Error fetching routine folders", e),
        }
    }

    #[tool(description = "Get a single routine folder by its ID.")]
    async fn get_routine_folder(
        &self,
        Parameters(p): Parameters<FolderIdParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client.get_routine_folder(&p.id).await {
            Ok(f) => text_result(format_folder(&f)),
            Err(e) => api_error("Error fetching routine folder", e),
        }
    }

    #[tool(description = "Create a new folder to organize workout routines in Hevy.")]
    async fn create_routine_folder(
        &self,
        Parameters(p): Parameters<CreateRoutineFolderParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client.create_routine_folder(p.into()).await {
            Ok(f) => text_result(format!(
                "Folder created successfully!\n\n{}",
                format_folder(&f)
            )),
            Err(e) => api_error("Error creating Folder", e),
        }
    }

    #[tool(
        description = "List exercise templates from Hevy's exercise library. These are the exercises you can add to workouts and routines."
    )]
    async fn get_exercise_templates(
        &self,
        Parameters(p): Parameters<PaginationParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client.get_exercise_templates(p.page, p.page_size).await {
            Ok(r) => {
                let r: crate::hevy::PaginatedResponse<ExerciseTemplatesResponse> = r;
                text_result(format_page(
                    r.page,
                    r.page_count,
                    &r.data.exercise_templates,
                    "No exercise templates found.",
                    format_exercise_template,
                ))
            }
            Err(e) => api_error("Error fetching exercise templates", e),
        }
    }

    #[tool(
        description = "Get a single exercise template by its ID. Returns exercise details including muscle groups and equipment."
    )]
    async fn get_exercise_template(
        &self,
        Parameters(p): Parameters<TemplateIdParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client.get_exercise_template(&p.id).await {
            Ok(t) => text_result(format_exercise_template(&t)),
            Err(e) => api_error("Error fetching exercise template", e),
        }
    }

    #[tool(description = "Create a new custom exercise template.")]
    async fn create_exercise_template(
        &self,
        Parameters(p): Parameters<CreateExerciseTemplateParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client = self.client_for(&ctx)?;
        match client.create_exercise_template(p.into()).await {
            Ok(c) => text_result(format!(
                "Exercise template created successfully!\nID: {}",
                c.id
            )),
            Err(e) => api_error("Error creating exercise template", e),
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
