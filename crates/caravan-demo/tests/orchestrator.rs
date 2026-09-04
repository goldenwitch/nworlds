use caravan_demo::engine_integration::{
    actual_worldline as actual, BranchKind, CaravanJournalWriter as JournalWriter, GameState,
    LogicalTime, Renderer, Tau,
};
use caravan_demo::input::{Button, InputPacket};
use caravan_demo::{CaravanInteraction, CaravanOrchestrator, CaravanStage, OrchestratorError};
use caravan_domain::{GameJournalEntry, Terrain, TileId};
use caravan_reference::{state, Snapshot};

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProbeFrame {
    logical_time: LogicalTime,
    tau: Tau,
    origin_terrain: Option<Terrain>,
}

struct ProbeRenderer;

impl Renderer<Snapshot> for ProbeRenderer {
    type Output = ProbeFrame;

    fn render(state: &GameState<Snapshot>, tau: Tau) -> Self::Output {
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

fn malformed_stage() -> CaravanStage<CaravanInteraction, ProbeRenderer> {
    let mut writer = JournalWriter::new();
    writer.record(GameJournalEntry::CreateSaucer { radius: 4 });
    let orchestrator = CaravanOrchestrator::new(
        actual(writer.finish()),
        LogicalTime::zero(),
        Tau::zero(),
        CaravanInteraction,
    )
    .expect("authoring cursor should accept malformed payloads");
    CaravanStage::new(orchestrator)
}

fn stage() -> CaravanStage<CaravanInteraction, ProbeRenderer> {
    let orchestrator = CaravanOrchestrator::new(
        worldline(),
        LogicalTime::zero(),
        Tau::zero(),
        CaravanInteraction,
    )
    .expect("test orchestrator should initialize");
    CaravanStage::new(orchestrator)
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
            .expect("published sample should be valid")
            .payload()
            .terrain_at(TileId::origin()),
        Some(Terrain::Wheat)
    );
}

#[test]
fn explicit_past_and_future_samples_are_repeatable_without_cursor_mutation() {
    let stage = stage();
    let tau = Tau::from_ticks(time(4).ticks());

    let first = stage
        .present_at(time(4), tau)
        .expect("first explicit sample should be valid");
    let second = stage
        .present_at(time(4), tau)
        .expect("second explicit sample should be valid");

    assert_eq!(first, second);
    assert_eq!(first.payload().logical_time, time(4));
    assert_eq!(first.tau(), tau);
    assert_eq!(stage.orchestrator().tau(), Tau::zero());
}

#[test]
fn non_monotonic_samples_are_equal_without_query_history() {
    let stage = stage();
    let later_tau = Tau::from_ticks(time(5).ticks());
    let earlier_tau = Tau::from_ticks(time(2).ticks());

    let later_first = stage
        .present_at(time(5), later_tau)
        .expect("later sample should be valid");
    let _earlier = stage
        .present_at(time(2), earlier_tau)
        .expect("earlier sample should be valid");
    let later_again = stage
        .present_at(time(5), later_tau)
        .expect("repeated later sample should be valid");

    assert_eq!(later_first, later_again);
}

#[test]
fn render_output_carries_both_selected_state_time_and_tau() {
    let mut stage = stage();
    stage.receive_packet(InputPacket::ButtonPressed(Button::Primary));
    stage
        .interact_and_apply()
        .expect("publication should succeed");
    let first_tau = Tau::zero();
    let second_tau = Tau::from_ticks(time(3).ticks());

    let first = stage
        .present_at(time(0), first_tau)
        .expect("first render sample should be valid");
    let second = stage
        .present_at(time(3), second_tau)
        .expect("second render sample should be valid");

    assert_eq!(first.payload().origin_terrain, Some(Terrain::Wheat));
    assert_eq!(second.payload().origin_terrain, Some(Terrain::Wheat));
    assert_eq!(first.payload().logical_time, time(0));
    assert_eq!(second.payload().logical_time, time(3));
    assert_eq!(first.payload().tau, first_tau);
    assert_eq!(second.payload().tau, second_tau);
}

#[test]
fn public_stage_can_publish_a_counterfactual_child_without_mutating_parent() {
    let mut stage = stage();
    let parent = stage.orchestrator().worldline().clone();

    assert!(stage
        .apply_counterfactual(
            time(0),
            time(1),
            caravan_demo::transformation::Transformation::SetTerrain {
                tile: TileId::origin(),
                terrain: Terrain::Forest,
            },
        )
        .expect("counterfactual publication should succeed"));
    assert_eq!(
        stage.orchestrator().worldline().kind(),
        BranchKind::Counterfactual
    );
    assert!(parent.is_actual());
    assert_eq!(parent.journal().len(), 1);
}

#[test]
fn malformed_stage_presentation_returns_projection_error() {
    let stage = malformed_stage();

    assert!(matches!(
        stage.present(),
        Err(OrchestratorError::Projection(
            caravan_reference::ProjectionError::UnsupportedSaucerRadius { found: 4, .. }
        ))
    ));
}

#[test]
fn selected_worldline_round_trips_through_orchestrator_save_choice() {
    let stage = stage();
    let bytes = stage
        .orchestrator()
        .save_selected()
        .expect("selected worldline should save");
    let restored = caravan_persistence::decode(&bytes).expect("saved worldline should decode");

    assert_eq!(&restored, stage.orchestrator().worldline());
}

#[test]
fn orchestrator_trace_is_a_stable_external_artifact() {
    let mut stage = stage();
    stage.receive_packet(InputPacket::ButtonPressed(Button::Primary));
    let applied = stage
        .interact_and_apply()
        .expect("publication should succeed");
    let rendered = stage
        .present()
        .expect("stage sample should be valid")
        .payload()
        .clone();
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
