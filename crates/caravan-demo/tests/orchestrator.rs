use caravan_demo::input::{Button, InputPacket};
use caravan_demo::{CaravanInteraction, CaravanOrchestrator, CaravanStage};
use caravan_domain::{GameJournalEntry, Terrain, TileId};
use caravan_reference::{actual, state, Snapshot};
use engine_journal::JournalWriter;
use engine_presentation::{LinearPlayback, Renderer};
use engine_sdk::GameState;
use engine_time::{LogicalTime, Tau};

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProbeFrame {
    logical_time: LogicalTime,
    tau: Tau,
    origin_terrain: Option<Terrain>,
}

struct ProbeRenderer;

impl Renderer<Snapshot> for ProbeRenderer {
    type Output = ProbeFrame;

    fn render(&self, state: &GameState<Snapshot>, tau: Tau) -> Self::Output {
        ProbeFrame {
            logical_time: state.logical_time(),
            tau,
            origin_terrain: state.payload().terrain_at(TileId::origin()),
        }
    }
}

fn time(game_ticks: i64) -> LogicalTime {
    LogicalTime::from_game_ticks(game_ticks).expect("test time is representable")
}

fn worldline() -> caravan_reference::ReferenceWorldline {
    let mut writer = JournalWriter::new();
    writer.record(GameJournalEntry::create_saucer());
    actual(writer.finish())
}

fn stage() -> CaravanStage<CaravanInteraction, ProbeRenderer> {
    let orchestrator = CaravanOrchestrator::new(
        worldline(),
        LinearPlayback::one_to_one(),
        Tau::zero(),
        CaravanInteraction,
    )
    .expect("test orchestrator should initialize");
    CaravanStage::new(orchestrator, ProbeRenderer)
}

#[test]
fn public_stage_publishes_a_transformation_as_a_new_worldline() {
    let mut stage = stage();
    let parent = stage.orchestrator().worldline().clone();
    stage.receive_packet(InputPacket::ButtonPressed(Button::Primary));

    assert!(stage
        .interact_and_apply()
        .expect("publication should succeed"));
    assert_eq!(parent.journal().len(), 1);
    assert_eq!(stage.orchestrator().worldline().journal().len(), 2);
    assert_eq!(
        state(&parent, time(0))
            .payload()
            .terrain_at(TileId::origin()),
        Some(Terrain::Void)
    );
    assert_eq!(
        stage
            .orchestrator()
            .sample()
            .payload()
            .terrain_at(TileId::origin()),
        Some(Terrain::Wheat)
    );
}

#[test]
fn explicit_past_and_future_samples_are_repeatable_without_cursor_mutation() {
    let stage = stage();
    let tau = Tau::from_ticks(time(4).ticks());

    let first = stage.present_at(tau);
    let second = stage.present_at(tau);

    assert_eq!(first, second);
    assert_eq!(first.payload().logical_time, time(4));
    assert_eq!(first.tau(), tau);
    assert_eq!(stage.orchestrator().tau(), Tau::zero());
}

#[test]
fn orchestrator_trace_is_a_stable_external_artifact() {
    let mut stage = stage();
    stage.receive_packet(InputPacket::ButtonPressed(Button::Primary));
    let applied = stage
        .interact_and_apply()
        .expect("publication should succeed");
    let rendered = stage.present().payload().clone();
    let trace = format!(
        "applied={applied} journal_entries={} logical_time={} tau={} origin_terrain={:?}",
        stage.orchestrator().worldline().journal().len(),
        rendered.logical_time.ticks(),
        rendered.tau.ticks(),
        rendered.origin_terrain,
    );

    assert_eq!(
        trace,
        include_str!("../snapshots/orchestrator-trace.txt").trim()
    );
}
