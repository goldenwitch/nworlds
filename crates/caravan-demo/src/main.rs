#![forbid(unsafe_code)]

use caravan_domain::{ActorId, ActorKind, Effect, GameJournalEntry, Terrain, TileId};
use caravan_reference::{actual, state, ReferenceWorldline, Snapshot};
use caravan_seeded::generate_spawn_journal;
use engine_journal::{Journal, JournalWriter};
use engine_lookahead::{branch_view, future, ViewKind};
use engine_presentation::{present, LinearPlayback, Renderer};
use engine_sdk::{Frame, GameState};
use engine_time::{LogicalTime, Tau};

fn main() {
    let empty = actual(Journal::empty());
    let empty_state = state(&empty, time(0));
    println!("caravan anchor demo");
    println!(
        "empty journal: saucer={} tiles={} void={} actors={} effects={} wheat={} wood={}",
        empty_state.payload().has_saucer(),
        empty_state.payload().tiles().len(),
        terrain_count(empty_state.payload(), Terrain::Void),
        empty_state.payload().actors().len(),
        effect_count(empty_state.payload()),
        empty_state.payload().resources().wheat(),
        empty_state.payload().resources().wood(),
    );

    let mut writer = JournalWriter::new();
    writer
        .advance_to(time(0))
        .expect("the demo starts at logical time zero");
    let create_entry = writer.record(GameJournalEntry::create_saucer());
    writer
        .advance_to(time(10))
        .expect("postdated demo entries move the writer forward");
    let spawn_entry = writer.record(spawn(1, ActorKind::Farmer, TileId::origin()));
    let authored_journal = writer.finish();
    let authored = actual(authored_journal);
    let created = state(&authored, time(0));
    println!(
        "create saucer: journal_t_={} radius=5 tiles={} void={} actors={} effects={}",
        create_entry.logical_time().ticks(),
        created.payload().tiles().len(),
        terrain_count(created.payload(), Terrain::Void),
        created.payload().actors().len(),
        effect_count(created.payload()),
    );
    println!(
        "journal-owned timestamps: CreateSaucer@t_={} SpawnActor(id=1)@t_={}",
        create_entry.logical_time().ticks(),
        spawn_entry.logical_time().ticks(),
    );
    let before_spawn = state(&authored, time(9));
    let at_spawn = state(&authored, time(10));
    println!(
        "postdated spawn: t_=9 actors={:?}; t_=10 actors={:?}",
        actor_ids(before_spawn.payload()),
        actor_ids(at_spawn.payload()),
    );

    let sample_later = state(&authored, time(10));
    let sample_earlier = state(&authored, time(2));
    let sample_later_again = state(&authored, time(10));
    println!(
        "arbitrary sampling: t_=[10,2,10] tick_index=[{},{},{}] game_tick_period=1s same_data_at_2_and_10={} repeated_t10={}",
        sample_later.payload().tick_index(),
        sample_earlier.payload().tick_index(),
        sample_later_again.payload().tick_index(),
        same_automaton_data(sample_earlier.payload(), sample_later.payload()),
        sample_later == sample_later_again,
    );

    let farmer = actual(journal([
        (0, GameJournalEntry::create_saucer()),
        (0, spawn(1, ActorKind::Farmer, TileId::origin())),
    ]));
    let farmer_at_zero = state(&farmer, time(0));
    let farmer_at_one = state(&farmer, time(1));
    println!(
        "farmer result: t_=0 actors={:?} -> t_=1 actors={:?} wheat_tiles={} wheat_resource={}",
        actor_ids(farmer_at_zero.payload()),
        actor_ids(farmer_at_one.payload()),
        terrain_count(farmer_at_one.payload(), Terrain::Wheat),
        farmer_at_one.payload().resources().wheat(),
    );

    let fire_worldline = actual(journal([
        (0, GameJournalEntry::create_saucer()),
        (
            0,
            GameJournalEntry::SetTerrain {
                tile: tile(1, 0),
                terrain: Terrain::Wheat,
            },
        ),
        (0, spawn(2, ActorKind::Arsonist, TileId::origin())),
        (0, spawn(3, ActorKind::Arborist, tile(1, 0))),
    ]));
    let layers_at_fire_start = state(&fire_worldline, time(1));
    let layers_after_fire = state(&fire_worldline, time(4));
    println!(
        "three layers: t_=1 tile=(1, 0) {}",
        layer_summary(layers_at_fire_start.payload(), tile(1, 0)),
    );
    println!(
        "fire aging: t_=4 tile=(1, 0) {}",
        layer_summary(layers_after_fire.payload(), tile(1, 0)),
    );

    let first_seeded = generate_spawn_journal(0xCAFE, 20);
    let second_seeded = generate_spawn_journal(0xCAFE, 20);
    let spawn_times = first_seeded
        .iter()
        .filter_map(|entry| match entry.payload() {
            GameJournalEntry::SpawnActor { .. } => Some(entry.logical_time().ticks()),
            GameJournalEntry::CreateSaucer { .. } | GameJournalEntry::SetTerrain { .. } => None,
        })
        .collect::<Vec<_>>();
    println!(
        "seeded journal: seed=0xCAFE horizon=20 entries={} same_entries={} spawn_times={:?}",
        first_seeded.len(),
        first_seeded == second_seeded,
        spawn_times,
    );

    let seeded_worldline = actual(first_seeded);
    let lookahead = future(&seeded_worldline, time(30));
    println!(
        "lookahead: fixed_entries={} t_=30 tick_index={} tiles={} actors={}",
        seeded_worldline.journal().len(),
        lookahead.payload().tick_index(),
        lookahead.payload().tiles().len(),
        lookahead.payload().actors().len(),
    );

    let alternate = journal([(7, spawn(2, ActorKind::Forester, TileId::origin()))]);
    let replacement = journal([(6, spawn(3, ActorKind::Arborist, tile(0, 1)))]);
    let counterfactual = authored
        .counterfactual(time(5), &alternate)
        .expect("the counterfactual suffix is after its fork boundary");
    let corrected = authored
        .corrected_suffix(time(5), &replacement)
        .expect("the corrected suffix is after its fork boundary");
    let actual_view = branch_view(&authored);
    let counterfactual_view = branch_view(&counterfactual);
    let corrected_view = branch_view(&corrected);
    println!(
        "branch views: actual={} fork={} counterfactual={} fork={} corrected={} fork={}",
        branch_kind_name(actual_view.kind()),
        fork_label(actual_view.fork_boundary()),
        branch_kind_name(counterfactual_view.kind()),
        fork_label(counterfactual_view.fork_boundary()),
        branch_kind_name(corrected_view.kind()),
        fork_label(corrected_view.fork_boundary()),
    );

    let renderer = TraceRenderer;
    let playback = LinearPlayback::one_to_one();
    print_frame(
        "actual",
        present(
            &authored,
            &reference_query,
            &playback,
            &renderer,
            Tau::from_ticks(10),
        ),
    );
    print_frame(
        "counterfactual",
        present(
            &counterfactual,
            &reference_query,
            &playback,
            &renderer,
            Tau::from_ticks(10),
        ),
    );
    print_frame(
        "corrected",
        present(
            &corrected,
            &reference_query,
            &playback,
            &renderer,
            Tau::from_ticks(10),
        ),
    );
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct RenderValue {
    sampled_time: i64,
    tau: i64,
    actor_ids: Vec<u64>,
    wheat: u64,
    wood: u64,
}

struct TraceRenderer;

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

fn print_frame(label: &str, frame: Frame<RenderValue>) {
    let rendered = frame.payload();
    println!(
        "presentation {}: sdk_tau={} game_t_={} actor_ids={:?} wheat={} wood={}",
        label,
        rendered.tau,
        rendered.sampled_time,
        rendered.actor_ids,
        rendered.wheat,
        rendered.wood,
    );
}

fn reference_query(
    worldline: &ReferenceWorldline,
    logical_time: LogicalTime,
) -> GameState<Snapshot> {
    state(worldline, logical_time)
}

fn journal(entries: impl IntoIterator<Item = (i64, GameJournalEntry)>) -> Journal {
    let mut writer = JournalWriter::new();
    for (ticks, payload) in entries {
        writer
            .advance_to(time(ticks))
            .expect("demo journal timestamps are monotonic");
        writer.record(payload);
    }
    writer.finish()
}

fn spawn(id: u64, kind: ActorKind, tile: TileId) -> GameJournalEntry {
    GameJournalEntry::SpawnActor {
        id: ActorId::new(id).expect("demo actor IDs are positive"),
        kind,
        tile,
    }
}

fn time(ticks: i64) -> LogicalTime {
    LogicalTime::from_ticks(ticks)
}

fn tile(q: i32, r: i32) -> TileId {
    TileId::new(q, r).expect("demo coordinates are inside the saucer")
}

fn actor_ids(snapshot: &Snapshot) -> Vec<u64> {
    snapshot
        .actors()
        .iter()
        .map(|actor| actor.id().get())
        .collect()
}

fn terrain_count(snapshot: &Snapshot, terrain: Terrain) -> usize {
    snapshot
        .tiles()
        .iter()
        .filter(|tile| tile.layers().terrain() == terrain)
        .count()
}

fn effect_count(snapshot: &Snapshot) -> usize {
    snapshot
        .tiles()
        .iter()
        .filter(|tile| tile.layers().effect() != Effect::None)
        .count()
}

fn same_automaton_data(left: &Snapshot, right: &Snapshot) -> bool {
    left.saucer() == right.saucer()
        && left.tiles() == right.tiles()
        && left.actors() == right.actors()
        && left.resources() == right.resources()
}

fn layer_summary(snapshot: &Snapshot, tile: TileId) -> String {
    let layers = snapshot
        .layers_at(tile)
        .expect("layer summary coordinates are inside the saucer");
    let actor = layers
        .actor()
        .map(|id| id.get().to_string())
        .unwrap_or_else(|| "None".to_owned());
    let effect = match layers.effect() {
        Effect::None => "None".to_owned(),
        Effect::Fire { age_in_game_ticks } => format!("Fire(age={age_in_game_ticks})"),
    };
    format!(
        "terrain={} actor={} effect={}",
        terrain_name(layers.terrain()),
        actor,
        effect,
    )
}

fn terrain_name(terrain: Terrain) -> &'static str {
    match terrain {
        Terrain::Void => "Void",
        Terrain::Wheat => "Wheat",
        Terrain::Forest => "Forest",
    }
}

fn branch_kind_name(kind: ViewKind) -> &'static str {
    match kind {
        ViewKind::Actual => "Actual",
        ViewKind::Counterfactual => "Counterfactual",
        ViewKind::Corrected => "Corrected",
    }
}

fn fork_label(boundary: Option<LogicalTime>) -> String {
    boundary
        .map(|time| time.ticks().to_string())
        .unwrap_or_else(|| "-".to_owned())
}
