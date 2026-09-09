#![forbid(unsafe_code)]

mod game_surface;

pub use game_surface::{
    serve_game_surface_stdio, serve_game_surface_stdio_blocking, GameSurfaceServer,
};

use std::fmt::Display;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use engine_observation::{snapshot, ObservationRequest, RenderSource};
use engine_time::{LogicalTime, Tau};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct SnapshotRequest {
    #[schemars(description = "Logical time in engine ticks")]
    pub logical_time_ticks: i64,
    #[schemars(description = "Presentation time in engine ticks")]
    pub tau_ticks: i64,
    #[schemars(description = "Output width in pixels; defaults to 960")]
    pub width: Option<u32>,
    #[schemars(description = "Output height in pixels; defaults to 720")]
    pub height: Option<u32>,
}

impl SnapshotRequest {
    fn observation_request(&self) -> ObservationRequest {
        ObservationRequest::new(self.width.unwrap_or(960), self.height.unwrap_or(720))
    }

    fn times(&self) -> (LogicalTime, Tau) {
        (
            LogicalTime::from_ticks(self.logical_time_ticks),
            Tau::from_ticks(self.tau_ticks),
        )
    }
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct MetadataRequest {
    #[schemars(description = "Logical time in engine ticks")]
    pub logical_time_ticks: i64,
    #[schemars(description = "Presentation time in engine ticks")]
    pub tau_ticks: i64,
    #[schemars(description = "Output width in pixels; defaults to 960")]
    pub width: Option<u32>,
    #[schemars(description = "Output height in pixels; defaults to 720")]
    pub height: Option<u32>,
}

impl From<MetadataRequest> for SnapshotRequest {
    fn from(request: MetadataRequest) -> Self {
        Self {
            logical_time_ticks: request.logical_time_ticks,
            tau_ticks: request.tau_ticks,
            width: request.width,
            height: request.height,
        }
    }
}

/// Generic MCP tools for any engine render source.
#[derive(Clone)]
pub struct ObservationServer<S> {
    source: S,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl<S> ObservationServer<S>
where
    S: RenderSource + Clone + Send + Sync + 'static,
    S::Error: Display,
{
    pub fn new(source: S) -> Self {
        Self {
            source,
            tool_router: Self::tool_router(),
        }
    }

    fn render_snapshot(
        &self,
        request: SnapshotRequest,
    ) -> Result<engine_observation::RenderSnapshot, McpError> {
        let (logical_time, tau) = request.times();
        snapshot(
            &self.source,
            logical_time,
            tau,
            request.observation_request(),
        )
        .map_err(|error| McpError::invalid_params(error.to_string(), None))
    }
}

#[tool_router]
impl<S> ObservationServer<S>
where
    S: RenderSource + Clone + Send + Sync + 'static,
    S::Error: Display,
{
    #[tool(description = "Render an explicit engine sample and return a PNG image plus metadata")]
    fn snapshot(
        &self,
        Parameters(request): Parameters<SnapshotRequest>,
    ) -> Result<CallToolResult, McpError> {
        let rendered = self.render_snapshot(request)?;
        let metadata = serde_json::json!({
            "logical_time_ticks": rendered.logical_time().ticks(),
            "tau_ticks": rendered.tau().ticks(),
            "width": rendered.width(),
            "height": rendered.height(),
            "vertex_count": rendered.vertex_count(),
            "triangle_count": rendered.triangle_count(),
        });
        Ok(CallToolResult::success(vec![
            ContentBlock::text(metadata.to_string()),
            ContentBlock::image(BASE64.encode(rendered.png()), "image/png"),
        ]))
    }

    #[tool(description = "Return metadata for an explicit engine render sample")]
    fn metadata(
        &self,
        Parameters(request): Parameters<MetadataRequest>,
    ) -> Result<String, McpError> {
        let rendered = self.render_snapshot(request.into())?;
        serde_json::to_string(&serde_json::json!({
            "logical_time_ticks": rendered.logical_time().ticks(),
            "tau_ticks": rendered.tau().ticks(),
            "width": rendered.width(),
            "height": rendered.height(),
            "vertex_count": rendered.vertex_count(),
            "triangle_count": rendered.triangle_count(),
        }))
        .map_err(|error| McpError::internal_error(error.to_string(), None))
    }
}

#[tool_handler]
impl<S> ServerHandler for ObservationServer<S>
where
    S: RenderSource + Clone + Send + Sync + 'static,
    S::Error: Display,
{
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Observe deterministic engine render sources at explicit LogicalTime and Tau values.",
        )
    }
}

/// Runs one generic observation source as a VS Code-compatible stdio MCP server.
pub async fn serve_stdio<S>(source: S) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: RenderSource + Clone + Send + Sync + 'static,
    S::Error: Display,
{
    let server = ObservationServer::new(source)
        .serve(rmcp::transport::stdio())
        .await?;
    server.waiting().await?;
    Ok(())
}

/// Runs the stdio server from a synchronous binary entrypoint.
pub fn serve_stdio_blocking<S>(source: S) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: RenderSource + Clone + Send + Sync + 'static,
    S::Error: Display,
{
    tokio::runtime::Runtime::new()?.block_on(serve_stdio(source))
}

#[cfg(test)]
mod tests {
    use super::{MetadataRequest, ObservationServer, SnapshotRequest};
    use engine_observation::RenderSource;
    use engine_presentation::RenderBatch;
    use engine_sdk::Frame;
    use engine_time::{LogicalTime, Tau};
    use rmcp::ServerHandler;

    #[derive(Clone)]
    struct EmptySource;

    impl RenderSource for EmptySource {
        type Error = std::convert::Infallible;

        fn render(
            &self,
            _logical_time: LogicalTime,
            tau: Tau,
        ) -> Result<Frame<RenderBatch>, Self::Error> {
            Ok(Frame::new(tau, RenderBatch::empty()))
        }
    }

    #[test]
    fn server_is_generic_over_an_empty_render_source() {
        let server = ObservationServer::new(EmptySource);
        assert!(server.get_info().instructions.is_some());
        let _snapshot = SnapshotRequest {
            logical_time_ticks: 0,
            tau_ticks: 0,
            width: Some(8),
            height: Some(8),
        };
        let _metadata = MetadataRequest {
            logical_time_ticks: 0,
            tau_ticks: 0,
            width: Some(8),
            height: Some(8),
        };
    }
}
