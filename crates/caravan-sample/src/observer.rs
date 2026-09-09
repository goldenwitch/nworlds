use std::convert::Infallible;

use caravan_domain::{ActorId, ActorKind, GameJournalEntry, Terrain, TileId, SAUCER_RADIUS};
use caravan_reference::ReferenceContext;
use engine_api::{
    snapshot, Frame, GameSession, GameSurface, LogicalTime, RenderBatch, RenderSnapshot,
    RenderSource, SurfaceManifest, SurfaceSnapshotRequest, Tau,
};
use engine_observation_mcp::serve_game_surface_stdio_blocking;
use serde_json::{json, Value};

use crate::engine_integration::{actual_worldline, state, CaravanWorldline};
use crate::render::{project_output, render_batch_with_view, CaravanView};

/// The Caravan package's GameSurface definition for generic agent tooling.
#[derive(Clone, Copy, Debug, Default)]
pub struct CaravanSurface;

pub type CaravanGameSession = GameSession<CaravanSurface>;

pub fn new_game_session() -> CaravanGameSession {
    let mut writer = engine_api::JournalWriter::new();
    writer.record(GameJournalEntry::create_saucer());
    let worldline = actual_worldline(writer.finish());
    GameSession::new(CaravanSurface, worldline).expect("Caravan worldline is actual")
}

impl GameSurface for CaravanSurface {
    type Context = ReferenceContext;
    type Fact = GameJournalEntry;
    type View = CaravanView;
    type Error = String;

    fn manifest(&self) -> SurfaceManifest {
        SurfaceManifest {
            name: "caravan-sample".to_owned(),
            capabilities: vec![
                "explicit-observation".to_owned(),
                "view-control".to_owned(),
                "journal-read".to_owned(),
                "counterfactual-branches".to_owned(),
                "preview-append".to_owned(),
                "commit-append".to_owned(),
                "discard-branch".to_owned(),
            ],
            fact_schema: json!({
                "type": "object",
                "description": "One Caravan journal fact.",
                "oneOf": [
                    { "properties": { "kind": { "const": "CreateSaucer" }, "radius": { "const": SAUCER_RADIUS } }, "required": ["kind", "radius"] },
                    { "properties": { "kind": { "const": "SpawnActor" }, "id": { "type": "integer", "minimum": 1 }, "actor_kind": { "enum": ["Farmer", "Forester", "Arsonist", "Fighter", "Arborist"] }, "tile": { "type": "object" } }, "required": ["kind", "id", "actor_kind", "tile"] },
                    { "properties": { "kind": { "const": "SetTerrain" }, "tile": { "type": "object" }, "terrain": { "enum": ["Void", "Wheat", "Forest"] } }, "required": ["kind", "tile", "terrain"] }
                ]
            }),
            view_schema: json!({
                "type": "object",
                "properties": {
                    "operation": { "enum": ["zoom", "pan", "reset"] },
                    "zoom_delta": { "type": "number" },
                    "x_delta": { "type": "number" },
                    "y_delta": { "type": "number" }
                },
                "required": ["operation"]
            }),
        }
    }

    fn default_view(&self) -> Self::View {
        CaravanView::default()
    }

    fn view(&self, view: &Self::View) -> Result<Value, Self::Error> {
        Ok(encode_view(*view))
    }

    fn update_view(&self, view: &mut Self::View, update: Value) -> Result<Value, Self::Error> {
        let object = update
            .as_object()
            .ok_or_else(|| "view update must be an object".to_owned())?;
        let operation = object
            .get("operation")
            .and_then(Value::as_str)
            .ok_or_else(|| "view update requires an operation".to_owned())?;
        match operation {
            "zoom" => view.zoom_by(number(object, "zoom_delta")?),
            "pan" => view.pan_by(number(object, "x_delta")?, number(object, "y_delta")?),
            "reset" => view.reset(),
            _ => return Err(format!("unsupported view operation: {operation}")),
        }
        Ok(encode_view(*view))
    }

    fn encode_fact(&self, fact: &Self::Fact) -> Value {
        encode_fact(fact)
    }

    fn decode_fact(&self, value: Value) -> Result<Self::Fact, Self::Error> {
        decode_fact(value)
    }

    fn observe(
        &self,
        worldline: &CaravanWorldline,
        view: &Self::View,
        request: SurfaceSnapshotRequest,
    ) -> Result<RenderSnapshot, Self::Error> {
        let source = CaravanBranchSource {
            worldline,
            view: *view,
        };
        snapshot(
            &source,
            request.logical_time,
            request.tau,
            request.observation,
        )
        .map_err(|error| error.to_string())
    }
}

struct CaravanBranchSource<'a> {
    worldline: &'a CaravanWorldline,
    view: CaravanView,
}

impl RenderSource for CaravanBranchSource<'_> {
    type Error = Infallible;

    fn render(
        &self,
        logical_time: LogicalTime,
        tau: Tau,
    ) -> Result<Frame<RenderBatch>, Self::Error> {
        let sampled = state(self.worldline, logical_time);
        let output = project_output(&sampled);
        Ok(Frame::new(tau, render_batch_with_view(&output, self.view)))
    }
}

fn decode_fact(value: Value) -> Result<GameJournalEntry, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "fact must be a JSON object".to_owned())?;
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "fact.kind must be a string".to_owned())?;
    match kind {
        "CreateSaucer" => {
            let radius = object
                .get("radius")
                .and_then(Value::as_u64)
                .and_then(|value| u8::try_from(value).ok())
                .ok_or_else(|| "fact.radius must be an integer".to_owned())?;
            if radius != SAUCER_RADIUS {
                return Err(format!(
                    "fact.radius must equal the Caravan saucer radius {SAUCER_RADIUS}"
                ));
            }
            Ok(GameJournalEntry::CreateSaucer { radius })
        }
        "SpawnActor" => Ok(GameJournalEntry::SpawnActor {
            id: decode_actor_id(object.get("id"))?,
            kind: decode_actor_kind(object.get("actor_kind"))?,
            tile: decode_tile(object.get("tile"))?,
        }),
        "SetTerrain" => Ok(GameJournalEntry::SetTerrain {
            tile: decode_tile(object.get("tile"))?,
            terrain: decode_terrain(object.get("terrain"))?,
        }),
        _ => Err(format!("unsupported Caravan fact kind: {kind}")),
    }
}

fn decode_actor_id(value: Option<&Value>) -> Result<ActorId, String> {
    value
        .and_then(Value::as_u64)
        .and_then(ActorId::new)
        .ok_or_else(|| "fact.id must be a positive integer".to_owned())
}

fn decode_actor_kind(value: Option<&Value>) -> Result<ActorKind, String> {
    match value.and_then(Value::as_str) {
        Some("Farmer") => Ok(ActorKind::Farmer),
        Some("Forester") => Ok(ActorKind::Forester),
        Some("Arsonist") => Ok(ActorKind::Arsonist),
        Some("Fighter") => Ok(ActorKind::Fighter),
        Some("Arborist") => Ok(ActorKind::Arborist),
        _ => Err("fact.actor_kind must name a supported ActorKind".to_owned()),
    }
}

fn decode_terrain(value: Option<&Value>) -> Result<Terrain, String> {
    match value.and_then(Value::as_str) {
        Some("Void") => Ok(Terrain::Void),
        Some("Wheat") => Ok(Terrain::Wheat),
        Some("Forest") => Ok(Terrain::Forest),
        _ => Err("fact.terrain must be Void, Wheat, or Forest".to_owned()),
    }
}

fn decode_tile(value: Option<&Value>) -> Result<TileId, String> {
    let object = value
        .and_then(Value::as_object)
        .ok_or_else(|| "fact.tile must be an object".to_owned())?;
    let coordinate = |name: &str| {
        object
            .get(name)
            .and_then(Value::as_i64)
            .and_then(|value| i32::try_from(value).ok())
            .ok_or_else(|| format!("fact.tile.{name} must be an i32"))
    };
    TileId::new(coordinate("q")?, coordinate("r")?)
        .ok_or_else(|| "fact.tile must be inside the Caravan saucer".to_owned())
}

fn encode_fact(fact: &GameJournalEntry) -> Value {
    match fact {
        GameJournalEntry::CreateSaucer { radius } => {
            json!({ "kind": "CreateSaucer", "radius": radius })
        }
        GameJournalEntry::SpawnActor { id, kind, tile } => json!({
            "kind": "SpawnActor",
            "id": id.get(),
            "actor_kind": format!("{kind:?}"),
            "tile": encode_tile(*tile),
        }),
        GameJournalEntry::SetTerrain { tile, terrain } => json!({
            "kind": "SetTerrain",
            "tile": encode_tile(*tile),
            "terrain": format!("{terrain:?}"),
        }),
    }
}

fn encode_tile(tile: TileId) -> Value {
    json!({ "q": tile.q(), "r": tile.r() })
}

fn encode_view(view: CaravanView) -> Value {
    let offset = view.offset();
    json!({
        "view": {
            "zoom": view.zoom(),
            "offset": { "x": offset[0], "y": offset[1] }
        }
    })
}

fn number(object: &serde_json::Map<String, Value>, name: &str) -> Result<f32, String> {
    let value = object
        .get(name)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("view update requires numeric {name}"))?;
    if !value.is_finite() {
        return Err(format!("view update requires finite {name}"));
    }
    let value = value as f32;
    if !value.is_finite() {
        return Err(format!("view update requires finite {name}"));
    }
    Ok(value)
}

/// Runs the Caravan GameSurface over the generic stdio MCP transport.
pub fn serve_stdio() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    serve_game_surface_stdio_blocking(new_game_session())
}

#[cfg(test)]
mod tests {
    use super::new_game_session;
    use engine_api::{AppendRequest, LogicalTime, ObservationRequest, SurfaceSnapshotRequest, Tau};
    use serde_json::json;

    #[test]
    fn surface_exposes_caravan_schema_and_observes_initial_saucer() {
        let surface = new_game_session();
        let manifest = surface.manifest();
        assert_eq!(manifest.name, "caravan-sample");
        assert!(manifest.fact_schema.to_string().contains("SetTerrain"));
        let snapshot = surface
            .observe(SurfaceSnapshotRequest {
                branch_id: 0,
                logical_time: LogicalTime::zero(),
                tau: Tau::zero(),
                observation: ObservationRequest::new(32, 32),
            })
            .expect("initial Caravan surface should render");
        assert!(snapshot.vertex_count() > 0);
        assert!(snapshot.triangle_count() > 0);
    }

    #[test]
    fn surface_authoring_and_view_updates_are_independent() {
        let mut surface = new_game_session();
        let before = surface.view().expect("view should be readable");
        let updated = surface
            .update_view(json!({
                "operation": "zoom",
                "zoom_delta": 0.25
            }))
            .expect("view should update");
        assert_ne!(before, updated);

        let result = surface
            .commit_append(AppendRequest {
                branch_id: 0,
                expected_revision: 0,
                logical_time: LogicalTime::from_ticks(1),
                facts: vec![json!({
                    "kind": "SetTerrain",
                    "tile": { "q": 0, "r": 0 },
                    "terrain": "Wheat"
                })],
            })
            .expect("valid Caravan fact should append");
        assert_eq!(result.new_revision, 1);
        assert_eq!(
            surface
                .journal(0)
                .expect("actual journal exists")
                .descriptor
                .revision,
            1
        );
    }

    #[test]
    fn invalid_caravan_fact_does_not_mutate_history() {
        let mut surface = new_game_session();
        let error = surface
            .commit_append(AppendRequest {
                branch_id: 0,
                expected_revision: 0,
                logical_time: LogicalTime::from_ticks(1),
                facts: vec![json!({
                    "kind": "SetTerrain",
                    "tile": { "q": 99, "r": 99 },
                    "terrain": "Wheat"
                })],
            })
            .expect_err("out-of-range tile should be rejected");
        assert!(error.to_string().contains("inside the Caravan saucer"));
        assert_eq!(
            surface
                .journal(0)
                .expect("actual journal exists")
                .facts
                .len(),
            1
        );
    }
}
