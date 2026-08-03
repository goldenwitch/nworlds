use caravan_domain::{ActorId, ActorKind, GameJournalEntry, Terrain, TileId};
use caravan_reference::{actual, state as reference_state, ReferenceWorldline, Snapshot, State};
use engine_journal::{Journal, JournalWriter};
use engine_presentation::{Animation, Renderer};
use engine_sdk::GameState;
use engine_time::{LogicalTime, Tau};

pub fn time(ticks: i64) -> LogicalTime {
    LogicalTime::from_ticks(ticks)
}

pub fn tile(q: i32, r: i32) -> TileId {
    TileId::new(q, r).expect("conformance coordinates are inside the saucer")
}

pub fn actor_id(value: u64) -> ActorId {
    ActorId::new(value).expect("conformance actor IDs are positive")
}

pub fn spawn(id: u64, kind: ActorKind, tile: TileId) -> GameJournalEntry {
    GameJournalEntry::SpawnActor {
        id: actor_id(id),
        kind,
        tile,
    }
}

pub fn terrain(tile: TileId, value: Terrain) -> GameJournalEntry {
    GameJournalEntry::SetTerrain {
        tile,
        terrain: value,
    }
}

pub fn journal(entries: impl IntoIterator<Item = (i64, GameJournalEntry)>) -> Journal {
    let mut writer = JournalWriter::new();
    for (ticks, payload) in entries {
        writer
            .advance_to(time(ticks))
            .expect("conformance journal timestamps are monotonic");
        writer.record(payload);
    }
    writer.finish()
}

pub fn worldline(entries: impl IntoIterator<Item = (i64, GameJournalEntry)>) -> ReferenceWorldline {
    actual(journal(entries))
}

pub fn snapshot_at(worldline: &ReferenceWorldline, ticks: i64) -> Snapshot {
    reference_state(worldline, time(ticks)).into_payload()
}

pub fn actor_ids(snapshot: &Snapshot) -> Vec<u64> {
    snapshot
        .actors()
        .iter()
        .map(|actor| actor.id().get())
        .collect()
}

pub fn terrain_count(snapshot: &Snapshot, terrain: Terrain) -> usize {
    snapshot
        .tiles()
        .iter()
        .filter(|tile| tile.layers().terrain() == terrain)
        .count()
}

pub fn reference_query(
    worldline: &ReferenceWorldline,
    logical_time: LogicalTime,
) -> GameState<Snapshot> {
    reference_state(worldline, logical_time)
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RenderValue {
    pub sampled_time: i64,
    pub tau: i64,
    pub actor_ids: Vec<u64>,
    pub wheat: u64,
    pub wood: u64,
}

pub struct TraceRenderer;

impl Renderer<Snapshot> for TraceRenderer {
    type Output = RenderValue;

    fn render(&self, state: &GameState<Snapshot>, tau: Tau) -> Self::Output {
        RenderValue {
            sampled_time: state.logical_time().ticks(),
            tau: tau.ticks(),
            actor_ids: actor_ids(state.payload()),
            wheat: state.payload().resources().wheat(),
            wood: state.payload().resources().wood(),
        }
    }
}

pub struct ParityAnimation;

impl Animation<Snapshot> for ParityAnimation {
    type Output = i64;

    fn sample(&self, state: &GameState<Snapshot>, tau: Tau) -> Option<Self::Output> {
        (tau.ticks().rem_euclid(2) == 0).then(|| state.logical_time().ticks() * 10 + tau.ticks())
    }
}

pub fn seeded_worldline() -> ReferenceWorldline {
    caravan_reference::actual(caravan_seeded::generate_spawn_journal(0xCAFE, 20))
}

pub fn state(worldline: &ReferenceWorldline, ticks: i64) -> State {
    caravan_reference::state(worldline, time(ticks))
}
