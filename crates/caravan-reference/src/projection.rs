use std::collections::{BTreeMap, BTreeSet, VecDeque};

use caravan_domain::{
    Actor, ActorId, ActorKind, Effect, GameJournalEntry, Resources, Saucer, Terrain, TileId,
    TileLayers,
};
use caravan_vegetation::IndexedTile;
use engine_journal::{Journal, JournalEntry};
use engine_sdk::{Context, GameState};
use engine_time::{game_tick_index, LogicalTime, TICKS_PER_LOGICAL_SECOND};

use crate::discontinuities::{DiscontinuityIndex, PieceInput};
use crate::{ReferenceContext, ReferenceWorldline, Snapshot, State};

pub fn project(worldline: &ReferenceWorldline, logical_time: LogicalTime) -> State {
    project_query(worldline.context(), worldline.journal(), logical_time)
}

pub fn project_query(
    context: &Context<ReferenceContext>,
    journal: &Journal,
    logical_time: LogicalTime,
) -> State {
    let index = DiscontinuityIndex::for_sample(journal, logical_time);
    project_with_index(context, &index, logical_time)
}

pub fn project_with_index(
    context: &Context<ReferenceContext>,
    index: &DiscontinuityIndex,
    logical_time: LogicalTime,
) -> State {
    let piece = index.select(logical_time);
    let piece_input: PieceInput = *piece.payload();
    let entries = index.entries_for(piece);
    let facts = JournalFacts::from_entries(*context.payload(), entries);
    let snapshot = if piece_input.is_tick_indexed() {
        project_tick(&facts, logical_time)
    } else {
        project_journal(&facts, logical_time)
    };

    GameState::new(logical_time, snapshot)
}

#[derive(Clone, Copy, Debug)]
struct ActorFact {
    actor: Actor,
    spawn_time: LogicalTime,
}

#[derive(Clone, Copy, Debug)]
struct TerrainFact {
    logical_time: LogicalTime,
    tile: TileId,
    terrain: Terrain,
    append_ordinal: usize,
}

#[derive(Clone, Debug, Default)]
struct JournalFacts {
    saucer: Option<Saucer>,
    terrain: BTreeMap<TileId, Terrain>,
    terrain_facts: Vec<TerrainFact>,
    actors: BTreeMap<ActorId, ActorFact>,
}

impl JournalFacts {
    fn from_entries(context: ReferenceContext, entries: &[JournalEntry]) -> Self {
        let mut facts = Self::default();

        for (append_ordinal, entry) in entries.iter().enumerate() {
            match *entry.payload() {
                GameJournalEntry::CreateSaucer { radius }
                    if radius == context.saucer().radius() && facts.saucer.is_none() =>
                {
                    facts.saucer = Some(context.saucer());
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
                    if facts.saucer.is_some() && context.saucer().tiles().contains(&tile) =>
                {
                    facts.terrain.insert(tile, terrain);
                    facts.terrain_facts.push(TerrainFact {
                        logical_time: entry.logical_time(),
                        tile,
                        terrain,
                        append_ordinal,
                    });
                }
                GameJournalEntry::SetTerrain { .. } => {}
            }
        }

        facts
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

fn project_tick(facts: &JournalFacts, logical_time: LogicalTime) -> Snapshot {
    let target_tick = game_tick_index(logical_time);
    if target_tick < 0 {
        return project_journal(facts, logical_time);
    }

    let actor_facts = facts.actors.values().copied().collect::<Vec<_>>();
    let journal_terrain = journal_terrain(facts);
    let farmer_actions = farmer_actions(facts, &actor_facts, target_tick);
    let conversions = arborist_conversions(&actor_facts, target_tick);
    let fire_ignitions = fire_ignitions(facts, &actor_facts, target_tick);
    let terrain_events = terrain_events(
        facts,
        target_tick,
        &farmer_actions,
        &conversions,
        &fire_ignitions,
    );
    let terrain = terrain_at_target(facts, &terrain_events, target_tick);
    let effects = fire_effects(&fire_ignitions, target_tick);
    let actors = actors_at_tick(&actor_facts, &journal_terrain, &farmer_actions, target_tick);
    let resources = resources_at_tick(
        facts,
        &terrain_events,
        &actor_facts,
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

fn farmer_actions(
    facts: &JournalFacts,
    actors: &[ActorFact],
    target_tick: i64,
) -> Vec<FarmerAction> {
    let terrain = journal_terrain(facts);
    actors
        .iter()
        .filter(|fact| fact.actor.kind() == ActorKind::Farmer)
        .filter_map(|fact| {
            let activation_tick = first_active_tick(fact.spawn_time);
            if activation_tick > target_tick {
                return None;
            }

            let actor_id = fact.actor.id();
            let destination = fact
                .actor
                .tile()
                .neighbors()
                .into_iter()
                .flatten()
                .find(|tile| open_void(*tile, actor_id, &terrain, actors))
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
    facts: &JournalFacts,
    actors: &[ActorFact],
    target_tick: i64,
) -> BTreeMap<TileId, i64> {
    let terrain = journal_terrain(facts);
    let mut ignition_times = BTreeMap::new();
    let mut pending = VecDeque::new();

    for fact in actors
        .iter()
        .filter(|fact| fact.actor.kind() == ActorKind::Arsonist)
    {
        let activation_tick = first_active_tick(fact.spawn_time);
        let has_target = actors
            .iter()
            .any(|other| other.actor.id() != fact.actor.id());
        if activation_tick > target_tick || !has_target {
            continue;
        }

        for tile in fact.actor.tile().neighbors().into_iter().flatten() {
            if is_burnable(terrain.get(&tile).copied().unwrap_or_default()) {
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

        for tile in source.neighbors().into_iter().flatten() {
            if is_burnable(terrain.get(&tile).copied().unwrap_or_default()) {
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

    for fact in &facts.terrain_facts {
        let Some(tick) = journal_activation_tick(fact.logical_time, target_tick) else {
            continue;
        };
        if let Some(tile_events) = events.get_mut(&fact.tile) {
            tile_events.push(TerrainEvent {
                tick,
                order: fact.append_ordinal,
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

fn actors_at_tick(
    actors: &[ActorFact],
    terrain: &BTreeMap<TileId, Terrain>,
    farmer_actions: &[FarmerAction],
    target_tick: i64,
) -> Vec<Actor> {
    let removed = actors
        .iter()
        .filter_map(|fact| match fact.actor.kind() {
            ActorKind::Farmer => farmer_actions
                .iter()
                .find(|action| action.actor_id == fact.actor.id())
                .filter(|action| action.activation_tick <= target_tick)
                .map(|_| fact.actor.id()),
            ActorKind::Arsonist => {
                let activation_tick = first_active_tick(fact.spawn_time);
                let has_target = actors
                    .iter()
                    .any(|other| other.actor.id() != fact.actor.id());
                (activation_tick <= target_tick && has_target).then_some(fact.actor.id())
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    let mut desired = BTreeMap::new();
    for fact in actors {
        if removed.contains(&fact.actor.id()) {
            continue;
        }

        let tile = match fact.actor.kind() {
            ActorKind::Forester => forester_position(fact, actors, terrain, target_tick),
            ActorKind::Fighter => fighter_position(fact, actors, target_tick),
            _ => fact.actor.tile(),
        };
        desired.insert(fact.actor.id(), tile);
    }

    let mut winners = BTreeMap::<TileId, ActorId>::new();
    for (actor_id, tile) in &desired {
        winners
            .entry(*tile)
            .and_modify(|winner| *winner = (*winner).min(*actor_id))
            .or_insert(*actor_id);
    }

    actors
        .iter()
        .filter_map(|fact| {
            if removed.contains(&fact.actor.id()) {
                return None;
            }
            let candidate = desired
                .get(&fact.actor.id())
                .copied()
                .unwrap_or(fact.actor.tile());
            let tile = if winners.get(&candidate) == Some(&fact.actor.id()) {
                candidate
            } else {
                fact.actor.tile()
            };
            Some(Actor::new(fact.actor.id(), fact.actor.kind(), tile))
        })
        .collect()
}

fn forester_position(
    fact: &ActorFact,
    actors: &[ActorFact],
    terrain: &BTreeMap<TileId, Terrain>,
    target_tick: i64,
) -> TileId {
    let activation_tick = first_active_tick(fact.spawn_time);
    if activation_tick > target_tick {
        return fact.actor.tile();
    }

    let occupied = actors
        .iter()
        .filter(|other| other.actor.id() != fact.actor.id())
        .map(|other| other.actor.tile())
        .collect::<BTreeSet<_>>();
    let turns = target_tick
        .saturating_sub(activation_tick)
        .saturating_add(1);

    forester_after_turns(fact.actor.tile(), turns, terrain, &occupied)
}

fn forester_after_turns(
    origin: TileId,
    turns: i64,
    terrain: &BTreeMap<TileId, Terrain>,
    occupied: &BTreeSet<TileId>,
) -> TileId {
    let mut positions = Vec::new();
    let mut seen = BTreeMap::<TileId, usize>::new();
    let mut position = origin;

    loop {
        if turns < positions.len() as i64 {
            return positions[turns as usize];
        }
        if let Some(cycle_start) = seen.get(&position).copied() {
            let cycle_length = positions.len() - cycle_start;
            let cycle_offset = (turns - cycle_start as i64).rem_euclid(cycle_length as i64);
            return positions[cycle_start + cycle_offset as usize];
        }

        seen.insert(position, positions.len());
        positions.push(position);

        if terrain.get(&position).copied().unwrap_or_default() == Terrain::Forest {
            return position;
        }
        let Some(destination) = position
            .neighbors()
            .into_iter()
            .flatten()
            .find(|tile| !occupied.contains(tile))
        else {
            return position;
        };
        position = destination;
    }
}

fn fighter_position(fact: &ActorFact, actors: &[ActorFact], target_tick: i64) -> TileId {
    let fighter_activation = first_active_tick(fact.spawn_time);
    if fighter_activation > target_tick {
        return fact.actor.tile();
    }

    let Some(target) = actors
        .iter()
        .filter(|other| other.actor.kind() == ActorKind::Arsonist)
        .min_by_key(|other| other.actor.id())
    else {
        return fact.actor.tile();
    };

    let target_activation = first_active_tick(target.spawn_time);
    let movement_start = fighter_activation.max(target_activation);
    let last_movement_tick = target_tick.min(target_activation);
    if movement_start > last_movement_tick {
        return fact.actor.tile();
    }

    let steps = last_movement_tick
        .saturating_sub(movement_start)
        .saturating_add(1);
    advance_towards(fact.actor.tile(), target.actor.tile(), steps)
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
    let wood = actors
        .iter()
        .filter(|fact| fact.actor.kind() == ActorKind::Forester)
        .fold(0_u64, |total, fact| {
            let produced = forester_wood(fact, actors, terrain_events, target_tick, logical_time);
            total
                .checked_add(produced)
                .expect("indexed wood total overflowed u64")
        });

    Resources::new(wheat, wood)
}

fn forester_wood(
    fact: &ActorFact,
    actors: &[ActorFact],
    terrain_events: &BTreeMap<TileId, Vec<TerrainEvent>>,
    target_tick: i64,
    logical_time: LogicalTime,
) -> u64 {
    let Some(visible_tick) = resource_tick(fact.spawn_time, logical_time, target_tick) else {
        return 0;
    };
    if visible_tick > target_tick {
        return 0;
    }

    let activation_tick = first_active_tick(fact.spawn_time);
    let origin = fact.actor.tile();
    let origin_events = terrain_events
        .get(&origin)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if activation_tick > target_tick {
        return count_terrain_ticks(origin_events, Terrain::Forest, visible_tick, target_tick);
    }

    if terrain_at_tick(origin_events, target_tick) == Terrain::Forest {
        return count_terrain_ticks(origin_events, Terrain::Forest, visible_tick, target_tick);
    }

    if terrain_at_tick(origin_events, activation_tick - 1) == Terrain::Forest {
        return count_terrain_ticks(origin_events, Terrain::Forest, visible_tick, target_tick);
    }

    let occupied = actors
        .iter()
        .filter(|other| other.actor.id() != fact.actor.id())
        .map(|other| other.actor.tile())
        .collect::<BTreeSet<_>>();
    let Some(destination) = origin
        .neighbors()
        .into_iter()
        .flatten()
        .find(|tile| !occupied.contains(tile))
    else {
        return 0;
    };
    let destination_events = terrain_events
        .get(&destination)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if terrain_at_tick(destination_events, activation_tick - 1) != Terrain::Forest {
        return 0;
    }

    count_terrain_ticks(
        destination_events,
        Terrain::Forest,
        activation_tick,
        target_tick,
    )
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

fn journal_activation_tick(logical_time: LogicalTime, target_tick: i64) -> Option<i64> {
    if logical_time.ticks() < 0 {
        return Some(0);
    }
    let entry_tick = game_tick_index(logical_time);
    if entry_tick == target_tick {
        return Some(target_tick);
    }
    let remainder = logical_time.ticks().rem_euclid(TICKS_PER_LOGICAL_SECOND);
    entry_tick.checked_add(i64::from(remainder != 0))
}

fn resource_tick(
    logical_time: LogicalTime,
    target_time: LogicalTime,
    target_tick: i64,
) -> Option<i64> {
    if logical_time > target_time {
        return None;
    }
    if logical_time.ticks() <= 0 {
        return Some(0);
    }
    journal_activation_tick(logical_time, target_tick)
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
