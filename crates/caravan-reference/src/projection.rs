use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

use caravan_domain::{
    Actor, ActorId, ActorKind, Effect, GameJournalEntry, Resources, Saucer, Terrain, TileId,
    TileLayers,
};
use caravan_vegetation::IndexedTile;
use engine_sdk::{Context, GameState};
use engine_time::{game_tick_index, LogicalTime, TICKS_PER_LOGICAL_SECOND};

use crate::discontinuities::DiscontinuityIndex;
use crate::{Journal, JournalEntry, ReferenceContext, ReferenceWorldline, Snapshot, State};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ProjectionError {
    UnsupportedSaucerRadius {
        append_ordinal: usize,
        expected: u8,
        found: u8,
    },
    InsufficientTrajectoryHorizon {
        requested_tick: i64,
        indexed_through: i64,
    },
}

impl core::fmt::Display for ProjectionError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedSaucerRadius {
                append_ordinal,
                expected,
                found,
            } => write!(
                formatter,
                "journal entry {append_ordinal} declares saucer radius {found}; anchor radius is {expected}"
            ),
            Self::InsufficientTrajectoryHorizon {
                requested_tick,
                indexed_through,
            } => write!(
                formatter,
                "trajectory is indexed through tick {indexed_through}, but query requests tick {requested_tick}"
            ),
        }
    }
}

impl std::error::Error for ProjectionError {}

pub fn project(worldline: &ReferenceWorldline, logical_time: LogicalTime) -> State {
    try_project(worldline, logical_time).expect("reference projection rejected the journal")
}

pub fn try_project(
    worldline: &ReferenceWorldline,
    logical_time: LogicalTime,
) -> Result<State, ProjectionError> {
    try_project_query(worldline.context(), worldline.journal(), logical_time)
}

pub fn project_query(
    context: &Context<ReferenceContext>,
    journal: &Journal,
    logical_time: LogicalTime,
) -> State {
    try_project_query(context, journal, logical_time)
        .expect("reference projection rejected the journal")
}

pub fn try_project_query(
    context: &Context<ReferenceContext>,
    journal: &Journal,
    logical_time: LogicalTime,
) -> Result<State, ProjectionError> {
    let index = DiscontinuityIndex::for_query(journal, logical_time);
    try_project_with_index(context, &index, logical_time)
}

pub fn project_with_index(
    context: &Context<ReferenceContext>,
    index: &DiscontinuityIndex,
    logical_time: LogicalTime,
) -> State {
    try_project_with_index(context, index, logical_time)
        .expect("reference projection rejected the journal")
}

pub fn try_project_with_index(
    context: &Context<ReferenceContext>,
    index: &DiscontinuityIndex,
    logical_time: LogicalTime,
) -> Result<State, ProjectionError> {
    let piece = index.select(logical_time);
    let piece_input = piece.payload();
    let entries = piece_input.visible_entries();
    let facts = JournalFacts::from_entries(*context.payload(), entries)?;
    let target_tick = game_tick_index(logical_time);
    if piece_input.is_tick_indexed() && !piece_input.actor_trajectory().can_sample(target_tick) {
        return Err(ProjectionError::InsufficientTrajectoryHorizon {
            requested_tick: target_tick,
            indexed_through: piece_input.actor_trajectory().indexed_through(),
        });
    }
    let snapshot = if piece_input.is_tick_indexed() {
        project_tick(&facts, logical_time, piece_input.actor_trajectory())
    } else {
        project_journal(&facts, logical_time)
    };

    Ok(GameState::new(logical_time, snapshot))
}

#[derive(Clone, Copy, Debug)]
struct ActorFact {
    actor: Actor,
    spawn_time: LogicalTime,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ActorLayerState {
    positions: BTreeMap<ActorId, TileId>,
    live: BTreeSet<ActorId>,
    removed: BTreeSet<ActorId>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ActorSegment {
    start_tick: i64,
    end_tick: i64,
    states: Arc<[ActorLayerState]>,
    cycle_period: Option<usize>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ActorTrajectory {
    segments: Arc<[ActorSegment]>,
    indexed_through: i64,
    has_actors: bool,
}

impl ActorTrajectory {
    fn empty() -> Self {
        Self {
            segments: Arc::from(Vec::<ActorSegment>::new()),
            indexed_through: -1,
            has_actors: false,
        }
    }

    fn indexed_through(&self) -> i64 {
        self.indexed_through
    }

    fn can_sample(&self, tick_index: i64) -> bool {
        !self.has_actors || tick_index <= self.indexed_through
    }

    fn sample(&self, tick_index: i64) -> Option<&ActorLayerState> {
        let segment_index = self
            .segments
            .partition_point(|segment| segment.end_tick < tick_index);
        let segment = self.segments.get(segment_index)?;
        if tick_index < segment.start_tick || tick_index > segment.end_tick {
            return None;
        }

        let offset = usize::try_from(tick_index - segment.start_tick).ok()?;
        let state_index = segment
            .cycle_period
            .map_or(offset, |period| offset % period);
        segment.states.get(state_index)
    }
}

#[derive(Clone, Copy, Debug)]
struct TerrainFact {
    logical_time: LogicalTime,
    tile: TileId,
    terrain: Terrain,
}

#[derive(Clone, Debug, Default)]
struct JournalFacts {
    saucer: Option<Saucer>,
    terrain: BTreeMap<TileId, Terrain>,
    terrain_facts: Vec<TerrainFact>,
    actors: BTreeMap<ActorId, ActorFact>,
}

pub(crate) fn build_actor_trajectory(
    context: ReferenceContext,
    entries: &[JournalEntry],
    target_tick: i64,
) -> ActorTrajectory {
    if target_tick < 0 {
        return ActorTrajectory::empty();
    }

    let Ok(facts) = JournalFacts::from_entries(context, entries) else {
        return ActorTrajectory::empty();
    };
    let actors = facts.actors.values().copied().collect::<Vec<_>>();
    let farmer_actions = farmer_actions(&facts, &actors, target_tick);
    let conversions = arborist_conversions(&actors, target_tick);
    let vegetation_events = terrain_events(
        &facts,
        target_tick,
        &farmer_actions,
        &conversions,
        &BTreeMap::new(),
        None,
    );
    let fire_ignitions = fire_ignitions(&actors, target_tick, &vegetation_events);
    let terrain_events = terrain_events(
        &facts,
        target_tick,
        &farmer_actions,
        &conversions,
        &fire_ignitions,
        None,
    );
    let facts_by_id = actor_facts_by_id(&actors);
    let mut layer = ActorLayerState {
        positions: initial_positions(&facts_by_id),
        live: initial_live_actors(&facts_by_id),
        removed: BTreeSet::new(),
    };
    let mut segments = Vec::new();
    let mut start_tick = 0;

    for event_tick in actor_event_ticks(&actors, &farmer_actions, &terrain_events, target_tick) {
        layer = append_actor_interval(
            &mut segments,
            start_tick,
            event_tick - 1,
            layer,
            &facts_by_id,
            &terrain_events,
            &farmer_actions,
        );
        transition_actor_layer_at_tick(
            &facts_by_id,
            &mut layer.positions,
            &mut layer.live,
            &mut layer.removed,
            &terrain_events,
            &farmer_actions,
            event_tick,
        );
        start_tick = event_tick;
    }

    append_actor_interval(
        &mut segments,
        start_tick,
        target_tick,
        layer,
        &facts_by_id,
        &terrain_events,
        &farmer_actions,
    );

    ActorTrajectory {
        segments: Arc::from(segments),
        indexed_through: target_tick,
        has_actors: !facts.actors.is_empty(),
    }
}

fn actor_event_ticks(
    actors: &[ActorFact],
    farmer_actions: &[FarmerAction],
    terrain_events: &BTreeMap<TileId, Vec<TerrainEvent>>,
    target_tick: i64,
) -> Vec<i64> {
    let mut event_ticks = BTreeSet::new();

    for fact in actors {
        if let Some(visible_tick) = entry_activation_tick(fact.spawn_time) {
            if visible_tick > 0 && visible_tick <= target_tick {
                event_ticks.insert(visible_tick);
            }
        }
        let active_tick = first_active_tick(fact.spawn_time);
        if active_tick > 0 && active_tick <= target_tick {
            event_ticks.insert(active_tick);
        }
    }

    for action in farmer_actions {
        if action.activation_tick > 0 && action.activation_tick <= target_tick {
            event_ticks.insert(action.activation_tick);
        }
    }

    for events in terrain_events.values() {
        for event in events {
            if event.tick > 0 && event.tick <= target_tick {
                event_ticks.insert(event.tick);
            }
        }
    }

    event_ticks.into_iter().collect()
}

fn append_actor_interval(
    segments: &mut Vec<ActorSegment>,
    start_tick: i64,
    end_tick: i64,
    mut layer: ActorLayerState,
    facts_by_id: &BTreeMap<ActorId, ActorFact>,
    terrain_events: &BTreeMap<TileId, Vec<TerrainEvent>>,
    farmer_actions: &[FarmerAction],
) -> ActorLayerState {
    if start_tick > end_tick {
        return layer;
    }

    let mut states = Vec::new();
    let mut seen = BTreeMap::new();
    let mut tick = start_tick;

    loop {
        if let Some(&cycle_start_index) = seen.get(&layer) {
            let cycle_start_tick = start_tick
                + i64::try_from(cycle_start_index).expect("trajectory index is representable");

            if cycle_start_index > 0 {
                segments.push(ActorSegment {
                    start_tick,
                    end_tick: cycle_start_tick - 1,
                    states: Arc::from(states[..cycle_start_index].to_vec()),
                    cycle_period: None,
                });
            }

            let cycle_states = states[cycle_start_index..].to_vec();
            let cycle_period = cycle_states.len();
            let final_offset = usize::try_from(end_tick - cycle_start_tick)
                .expect("trajectory cycle offset is representable")
                % cycle_period;
            segments.push(ActorSegment {
                start_tick: cycle_start_tick,
                end_tick,
                states: Arc::from(cycle_states.clone()),
                cycle_period: Some(cycle_period),
            });
            return cycle_states[final_offset].clone();
        }

        seen.insert(layer.clone(), states.len());
        states.push(layer.clone());

        if tick == end_tick {
            break;
        }

        let transition_tick = tick + 1;
        transition_actor_layer_at_tick(
            facts_by_id,
            &mut layer.positions,
            &mut layer.live,
            &mut layer.removed,
            terrain_events,
            farmer_actions,
            transition_tick,
        );
        tick = transition_tick;
    }

    segments.push(ActorSegment {
        start_tick,
        end_tick,
        states: Arc::from(states),
        cycle_period: None,
    });
    layer
}

fn transition_actor_layer_at_tick(
    facts_by_id: &BTreeMap<ActorId, ActorFact>,
    positions: &mut BTreeMap<ActorId, TileId>,
    live: &mut BTreeSet<ActorId>,
    removed: &mut BTreeSet<ActorId>,
    terrain_events: &BTreeMap<TileId, Vec<TerrainEvent>>,
    farmer_actions: &[FarmerAction],
    transition_tick: i64,
) {
    let boundary = LogicalTime::from_game_ticks(transition_tick)
        .expect("selected actor transition time is representable");
    for (actor_id, fact) in facts_by_id {
        if fact.spawn_time <= boundary && !removed.contains(actor_id) {
            live.insert(*actor_id);
        }
    }
    transition_actor_layer(
        facts_by_id,
        positions,
        live,
        removed,
        terrain_events,
        farmer_actions,
        transition_tick,
    );
}

impl JournalFacts {
    fn from_entries(
        context: ReferenceContext,
        entries: &[JournalEntry],
    ) -> Result<Self, ProjectionError> {
        let mut facts = Self::default();
        let saucer = Saucer::new();

        for (append_ordinal, entry) in entries.iter().enumerate() {
            match *entry.payload() {
                GameJournalEntry::CreateSaucer { radius } if radius != context.saucer_radius() => {
                    return Err(ProjectionError::UnsupportedSaucerRadius {
                        append_ordinal,
                        expected: context.saucer_radius(),
                        found: radius,
                    });
                }
                GameJournalEntry::CreateSaucer { radius }
                    if radius == context.saucer_radius() && facts.saucer.is_none() =>
                {
                    facts.saucer = Some(saucer);
                }
                GameJournalEntry::CreateSaucer { .. } => {}
                GameJournalEntry::SpawnActor { id, kind, tile } if facts.saucer.is_some() => {
                    let spawn_time = facts
                        .actors
                        .get(&id)
                        .map(|fact| fact.spawn_time)
                        .unwrap_or(entry.logical_time());
                    facts.actors.insert(
                        id,
                        ActorFact {
                            actor: Actor::new(id, kind, tile),
                            spawn_time,
                        },
                    );
                }
                GameJournalEntry::SpawnActor { .. } => {}
                GameJournalEntry::SetTerrain { tile, terrain }
                    if facts.saucer.is_some() && saucer.tiles().contains(&tile) =>
                {
                    facts.terrain.insert(tile, terrain);
                    facts.terrain_facts.push(TerrainFact {
                        logical_time: entry.logical_time(),
                        tile,
                        terrain,
                    });
                }
                GameJournalEntry::SetTerrain { .. } => {}
            }
        }

        Ok(facts)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct FarmerAction {
    actor_id: ActorId,
    activation_tick: i64,
    destination: TileId,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct TerrainEvent {
    tick: i64,
    order: usize,
    terrain: Terrain,
}

fn project_journal(facts: &JournalFacts, logical_time: LogicalTime) -> Snapshot {
    let tick_index = game_tick_index(logical_time);
    let terrain = journal_terrain(facts);
    let actors = facts
        .actors
        .values()
        .map(|fact| fact.actor)
        .collect::<Vec<_>>();
    snapshot(
        facts.saucer,
        tick_index,
        &terrain,
        &BTreeMap::new(),
        actors,
        Resources::default(),
    )
}

fn project_tick(
    facts: &JournalFacts,
    logical_time: LogicalTime,
    actor_trajectory: &ActorTrajectory,
) -> Snapshot {
    let target_tick = game_tick_index(logical_time);
    if target_tick < 0 {
        return project_journal(facts, logical_time);
    }

    let actor_facts = facts.actors.values().copied().collect::<Vec<_>>();
    let farmer_actions = farmer_actions(facts, &actor_facts, target_tick);
    let conversions = arborist_conversions(&actor_facts, target_tick);
    let vegetation_events = terrain_events(
        facts,
        target_tick,
        &farmer_actions,
        &conversions,
        &BTreeMap::new(),
        None,
    );
    let fire_ignitions = fire_ignitions(&actor_facts, target_tick, &vegetation_events);
    let terrain_events = terrain_events(
        facts,
        target_tick,
        &farmer_actions,
        &conversions,
        &fire_ignitions,
        Some(target_tick),
    );
    let terrain = terrain_at_target(facts, &terrain_events, target_tick);
    let effects = fire_effects(&fire_ignitions, target_tick);
    let actors = actors_at_sample(&actor_facts, actor_trajectory, target_tick, logical_time);
    let resources = resources_at_tick(
        facts,
        &terrain_events,
        &actor_facts,
        actor_trajectory,
        target_tick,
        logical_time,
    );

    snapshot(
        facts.saucer,
        target_tick,
        &terrain,
        &effects,
        actors,
        resources,
    )
}

fn journal_terrain(facts: &JournalFacts) -> BTreeMap<TileId, Terrain> {
    let Some(saucer) = facts.saucer else {
        return BTreeMap::new();
    };

    saucer
        .tiles()
        .iter()
        .copied()
        .map(|tile| (tile, facts.terrain.get(&tile).copied().unwrap_or_default()))
        .collect()
}

fn authored_terrain_at_tick(facts: &JournalFacts, target_tick: i64) -> BTreeMap<TileId, Terrain> {
    let mut terrain = facts
        .saucer
        .map(|saucer| {
            saucer
                .tiles()
                .iter()
                .copied()
                .map(|tile| (tile, Terrain::Void))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    for fact in &facts.terrain_facts {
        if entry_activation_tick(fact.logical_time).is_some_and(|tick| tick <= target_tick) {
            terrain.insert(fact.tile, fact.terrain);
        }
    }

    terrain
}

fn farmer_actions(
    facts: &JournalFacts,
    actors: &[ActorFact],
    target_tick: i64,
) -> Vec<FarmerAction> {
    actors
        .iter()
        .filter(|fact| fact.actor.kind() == ActorKind::Farmer)
        .filter_map(|fact| {
            let activation_tick = first_active_tick(fact.spawn_time);
            if activation_tick > target_tick {
                return None;
            }

            let activation_time = LogicalTime::from_game_ticks(activation_tick)?;
            let terrain = authored_terrain_at_tick(facts, activation_tick);
            let active_actors = actors
                .iter()
                .copied()
                .filter(|other| other.spawn_time <= activation_time)
                .collect::<Vec<_>>();

            let actor_id = fact.actor.id();
            let destination = fact
                .actor
                .tile()
                .neighbors()
                .into_iter()
                .flatten()
                .find(|tile| open_void(*tile, actor_id, &terrain, &active_actors))
                .unwrap_or(fact.actor.tile());

            Some(FarmerAction {
                actor_id,
                activation_tick,
                destination,
            })
        })
        .collect()
}

fn arborist_conversions(actors: &[ActorFact], target_tick: i64) -> BTreeMap<TileId, i64> {
    actors
        .iter()
        .filter(|fact| fact.actor.kind() == ActorKind::Arborist)
        .filter_map(|fact| {
            let conversion_tick = first_active_tick(fact.spawn_time).saturating_add(2);
            (conversion_tick <= target_tick).then_some((fact.actor.tile(), conversion_tick))
        })
        .collect()
}

fn fire_ignitions(
    actors: &[ActorFact],
    target_tick: i64,
    terrain_events: &BTreeMap<TileId, Vec<TerrainEvent>>,
) -> BTreeMap<TileId, i64> {
    let mut ignition_times = BTreeMap::new();
    let mut destruction_times = BTreeMap::<TileId, i64>::new();
    let mut pending = VecDeque::new();

    for fact in actors
        .iter()
        .filter(|fact| fact.actor.kind() == ActorKind::Arsonist)
    {
        let activation_tick = first_active_tick(fact.spawn_time);
        let Some(activation_time) = LogicalTime::from_game_ticks(activation_tick) else {
            continue;
        };
        let has_target = actors.iter().any(|other| {
            other.actor.id() != fact.actor.id() && other.spawn_time <= activation_time
        });
        if activation_tick > target_tick || !has_target {
            continue;
        }

        for tile in fact.actor.tile().neighbors().into_iter().flatten() {
            let terrain = terrain_at_tick(
                terrain_events.get(&tile).map(Vec::as_slice).unwrap_or(&[]),
                activation_tick,
            );
            if is_burnable(terrain) {
                enqueue_ignition(&mut ignition_times, &mut pending, tile, activation_tick);
            }
        }
    }

    while let Some((source, ignition_tick)) = pending.pop_front() {
        let Some(next_tick) = ignition_tick.checked_add(3) else {
            continue;
        };
        if next_tick > target_tick {
            continue;
        }

        destruction_times
            .entry(source)
            .and_modify(|current| *current = (*current).min(next_tick))
            .or_insert(next_tick);

        for tile in source.neighbors().into_iter().flatten() {
            let terrain = terrain_at_tick(
                terrain_events.get(&tile).map(Vec::as_slice).unwrap_or(&[]),
                next_tick,
            );
            let already_destroyed = destruction_times
                .get(&tile)
                .is_some_and(|destruction_tick| *destruction_tick <= next_tick);
            if is_burnable(terrain) && !already_destroyed {
                enqueue_ignition(&mut ignition_times, &mut pending, tile, next_tick);
            }
        }
    }

    ignition_times
}

fn enqueue_ignition(
    ignition_times: &mut BTreeMap<TileId, i64>,
    pending: &mut VecDeque<(TileId, i64)>,
    tile: TileId,
    ignition_tick: i64,
) {
    let is_earlier = ignition_times
        .get(&tile)
        .is_none_or(|current| ignition_tick < *current);
    if is_earlier {
        ignition_times.insert(tile, ignition_tick);
        pending.push_back((tile, ignition_tick));
    }
}

fn terrain_events(
    facts: &JournalFacts,
    target_tick: i64,
    farmer_actions: &[FarmerAction],
    conversions: &BTreeMap<TileId, i64>,
    fire_ignitions: &BTreeMap<TileId, i64>,
    journal_query_tick: Option<i64>,
) -> BTreeMap<TileId, Vec<TerrainEvent>> {
    let mut events = facts
        .saucer
        .map(|saucer| {
            saucer
                .tiles()
                .iter()
                .copied()
                .map(|tile| (tile, Vec::new()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    for (authored_order, fact) in facts.terrain_facts.iter().enumerate() {
        let Some(tick) = journal_activation_tick(fact.logical_time, journal_query_tick) else {
            continue;
        };
        if let Some(tile_events) = events.get_mut(&fact.tile) {
            tile_events.push(TerrainEvent {
                tick,
                order: authored_order,
                terrain: fact.terrain,
            });
        }
    }

    let mut order = facts.terrain_facts.len();
    for action in farmer_actions {
        for tile in action.destination.neighbors().into_iter().flatten() {
            if let Some(tile_events) = events.get_mut(&tile) {
                tile_events.push(TerrainEvent {
                    tick: action.activation_tick,
                    order,
                    terrain: Terrain::Wheat,
                });
                order += 1;
            }
        }
    }

    for (tile, conversion_tick) in conversions {
        if let Some(tile_events) = events.get_mut(tile) {
            tile_events.push(TerrainEvent {
                tick: *conversion_tick,
                order,
                terrain: Terrain::Forest,
            });
            order += 1;
        }
    }

    for (tile, ignition_tick) in fire_ignitions {
        if let Some(destruction_tick) = ignition_tick.checked_add(3) {
            if destruction_tick <= target_tick {
                if let Some(tile_events) = events.get_mut(tile) {
                    tile_events.push(TerrainEvent {
                        tick: destruction_tick,
                        order,
                        terrain: Terrain::Void,
                    });
                    order += 1;
                }
            }
        }
    }

    for tile_events in events.values_mut() {
        tile_events.sort_by_key(|event| (event.tick, event.order));
    }
    events
}

fn terrain_at_target(
    facts: &JournalFacts,
    events: &BTreeMap<TileId, Vec<TerrainEvent>>,
    target_tick: i64,
) -> BTreeMap<TileId, Terrain> {
    let Some(saucer) = facts.saucer else {
        return BTreeMap::new();
    };

    saucer
        .tiles()
        .iter()
        .copied()
        .map(|tile| {
            (
                tile,
                terrain_at_tick(
                    events.get(&tile).map(Vec::as_slice).unwrap_or(&[]),
                    target_tick,
                ),
            )
        })
        .collect()
}

fn fire_effects(
    fire_ignitions: &BTreeMap<TileId, i64>,
    target_tick: i64,
) -> BTreeMap<TileId, Effect> {
    fire_ignitions
        .iter()
        .filter_map(|(tile, ignition_tick)| {
            let age = target_tick.checked_sub(*ignition_tick)?;
            (0..=2).contains(&age).then_some((
                *tile,
                Effect::fire(u32::try_from(age).expect("fire age is nonnegative")),
            ))
        })
        .collect()
}

fn actor_facts_by_id(actors: &[ActorFact]) -> BTreeMap<ActorId, ActorFact> {
    actors.iter().map(|fact| (fact.actor.id(), *fact)).collect()
}

fn initial_positions(facts_by_id: &BTreeMap<ActorId, ActorFact>) -> BTreeMap<ActorId, TileId> {
    facts_by_id
        .iter()
        .map(|(actor_id, fact)| (*actor_id, fact.actor.tile()))
        .collect()
}

fn initial_live_actors(facts_by_id: &BTreeMap<ActorId, ActorFact>) -> BTreeSet<ActorId> {
    facts_by_id
        .iter()
        .filter(|(_, fact)| fact.spawn_time <= LogicalTime::zero())
        .map(|(actor_id, _)| *actor_id)
        .collect()
}

fn actors_from_layer(
    facts_by_id: &BTreeMap<ActorId, ActorFact>,
    positions: &BTreeMap<ActorId, TileId>,
    live: &BTreeSet<ActorId>,
) -> Vec<Actor> {
    facts_by_id
        .iter()
        .filter(|(actor_id, _)| live.contains(actor_id))
        .map(|(actor_id, fact)| Actor::new(*actor_id, fact.actor.kind(), positions[actor_id]))
        .collect()
}

fn actors_at_sample(
    actors: &[ActorFact],
    trajectory: &ActorTrajectory,
    target_tick: i64,
    logical_time: LogicalTime,
) -> Vec<Actor> {
    let boundary = LogicalTime::from_game_ticks(target_tick).unwrap_or(logical_time);
    let mut sampled = trajectory
        .sample(target_tick)
        .map(|layer| actors_from_layer(&actor_facts_by_id(actors), &layer.positions, &layer.live))
        .unwrap_or_default();
    let sampled_ids = sampled
        .iter()
        .map(|actor| actor.id())
        .collect::<BTreeSet<_>>();

    sampled.extend(
        actors
            .iter()
            .filter(|fact| {
                fact.spawn_time > boundary
                    && fact.spawn_time <= logical_time
                    && !sampled_ids.contains(&fact.actor.id())
            })
            .map(|fact| fact.actor),
    );
    sampled
}

fn transition_actor_layer(
    facts_by_id: &BTreeMap<ActorId, ActorFact>,
    positions: &mut BTreeMap<ActorId, TileId>,
    live: &mut BTreeSet<ActorId>,
    removed: &mut BTreeSet<ActorId>,
    terrain_events: &BTreeMap<TileId, Vec<TerrainEvent>>,
    farmer_actions: &[FarmerAction],
    transition_tick: i64,
) {
    let pre_live = live.clone();
    let mut removed_this_tick = BTreeSet::new();

    for actor_id in &pre_live {
        let fact = facts_by_id[actor_id];
        if fact.actor.kind() == ActorKind::Farmer
            && farmer_actions.iter().any(|action| {
                action.actor_id == *actor_id && action.activation_tick == transition_tick
            })
        {
            removed_this_tick.insert(*actor_id);
        }
        if fact.actor.kind() == ActorKind::Arsonist
            && first_active_tick(fact.spawn_time) <= transition_tick
            && pre_live.iter().any(|other_id| other_id != actor_id)
        {
            removed_this_tick.insert(*actor_id);
        }
    }

    let occupied = pre_live
        .iter()
        .map(|actor_id| positions[actor_id])
        .collect::<BTreeSet<_>>();
    let mut desired = BTreeMap::new();
    let mut fighter_targets = BTreeMap::new();

    for actor_id in &pre_live {
        if removed_this_tick.contains(actor_id) {
            continue;
        }
        let fact = facts_by_id[actor_id];
        let current = positions[actor_id];
        let destination = match fact.actor.kind() {
            ActorKind::Forester if first_active_tick(fact.spawn_time) <= transition_tick => {
                forester_destination(current, terrain_events, transition_tick, &occupied)
            }
            ActorKind::Fighter if first_active_tick(fact.spawn_time) <= transition_tick => {
                let target = pre_live
                    .iter()
                    .filter(|target_id| facts_by_id[target_id].actor.kind() == ActorKind::Arsonist)
                    .min();
                if let Some(target_id) = target {
                    fighter_targets.insert(*actor_id, *target_id);
                    advance_towards(current, positions[target_id], 1)
                } else {
                    current
                }
            }
            _ => current,
        };
        desired.insert(*actor_id, destination);
    }

    let mut winners = BTreeMap::<TileId, ActorId>::new();
    for (actor_id, tile) in &desired {
        winners
            .entry(*tile)
            .and_modify(|winner| *winner = (*winner).min(*actor_id))
            .or_insert(*actor_id);
    }

    let mut final_positions = BTreeMap::new();
    for (actor_id, candidate) in desired {
        let tile = if winners.get(&candidate) == Some(&actor_id) {
            candidate
        } else {
            positions[&actor_id]
        };
        final_positions.insert(actor_id, tile);
    }

    for (fighter_id, target_id) in fighter_targets {
        if final_positions.get(&fighter_id).copied() == positions.get(&target_id).copied() {
            removed_this_tick.insert(target_id);
        }
    }

    for actor_id in removed_this_tick {
        live.remove(&actor_id);
        removed.insert(actor_id);
    }
    for (actor_id, tile) in final_positions {
        if live.contains(&actor_id) {
            positions.insert(actor_id, tile);
        }
    }
}

fn forester_destination(
    current: TileId,
    terrain_events: &BTreeMap<TileId, Vec<TerrainEvent>>,
    transition_tick: i64,
    occupied: &BTreeSet<TileId>,
) -> TileId {
    let current_events = terrain_events
        .get(&current)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if terrain_at_tick(current_events, transition_tick) == Terrain::Forest {
        return current;
    }

    current
        .neighbors()
        .into_iter()
        .flatten()
        .find(|tile| !occupied.contains(tile))
        .unwrap_or(current)
}

fn advance_towards(mut current: TileId, target: TileId, steps: i64) -> TileId {
    let mut remaining = steps;
    while remaining > 0 && current != target {
        let current_distance = current.axial().distance_to(target.axial());
        let Some(next) = current
            .neighbors()
            .into_iter()
            .flatten()
            .find(|tile| tile.axial().distance_to(target.axial()) < current_distance)
        else {
            break;
        };
        current = next;
        remaining -= 1;
    }
    current
}

fn resources_at_tick(
    facts: &JournalFacts,
    terrain_events: &BTreeMap<TileId, Vec<TerrainEvent>>,
    actors: &[ActorFact],
    actor_trajectory: &ActorTrajectory,
    target_tick: i64,
    logical_time: LogicalTime,
) -> Resources {
    let Some(saucer) = facts.saucer else {
        return Resources::default();
    };

    let wheat = saucer.tiles().iter().copied().fold(0_u64, |total, tile| {
        let produced = count_terrain_ticks(
            terrain_events.get(&tile).map(Vec::as_slice).unwrap_or(&[]),
            Terrain::Wheat,
            0,
            target_tick,
        );
        total
            .checked_add(produced)
            .expect("indexed wheat total overflowed u64")
    });
    let wood = wood_at_ticks(
        actors,
        terrain_events,
        actor_trajectory,
        target_tick,
        logical_time,
    );

    Resources::new(wheat, wood)
}

fn wood_at_ticks(
    actors: &[ActorFact],
    terrain_events: &BTreeMap<TileId, Vec<TerrainEvent>>,
    actor_trajectory: &ActorTrajectory,
    target_tick: i64,
    logical_time: LogicalTime,
) -> u64 {
    if target_tick < 0 {
        return 0;
    }

    let facts_by_id = actor_facts_by_id(actors);
    let prior_ticks = target_tick.saturating_sub(1);
    let prior_total = trajectory_wood_total(
        actor_trajectory,
        &facts_by_id,
        terrain_events,
        0,
        prior_ticks,
    );
    let target_actors = actors_at_sample(actors, actor_trajectory, target_tick, logical_time);
    let target_total = target_actors
        .iter()
        .filter(|actor| actor.kind() == ActorKind::Forester)
        .filter(|actor| {
            terrain_events
                .get(&actor.tile())
                .map(Vec::as_slice)
                .map(|events| terrain_at_tick(events, target_tick) == Terrain::Forest)
                .unwrap_or(false)
        })
        .count() as u64;

    prior_total
        .checked_add(target_total)
        .expect("indexed wood total overflowed u64")
}

fn trajectory_wood_total(
    trajectory: &ActorTrajectory,
    facts_by_id: &BTreeMap<ActorId, ActorFact>,
    terrain_events: &BTreeMap<TileId, Vec<TerrainEvent>>,
    start_tick: i64,
    end_tick: i64,
) -> u64 {
    if start_tick > end_tick {
        return 0;
    }

    let mut total = 0_u64;
    for segment in trajectory.segments.iter() {
        let overlap_start = start_tick.max(segment.start_tick);
        let overlap_end = end_tick.min(segment.end_tick);
        if overlap_start > overlap_end {
            continue;
        }

        let events_at_start = |tile: TileId| {
            terrain_events
                .get(&tile)
                .map(Vec::as_slice)
                .map(|events| terrain_at_tick(events, segment.start_tick))
                .unwrap_or_default()
        };
        if let Some(period) = segment.cycle_period {
            let counts = segment
                .states
                .iter()
                .map(|layer| {
                    layer
                        .live
                        .iter()
                        .filter(|actor_id| {
                            facts_by_id[actor_id].actor.kind() == ActorKind::Forester
                        })
                        .filter(|actor_id| {
                            events_at_start(layer.positions[actor_id]) == Terrain::Forest
                        })
                        .count() as u64
                })
                .collect::<Vec<_>>();
            let cycle_total = counts.iter().sum::<u64>();
            let length = u64::try_from(overlap_end - overlap_start + 1)
                .expect("trajectory interval length is representable");
            let full_cycles = length / u64::try_from(period).expect("cycle period is positive");
            total = total
                .checked_add(
                    cycle_total
                        .checked_mul(full_cycles)
                        .expect("indexed wood total overflowed u64"),
                )
                .expect("indexed wood total overflowed u64");

            let remainder = usize::try_from(length % u64::try_from(period).unwrap())
                .expect("trajectory remainder is representable");
            let offset = usize::try_from(overlap_start - segment.start_tick)
                .expect("trajectory offset is representable")
                % period;
            for step in 0..remainder {
                total = total
                    .checked_add(counts[(offset + step) % period])
                    .expect("indexed wood total overflowed u64");
            }
        } else {
            for tick in overlap_start..=overlap_end {
                let state_index = usize::try_from(tick - segment.start_tick)
                    .expect("trajectory offset is representable");
                let layer = &segment.states[state_index];
                let produced = layer
                    .live
                    .iter()
                    .filter(|actor_id| facts_by_id[actor_id].actor.kind() == ActorKind::Forester)
                    .filter(|actor_id| {
                        events_at_start(layer.positions[actor_id]) == Terrain::Forest
                    })
                    .count() as u64;
                total = total
                    .checked_add(produced)
                    .expect("indexed wood total overflowed u64");
            }
        }
    }

    total
}

fn count_terrain_ticks(
    events: &[TerrainEvent],
    wanted: Terrain,
    start_tick: i64,
    end_tick: i64,
) -> u64 {
    if start_tick > end_tick {
        return 0;
    }

    let mut current = terrain_at_tick(events, start_tick - 1);
    let mut segment_start = start_tick;
    let mut total = 0_u64;
    let mut event_index = 0;

    while event_index < events.len() {
        let event = events[event_index];
        if event.tick < start_tick {
            event_index += 1;
            continue;
        }
        if event.tick > end_tick {
            break;
        }

        if event.tick > segment_start && current == wanted {
            total = total
                .checked_add(tick_span(segment_start, event.tick - 1))
                .expect("indexed terrain total overflowed u64");
        }

        let event_tick = event.tick;
        while event_index < events.len() && events[event_index].tick == event_tick {
            current = events[event_index].terrain;
            event_index += 1;
        }
        segment_start = event_tick;
    }

    if segment_start <= end_tick && current == wanted {
        total = total
            .checked_add(tick_span(segment_start, end_tick))
            .expect("indexed terrain total overflowed u64");
    }
    total
}

fn terrain_at_tick(events: &[TerrainEvent], tick: i64) -> Terrain {
    events
        .iter()
        .take_while(|event| event.tick <= tick)
        .last()
        .map(|event| event.terrain)
        .unwrap_or_default()
}

fn snapshot(
    saucer: Option<Saucer>,
    tick_index: i64,
    terrain: &BTreeMap<TileId, Terrain>,
    effects: &BTreeMap<TileId, Effect>,
    actors: Vec<Actor>,
    resources: Resources,
) -> Snapshot {
    let actor_by_tile =
        actors
            .iter()
            .fold(BTreeMap::<TileId, ActorId>::new(), |mut by_tile, actor| {
                by_tile
                    .entry(actor.tile())
                    .and_modify(|current| *current = (*current).min(actor.id()))
                    .or_insert(actor.id());
                by_tile
            });
    let tiles = saucer
        .map(|saucer| {
            saucer
                .tiles()
                .iter()
                .copied()
                .map(|tile| {
                    let layers = TileLayers::new(
                        terrain.get(&tile).copied().unwrap_or_default(),
                        actor_by_tile.get(&tile).copied(),
                        effects.get(&tile).copied().unwrap_or_default(),
                    );
                    IndexedTile::new(tile, layers)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Snapshot::from_parts(saucer, tick_index, tiles, actors, resources)
}

fn open_void(
    tile: TileId,
    ignored_actor: ActorId,
    terrain: &BTreeMap<TileId, Terrain>,
    actors: &[ActorFact],
) -> bool {
    terrain.get(&tile).copied().unwrap_or_default() == Terrain::Void
        && actors
            .iter()
            .all(|fact| fact.actor.id() == ignored_actor || fact.actor.tile() != tile)
}

fn journal_activation_tick(logical_time: LogicalTime, query_tick: Option<i64>) -> Option<i64> {
    let entry_tick = game_tick_index(logical_time);
    if query_tick == Some(entry_tick) {
        return Some(entry_tick);
    }
    entry_activation_tick(logical_time)
}

fn entry_activation_tick(logical_time: LogicalTime) -> Option<i64> {
    let entry_tick = game_tick_index(logical_time);
    let remainder = logical_time.ticks().rem_euclid(TICKS_PER_LOGICAL_SECOND);
    entry_tick.checked_add(i64::from(remainder != 0))
}

fn first_active_tick(spawn_time: LogicalTime) -> i64 {
    game_tick_index(spawn_time).saturating_add(1).max(1)
}

fn tick_span(start_tick: i64, end_tick: i64) -> u64 {
    u64::try_from(end_tick.saturating_sub(start_tick))
        .expect("indexed terrain span must be nonnegative")
        .saturating_add(1)
}

fn is_burnable(terrain: Terrain) -> bool {
    matches!(terrain, Terrain::Wheat | Terrain::Forest)
}
