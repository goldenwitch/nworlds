use std::fmt::Display;
use std::sync::{Arc, Mutex};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use engine_observation::ObservationRequest;
use engine_surface::{
    AppendRequest, AppendResult, BranchDescriptor, BranchId, GameSession, GameSurface, JournalView,
    Revision, SurfaceManifest, SurfaceSnapshotRequest,
};
use engine_time::{LogicalTime, Tau};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::{de::DeserializeOwned, Deserialize};
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

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct SurfaceBatchRequest {
    pub calls: Vec<SurfaceBatchCall>,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct SurfaceBatchCall {
    pub name: String,
    pub arguments: Value,
}

struct BatchCallOutput {
    value: Value,
    image: Option<Vec<u8>>,
}

/// Generic MCP tools for a package-owned GameSurface.
#[derive(Clone)]
pub struct GameSurfaceServer<S>
where
    S: GameSurface,
{
    session: Arc<Mutex<GameSession<S>>>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl<S> GameSurfaceServer<S>
where
    S: GameSurface + Send + Sync + 'static,
    S::Context: Send + Sync,
    S::Fact: Send + Sync,
    S::View: Send + Sync,
    S::Error: Display,
{
    pub fn new(session: GameSession<S>) -> Self {
        Self {
            session: Arc::new(Mutex::new(session)),
            tool_router: Self::tool_router(),
        }
    }

    fn with_surface<T>(
        &self,
        operation: impl FnOnce(&GameSession<S>) -> Result<T, engine_surface::GameSessionError<S::Error>>,
    ) -> Result<T, McpError> {
        let session = self
            .session
            .lock()
            .map_err(|_| McpError::internal_error("session lock poisoned", None))?;
        operation(&session).map_err(|error| McpError::invalid_params(error.to_string(), None))
    }

    fn with_surface_mut<T>(
        &self,
        operation: impl FnOnce(
            &mut GameSession<S>,
        ) -> Result<T, engine_surface::GameSessionError<S::Error>>,
    ) -> Result<T, McpError> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| McpError::internal_error("session lock poisoned", None))?;
        operation(&mut session).map_err(|error| McpError::invalid_params(error.to_string(), None))
    }

    fn execute_batch_call(
        session: &mut GameSession<S>,
        call: &SurfaceBatchCall,
    ) -> Result<BatchCallOutput, McpError> {
        let output = match call.name.as_str() {
            "game_manifest" => BatchCallOutput {
                value: manifest_value(session.manifest()),
                image: None,
            },
            "view_read" => BatchCallOutput {
                value: session
                    .view()
                    .map_err(|error| McpError::invalid_params(error.to_string(), None))?,
                image: None,
            },
            "view_update" => {
                let request: ViewUpdateRequest = decode_arguments(&call.arguments)?;
                BatchCallOutput {
                    value: session
                        .update_view(request.update)
                        .map_err(|error| McpError::invalid_params(error.to_string(), None))?,
                    image: None,
                }
            }
            "journal_read" => {
                let request: BranchIdRequest = decode_arguments(&call.arguments)?;
                BatchCallOutput {
                    value: journal_value(
                        session
                            .journal(request.branch_id)
                            .map_err(|error| McpError::invalid_params(error.to_string(), None))?,
                    ),
                    image: None,
                }
            }
            "branch_open" => {
                let request: BranchOpenRequest = decode_arguments(&call.arguments)?;
                let descriptor = session
                    .begin_counterfactual(
                        request.parent_branch_id,
                        request.expected_revision,
                        LogicalTime::from_ticks(request.fork_time_ticks),
                    )
                    .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
                BatchCallOutput {
                    value: descriptor_value(descriptor),
                    image: None,
                }
            }
            "branch_preview_append" => {
                let request: BranchAppendRequest = decode_arguments(&call.arguments)?;
                let preview = session
                    .preview_append(request.into())
                    .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
                BatchCallOutput {
                    value: json!({
                        "branch_id": preview.branch_id,
                        "current_revision": preview.current_revision,
                        "logical_time_ticks": preview.logical_time.ticks(),
                        "fact_count": preview.fact_count,
                    }),
                    image: None,
                }
            }
            "branch_append" => {
                let request: BranchAppendRequest = decode_arguments(&call.arguments)?;
                if request.branch_id == 0 {
                    return Err(McpError::invalid_params(
                        "branch_append targets speculative branches; use actual_append for the actual line",
                        None,
                    ));
                }
                let result = session
                    .commit_append(request.into())
                    .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
                BatchCallOutput {
                    value: append_result_value(result),
                    image: None,
                }
            }
            "actual_append" => {
                let request: ActualAppendRequest = decode_arguments(&call.arguments)?;
                let result = session
                    .commit_append(request.into())
                    .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
                BatchCallOutput {
                    value: append_result_value(result),
                    image: None,
                }
            }
            "surface_snapshot" => {
                let request: SurfaceSnapshotToolRequest = decode_arguments(&call.arguments)?;
                let snapshot = session
                    .observe(request.clone().into())
                    .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
                BatchCallOutput {
                    value: snapshot_metadata(&snapshot),
                    image: Some(snapshot.png().to_vec()),
                }
            }
            "branch_compare" => {
                let request: BranchCompareRequest = decode_arguments(&call.arguments)?;
                let left = session
                    .observe(request.left.clone().into())
                    .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
                let right = session
                    .observe(request.right.clone().into())
                    .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
                BatchCallOutput {
                    value: compare_value(&request, &left, &right),
                    image: None,
                }
            }
            "branch_discard" => {
                let request: BranchDiscardRequest = decode_arguments(&call.arguments)?;
                session
                    .discard_branch(request.branch_id)
                    .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
                BatchCallOutput {
                    value: json!({ "discarded_branch_id": request.branch_id }),
                    image: None,
                }
            }
            _ => {
                return Err(McpError::invalid_params(
                    format!("unsupported surface batch call: {}", call.name),
                    None,
                ));
            }
        };
        Ok(output)
    }
}

#[tool_router]
impl<S> GameSurfaceServer<S>
where
    S: GameSurface + Send + Sync + 'static,
    S::Context: Send + Sync,
    S::Fact: Send + Sync,
    S::View: Send + Sync,
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

    #[tool(description = "Execute existing surface calls in order under one local session lock")]
    fn surface_batch(
        &self,
        Parameters(request): Parameters<SurfaceBatchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| McpError::internal_error("session lock poisoned", None))?;
        let mut results = Vec::new();
        let mut images = Vec::new();
        let mut failed = None;

        for (index, call) in request.calls.iter().enumerate() {
            match Self::execute_batch_call(&mut session, call) {
                Ok(output) => {
                    results.push(json!({
                        "index": index,
                        "name": call.name,
                        "result": output.value,
                    }));
                    if let Some(image) = output.image {
                        images.push((index, image));
                    }
                }
                Err(error) => {
                    failed = Some(json!({
                        "index": index,
                        "name": call.name,
                        "error": error.to_string(),
                    }));
                    break;
                }
            }
        }

        let summary = if let Some(failed) = failed {
            json!({
                "ok": false,
                "completed": results,
                "failed": failed,
            })
        } else {
            json!({ "ok": true, "results": results })
        };
        let mut content = vec![ContentBlock::text(summary.to_string())];
        for (index, image) in images {
            content.push(ContentBlock::text(
                json!({ "image_call_index": index }).to_string(),
            ));
            content.push(ContentBlock::image(BASE64.encode(image), "image/png"));
        }
        Ok(CallToolResult::success(content))
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
    S: GameSurface + Send + Sync + 'static,
    S::Context: Send + Sync,
    S::Fact: Send + Sync,
    S::View: Send + Sync,
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
    serde_json::to_string(&descriptor_value(descriptor))
        .map_err(|error| McpError::internal_error(error.to_string(), None))
}

fn manifest_value(manifest: SurfaceManifest) -> Value {
    json!({
        "name": manifest.name,
        "capabilities": manifest.capabilities,
        "fact_schema": manifest.fact_schema,
        "view_schema": manifest.view_schema,
    })
}

fn decode_arguments<T: DeserializeOwned>(arguments: &Value) -> Result<T, McpError> {
    serde_json::from_value(arguments.clone())
        .map_err(|error| McpError::invalid_params(error.to_string(), None))
}

fn descriptor_value(descriptor: BranchDescriptor) -> Value {
    json!({
        "branch_id": descriptor.branch_id,
        "revision": descriptor.revision,
        "parent": descriptor.parent,
        "kind": format!("{:?}", descriptor.kind),
        "fork_boundary_ticks": descriptor.fork_boundary.map(|time| time.ticks()),
    })
}

fn journal_value(view: JournalView) -> Value {
    json!({
        "branch_id": view.descriptor.branch_id,
        "revision": view.descriptor.revision,
        "parent": view.descriptor.parent,
        "kind": format!("{:?}", view.descriptor.kind),
        "fork_boundary_ticks": view.descriptor.fork_boundary.map(|time| time.ticks()),
        "facts": view.facts,
    })
}

fn append_result_value(result: AppendResult) -> Value {
    json!({
        "branch_id": result.branch_id,
        "new_revision": result.new_revision,
        "logical_time_ticks": result.logical_time.ticks(),
        "fact_count": result.fact_count,
    })
}

fn snapshot_metadata(snapshot: &engine_observation::RenderSnapshot) -> Value {
    json!({
        "logical_time_ticks": snapshot.logical_time().ticks(),
        "tau_ticks": snapshot.tau().ticks(),
        "width": snapshot.width(),
        "height": snapshot.height(),
        "vertex_count": snapshot.vertex_count(),
        "triangle_count": snapshot.triangle_count(),
    })
}

fn compare_value(
    request: &BranchCompareRequest,
    left: &engine_observation::RenderSnapshot,
    right: &engine_observation::RenderSnapshot,
) -> Value {
    json!({
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
    })
}

pub async fn serve_game_surface_stdio<S>(
    session: GameSession<S>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: GameSurface + Send + Sync + 'static,
    S::Context: Send + Sync,
    S::Fact: Send + Sync,
    S::View: Send + Sync,
    S::Error: Display,
{
    let server = GameSurfaceServer::new(session)
        .serve(rmcp::transport::stdio())
        .await?;
    server.waiting().await?;
    Ok(())
}

pub fn serve_game_surface_stdio_blocking<S>(
    session: GameSession<S>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: GameSurface + Send + Sync + 'static,
    S::Context: Send + Sync,
    S::Fact: Send + Sync,
    S::View: Send + Sync,
    S::Error: Display,
{
    tokio::runtime::Runtime::new()?.block_on(serve_game_surface_stdio(session))
}

#[cfg(test)]
mod tests {
    use super::{GameSurfaceServer, SurfaceBatchCall};
    use engine_branches::Branch;
    use engine_observation::RenderSnapshot;
    use engine_sdk::Context;
    use engine_surface::{GameSession, GameSurface, SurfaceManifest, SurfaceSnapshotRequest};
    use serde_json::{json, Value};

    #[derive(Clone, Copy, Debug, Default)]
    struct BatchSurface;

    impl GameSurface for BatchSurface {
        type Context = ();
        type Fact = u8;
        type View = u8;
        type Error = String;

        fn manifest(&self) -> SurfaceManifest {
            SurfaceManifest {
                name: "batch-test".to_owned(),
                capabilities: Vec::new(),
                fact_schema: json!({ "type": "integer" }),
                view_schema: json!({ "type": "object" }),
            }
        }

        fn default_view(&self) -> Self::View {
            0
        }

        fn view(&self, view: &Self::View) -> Result<Value, Self::Error> {
            Ok(json!({ "value": view }))
        }

        fn update_view(&self, view: &mut Self::View, update: Value) -> Result<Value, Self::Error> {
            *view = update
                .get("value")
                .and_then(Value::as_u64)
                .and_then(|value| u8::try_from(value).ok())
                .ok_or_else(|| "batch test view needs a value".to_owned())?;
            self.view(view)
        }

        fn encode_fact(&self, fact: &Self::Fact) -> Value {
            json!(fact)
        }

        fn decode_fact(&self, value: Value) -> Result<Self::Fact, Self::Error> {
            value
                .as_u64()
                .and_then(|value| u8::try_from(value).ok())
                .ok_or_else(|| "batch test facts are u8 numbers".to_owned())
        }

        fn observe(
            &self,
            _branch: &Branch<(), u8>,
            _view: &Self::View,
            _request: SurfaceSnapshotRequest,
        ) -> Result<RenderSnapshot, Self::Error> {
            Err("batch test observation is unused".to_owned())
        }
    }

    #[test]
    fn batch_calls_are_ordered_on_one_local_session() {
        let mut session = GameSession::new(
            BatchSurface,
            Branch::new(Context::new(()), engine_journal::Journal::empty()),
        )
        .expect("batch test session should initialize");

        let view_update = SurfaceBatchCall {
            name: "view_update".to_owned(),
            arguments: json!({ "update": { "value": 7 } }),
        };
        GameSurfaceServer::<BatchSurface>::execute_batch_call(&mut session, &view_update)
            .expect("view update should succeed");

        let view_read = SurfaceBatchCall {
            name: "view_read".to_owned(),
            arguments: json!({}),
        };
        let view = GameSurfaceServer::<BatchSurface>::execute_batch_call(&mut session, &view_read)
            .expect("view read should succeed");
        assert_eq!(view.value["value"], 7);

        let append = SurfaceBatchCall {
            name: "actual_append".to_owned(),
            arguments: json!({
                "expected_revision": 0,
                "logical_time_ticks": 1,
                "facts": [3]
            }),
        };
        GameSurfaceServer::<BatchSurface>::execute_batch_call(&mut session, &append)
            .expect("append should succeed");

        let journal = SurfaceBatchCall {
            name: "journal_read".to_owned(),
            arguments: json!({ "branch_id": 0 }),
        };
        let journal = GameSurfaceServer::<BatchSurface>::execute_batch_call(&mut session, &journal)
            .expect("journal read should succeed");
        assert_eq!(journal.value["revision"], 1);
        assert_eq!(journal.value["facts"][0]["fact"], 3);
    }
}
