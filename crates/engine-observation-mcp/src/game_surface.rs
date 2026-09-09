use std::fmt::Display;
use std::sync::{Arc, Mutex};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use engine_observation::ObservationRequest;
use engine_surface::{AppendRequest, BranchId, GameSurface, Revision, SurfaceSnapshotRequest};
use engine_time::{LogicalTime, Tau};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct SurfaceSnapshotToolRequest {
    pub branch_id: BranchId,
    pub logical_time_ticks: i64,
    pub tau_ticks: i64,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl From<SurfaceSnapshotToolRequest> for SurfaceSnapshotRequest {
    fn from(request: SurfaceSnapshotToolRequest) -> Self {
        Self {
            branch_id: request.branch_id,
            logical_time: LogicalTime::from_ticks(request.logical_time_ticks),
            tau: Tau::from_ticks(request.tau_ticks),
            observation: ObservationRequest::new(
                request.width.unwrap_or(960),
                request.height.unwrap_or(720),
            ),
        }
    }
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct BranchOpenRequest {
    pub parent_branch_id: BranchId,
    pub expected_revision: Revision,
    pub fork_time_ticks: i64,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct BranchAppendRequest {
    pub branch_id: BranchId,
    pub expected_revision: Revision,
    pub logical_time_ticks: i64,
    pub facts: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct ActualAppendRequest {
    pub expected_revision: Revision,
    pub logical_time_ticks: i64,
    pub facts: Vec<Value>,
}

impl From<ActualAppendRequest> for AppendRequest {
    fn from(request: ActualAppendRequest) -> Self {
        Self {
            branch_id: 0,
            expected_revision: request.expected_revision,
            logical_time: LogicalTime::from_ticks(request.logical_time_ticks),
            facts: request.facts,
        }
    }
}

impl From<BranchAppendRequest> for AppendRequest {
    fn from(request: BranchAppendRequest) -> Self {
        Self {
            branch_id: request.branch_id,
            expected_revision: request.expected_revision,
            logical_time: LogicalTime::from_ticks(request.logical_time_ticks),
            facts: request.facts,
        }
    }
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct BranchDiscardRequest {
    pub branch_id: BranchId,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct BranchCompareRequest {
    pub left: SurfaceSnapshotToolRequest,
    pub right: SurfaceSnapshotToolRequest,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct ViewUpdateRequest {
    pub update: Value,
}

/// Generic MCP tools for a package-owned GameSurface.
#[derive(Clone)]
pub struct GameSurfaceServer<S> {
    surface: Arc<Mutex<S>>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl<S> GameSurfaceServer<S>
where
    S: GameSurface + Send + 'static,
    S::Error: Display,
{
    pub fn new(surface: S) -> Self {
        Self {
            surface: Arc::new(Mutex::new(surface)),
            tool_router: Self::tool_router(),
        }
    }

    fn with_surface<T>(
        &self,
        operation: impl FnOnce(&S) -> Result<T, S::Error>,
    ) -> Result<T, McpError> {
        let surface = self
            .surface
            .lock()
            .map_err(|_| McpError::internal_error("surface lock poisoned", None))?;
        operation(&surface).map_err(|error| McpError::invalid_params(error.to_string(), None))
    }

    fn with_surface_mut<T>(
        &self,
        operation: impl FnOnce(&mut S) -> Result<T, S::Error>,
    ) -> Result<T, McpError> {
        let mut surface = self
            .surface
            .lock()
            .map_err(|_| McpError::internal_error("surface lock poisoned", None))?;
        operation(&mut surface).map_err(|error| McpError::invalid_params(error.to_string(), None))
    }
}

#[tool_router]
impl<S> GameSurfaceServer<S>
where
    S: GameSurface + Send + 'static,
    S::Error: Display,
{
    #[tool(description = "Describe the game surface, capabilities, and fact schema")]
    fn game_manifest(&self) -> Result<String, McpError> {
        let manifest = self.with_surface(|surface| Ok(surface.manifest()))?;
        serde_json::to_string(&json!({
            "name": manifest.name,
            "capabilities": manifest.capabilities,
            "fact_schema": manifest.fact_schema,
            "view_schema": manifest.view_schema,
        }))
        .map_err(|error| McpError::internal_error(error.to_string(), None))
    }

    #[tool(description = "Read the current throwaway presentation view")]
    fn view_read(&self) -> Result<String, McpError> {
        let view = self.with_surface(|surface| surface.view())?;
        serde_json::to_string(&view)
            .map_err(|error| McpError::internal_error(error.to_string(), None))
    }

    #[tool(description = "Update the throwaway presentation view without writing the journal")]
    fn view_update(
        &self,
        Parameters(request): Parameters<ViewUpdateRequest>,
    ) -> Result<String, McpError> {
        let view = self.with_surface_mut(|surface| surface.update_view(request.update))?;
        serde_json::to_string(&view)
            .map_err(|error| McpError::internal_error(error.to_string(), None))
    }

    #[tool(description = "Read the package-owned journal view for one branch")]
    fn journal_read(
        &self,
        Parameters(request): Parameters<BranchIdRequest>,
    ) -> Result<String, McpError> {
        let view = self.with_surface(|surface| surface.journal(request.branch_id))?;
        serde_json::to_string(&json!({
            "branch_id": view.descriptor.branch_id,
            "revision": view.descriptor.revision,
            "parent": view.descriptor.parent,
            "kind": format!("{:?}", view.descriptor.kind),
            "fork_boundary_ticks": view.descriptor.fork_boundary.map(|time| time.ticks()),
            "facts": view.facts,
        }))
        .map_err(|error| McpError::internal_error(error.to_string(), None))
    }

    #[tool(description = "Open a counterfactual branch from an existing revision")]
    fn branch_open(
        &self,
        Parameters(request): Parameters<BranchOpenRequest>,
    ) -> Result<String, McpError> {
        let descriptor = self.with_surface_mut(|surface| {
            surface.begin_counterfactual(
                request.parent_branch_id,
                request.expected_revision,
                LogicalTime::from_ticks(request.fork_time_ticks),
            )
        })?;
        descriptor_json(descriptor)
    }

    #[tool(description = "Validate facts against a branch without committing them")]
    fn branch_preview_append(
        &self,
        Parameters(request): Parameters<BranchAppendRequest>,
    ) -> Result<String, McpError> {
        let preview = self.with_surface(|surface| surface.preview_append(request.into()))?;
        serde_json::to_string(&json!({
            "branch_id": preview.branch_id,
            "current_revision": preview.current_revision,
            "logical_time_ticks": preview.logical_time.ticks(),
            "fact_count": preview.fact_count,
        }))
        .map_err(|error| McpError::internal_error(error.to_string(), None))
    }

    #[tool(description = "Commit validated facts to a speculative branch")]
    fn branch_append(
        &self,
        Parameters(request): Parameters<BranchAppendRequest>,
    ) -> Result<String, McpError> {
        if request.branch_id == 0 {
            return Err(McpError::invalid_params(
                "branch_append targets speculative branches; use actual_append for the actual line",
                None,
            ));
        }
        let result = self.with_surface_mut(|surface| surface.commit_append(request.into()))?;
        serde_json::to_string(&json!({
            "branch_id": result.branch_id,
            "new_revision": result.new_revision,
            "logical_time_ticks": result.logical_time.ticks(),
            "fact_count": result.fact_count,
        }))
        .map_err(|error| McpError::internal_error(error.to_string(), None))
    }

    #[tool(description = "Commit validated facts to the actual game history")]
    fn actual_append(
        &self,
        Parameters(request): Parameters<ActualAppendRequest>,
    ) -> Result<String, McpError> {
        let result = self.with_surface_mut(|surface| surface.commit_append(request.into()))?;
        serde_json::to_string(&json!({
            "branch_id": result.branch_id,
            "new_revision": result.new_revision,
            "logical_time_ticks": result.logical_time.ticks(),
            "fact_count": result.fact_count,
        }))
        .map_err(|error| McpError::internal_error(error.to_string(), None))
    }

    #[tool(description = "Render an explicit branch sample as PNG plus metadata")]
    fn surface_snapshot(
        &self,
        Parameters(request): Parameters<SurfaceSnapshotToolRequest>,
    ) -> Result<CallToolResult, McpError> {
        let snapshot = self.with_surface(|surface| surface.observe(request.into()))?;
        let metadata = json!({
            "logical_time_ticks": snapshot.logical_time().ticks(),
            "tau_ticks": snapshot.tau().ticks(),
            "width": snapshot.width(),
            "height": snapshot.height(),
            "vertex_count": snapshot.vertex_count(),
            "triangle_count": snapshot.triangle_count(),
        });
        Ok(CallToolResult::success(vec![
            ContentBlock::text(metadata.to_string()),
            ContentBlock::image(BASE64.encode(snapshot.png()), "image/png"),
        ]))
    }

    #[tool(description = "Compare two explicit branch render samples")]
    fn branch_compare(
        &self,
        Parameters(request): Parameters<BranchCompareRequest>,
    ) -> Result<String, McpError> {
        let left = self.with_surface(|surface| surface.observe(request.left.clone().into()))?;
        let right = self.with_surface(|surface| surface.observe(request.right.clone().into()))?;
        serde_json::to_string(&json!({
            "same_png": left.png() == right.png(),
            "same_metadata": left.logical_time() == right.logical_time()
                && left.tau() == right.tau()
                && left.vertex_count() == right.vertex_count()
                && left.triangle_count() == right.triangle_count(),
            "left": {
                "branch_id": request.left.branch_id,
                "logical_time_ticks": left.logical_time().ticks(),
                "tau_ticks": left.tau().ticks(),
                "vertex_count": left.vertex_count(),
                "triangle_count": left.triangle_count(),
            },
            "right": {
                "branch_id": request.right.branch_id,
                "logical_time_ticks": right.logical_time().ticks(),
                "tau_ticks": right.tau().ticks(),
                "vertex_count": right.vertex_count(),
                "triangle_count": right.triangle_count(),
            }
        }))
        .map_err(|error| McpError::internal_error(error.to_string(), None))
    }

    #[tool(description = "Discard a speculative branch and keep the actual history")]
    fn branch_discard(
        &self,
        Parameters(request): Parameters<BranchDiscardRequest>,
    ) -> Result<String, McpError> {
        self.with_surface_mut(|surface| surface.discard_branch(request.branch_id))?;
        Ok(json!({ "discarded_branch_id": request.branch_id }).to_string())
    }
}

#[tool_handler]
impl<S> ServerHandler for GameSurfaceServer<S>
where
    S: GameSurface + Send + 'static,
    S::Error: Display,
{
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Observe, author, branch, preview, commit, and discard immutable game histories.",
        )
    }
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct BranchIdRequest {
    pub branch_id: BranchId,
}

fn descriptor_json(descriptor: engine_surface::BranchDescriptor) -> Result<String, McpError> {
    serde_json::to_string(&json!({
        "branch_id": descriptor.branch_id,
        "revision": descriptor.revision,
        "parent": descriptor.parent,
        "kind": format!("{:?}", descriptor.kind),
        "fork_boundary_ticks": descriptor.fork_boundary.map(|time| time.ticks()),
    }))
    .map_err(|error| McpError::internal_error(error.to_string(), None))
}

pub async fn serve_game_surface_stdio<S>(
    surface: S,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: GameSurface + Send + 'static,
    S::Error: Display,
{
    let server = GameSurfaceServer::new(surface)
        .serve(rmcp::transport::stdio())
        .await?;
    server.waiting().await?;
    Ok(())
}

pub fn serve_game_surface_stdio_blocking<S>(
    surface: S,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: GameSurface + Send + 'static,
    S::Error: Display,
{
    tokio::runtime::Runtime::new()?.block_on(serve_game_surface_stdio(surface))
}
