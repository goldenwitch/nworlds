use std::collections::{BTreeMap, BTreeSet};

use caravan_domain::{
    Actor, ActorId, ActorKind, Effect, GameJournalEntry, Resources, Saucer, Terrain, TileId,
    TileLayers,
};
use caravan_hazards::{
    ArboristDefinition, ArsonistDefinition, FighterDefinition, FireDefinition, FireOutcome,
    HazardCell,
};
use caravan_vegetation::{
    Farmer, FarmerResult, Forester, IndexedInput, IndexedTile, Snapshot as IndexedSnapshot, Wheat,
};
use engine_index::{IndexedQuery, QueryInput};
use engine_sdk::JournalEntry;
use engine_time::LogicalTime;

use crate::{ReferenceContext, Snapshot};

pub(crate) struct Oracle;

impl IndexedQuery<ReferenceContext, GameJournalEntry> for Oracle {
    type Result = Snapshot;

    fn query(&self, input: QueryInput<'_, ReferenceContext, GameJournalEntry>) -> Self::Result {
        evaluate(input)
    }
}

#[derive(Clone, Debug, Default)]
struct WorkingState {
    saucer: Option<Saucer>,
    tiles: BTreeMap<TileId, TileLayers>,
    actors: BTreeMap<ActorId, Actor>,
    spawn_ticks: BTreeMap<ActorId, i64>,
}

impl WorkingState {
    fn apply_entry(&mut self, entry: &JournalEntry<GameJournalEntry>, context: ReferenceContext) {
        match *entry.payload() {
            GameJournalEntry::CreateSaucer { radius }
                if radius == context.saucer().radius() && self.saucer.is_none() =>
            {
                let saucer = context.saucer();
                self.saucer = Some(saucer);
                self.tiles = saucer
                    .tiles()
                    .iter()
                    .copied()
                    .map(|tile| (tile, TileLayers::default()))
                    .collect();
            }
            GameJournalEntry::CreateSaucer { .. } => {}
            GameJournalEntry::SpawnActor { id, kind, tile } => {
                if self.tiles.contains_key(&tile) {
                    self.actors.insert(id, Actor::new(id, kind, tile));
                    self.spawn_ticks
                        .entry(id)
                        .or_insert(entry.logical_time().ticks());
                }
            }
            GameJournalEntry::SetTerrain { tile, terrain } => {
                if let Some(layers) = self.tiles.get_mut(&tile) {
                    *layers = TileLayers::new(terrain, layers.actor(), layers.effect());
                }
            }
        }
        self.sync_actor_layers();
    }

    fn sync_actor_layers(&mut self) {
        let mut actor_by_tile = BTreeMap::<TileId, ActorId>::new();
        for actor in self.actors.values() {
            actor_by_tile
                .entry(actor.tile())
                .and_modify(|current| *current = (*current).min(actor.id()))
                .or_insert(actor.id());
        }

        for (tile, layers) in &mut self.tiles {
            *layers = TileLayers::new(
                layers.terrain(),
                actor_by_tile.get(tile).copied(),
                layers.effect(),
            );
        }
    }

    fn set_terrain(&mut self, tile: TileId, terrain: Terrain) {
        if let Some(layers) = self.tiles.get_mut(&tile) {
            *layers = TileLayers::new(terrain, layers.actor(), layers.effect());
        }
    }

    fn set_effect(&mut self, tile: TileId, effect: Effect) {
        if let Some(layers) = self.tiles.get_mut(&tile) {
            *layers = TileLayers::new(layers.terrain(), layers.actor(), effect);
        }
    }

    fn indexed_tiles(&self) -> Vec<IndexedTile> {
        self.tiles
            .iter()
            .map(|(tile, layers)| IndexedTile::new(*tile, *layers))
            .collect()
    }

    fn actors_vec(&self) -> Vec<Actor> {
        self.actors.values().copied().collect()
    }

    fn hazard_cells(&self) -> Vec<HazardCell> {
        self.tiles
            .iter()
            .map(|(tile, layers)| HazardCell::new(*tile, *layers))
            .collect()
    }

    fn snapshot(&self, tick_index: i64, resources: Resources) -> Snapshot {
        Snapshot::from_parts(
            self.saucer,
            tick_index,
            self.indexed_tiles(),
            self.actors_vec(),
            resources,
        )
    }

    fn vegetation_snapshot(&self, tick_index: i64) -> IndexedSnapshot {
        IndexedSnapshot::new(tick_index, self.indexed_tiles(), self.actors_vec())
    }
}

fn evaluate(input: QueryInput<'_, ReferenceContext, GameJournalEntry>) -> Snapshot {
    let context = *input.context_payload();
    let target_time = input.logical_time();
    let target_tick = input.game_tick_index();
    let visible_entries = input
        .visible_entries()
        .map(|entry| (entry.logical_time(), entry.clone()))
        .collect::<Vec<_>>();
    let mut cursor = 0;
    let mut working = WorkingState::default();

    if target_tick < 0 {
        while cursor < visible_entries.len() && visible_entries[cursor].0 <= target_time {
            working.apply_entry(&visible_entries[cursor].1, context);
            cursor += 1;
        }
        return with_resources(working, target_tick, vec![]);
    }

    while cursor < visible_entries.len() && visible_entries[cursor].0 <= LogicalTime::zero() {
        working.apply_entry(&visible_entries[cursor].1, context);
        cursor += 1;
    }

    let mut history = vec![working.vegetation_snapshot(0)];

    for next_tick in 1..=target_tick {
        let next_time = LogicalTime::from_game_ticks(next_tick)
            .expect("the selected game-tick boundary is representable");
        while cursor < visible_entries.len() && visible_entries[cursor].0 < next_time {
            working.apply_entry(&visible_entries[cursor].1, context);
            cursor += 1;
        }

        working = transition(&working, next_tick - 1, next_tick, &history);

        while cursor < visible_entries.len() && visible_entries[cursor].0 <= next_time {
            working.apply_entry(&visible_entries[cursor].1, context);
            cursor += 1;
        }

        history.push(working.vegetation_snapshot(next_tick));
    }

    while cursor < visible_entries.len() && visible_entries[cursor].0 <= target_time {
        working.apply_entry(&visible_entries[cursor].1, context);
        cursor += 1;
    }

    with_resources(working, target_tick, history)
}

fn with_resources(
    working: WorkingState,
    tick_index: i64,
    mut history: Vec<IndexedSnapshot>,
) -> Snapshot {
    let current = working.snapshot(tick_index, Resources::default());
    let indexed_current = IndexedSnapshot::new(
        tick_index,
        current.tiles().iter().copied(),
        current.actors().iter().copied(),
    );
    if history.is_empty() {
        history.push(indexed_current.clone());
    } else {
        history.pop();
        history.push(indexed_current.clone());
    }

    let input = IndexedInput::at_tick(&indexed_current, &history, tick_index);
    let wheat = Wheat.query(&input);
    let forester = Forester.query(&input);
    let resources = Resources::new(
        wheat.indexed_total(),
        forester
            .action()
            .map(|action| action.indexed_wood_total())
            .unwrap_or(0),
    );

    Snapshot::from_parts(
        current.saucer(),
        tick_index,
        current.tiles().iter().copied(),
        current.actors().iter().copied(),
        resources,
    )
}

fn transition(
    current: &WorkingState,
    current_tick: i64,
    next_tick: i64,
    history: &[IndexedSnapshot],
) -> WorkingState {
    let current_snapshot = current.snapshot(current_tick, Resources::default());
    let actors = current.actors_vec();
    let cells = current.hazard_cells();
    let mut next = current.clone();
    let mut desired_tiles = BTreeMap::<ActorId, TileId>::new();
    let mut removed = BTreeSet::<ActorId>::new();
    let mut wheat_tiles = BTreeSet::<TileId>::new();
    let mut conversions = BTreeSet::<TileId>::new();
    let mut ignitions = BTreeSet::<TileId>::new();
    let mut fighter_collisions = Vec::<(ActorId, ActorId)>::new();

    for actor in actors
        .iter()
        .filter(|actor| actor.kind() == ActorKind::Farmer)
    {
        let filtered = single_kind_snapshot(&current_snapshot, *actor, ActorKind::Farmer);
        let input = IndexedInput::at_tick(&filtered, history, current_tick);
        if let FarmerResult::Completed(action) = Farmer.query(&input) {
            removed.insert(actor.id());
            wheat_tiles.extend(action.wheat_tiles().iter().copied());
        }
    }

    for actor in actors
        .iter()
        .filter(|actor| actor.kind() == ActorKind::Forester)
    {
        let filtered = single_kind_snapshot(&current_snapshot, *actor, ActorKind::Forester);
        let input = IndexedInput::at_tick(&filtered, history, current_tick);
        if let Some(action) = Forester.query(&input).action() {
            desired_tiles.insert(actor.id(), action.destination());
        }
    }

    for actor in actors
        .iter()
        .filter(|actor| actor.kind() == ActorKind::Arsonist)
    {
        let result = ArsonistDefinition::new(*actor, &actors, &cells).evaluate_at_tick(next_tick);
        if result.arsonist_removed() {
            removed.insert(actor.id());
        }
        ignitions.extend(result.ignitions().iter().map(|fire| fire.tile()));
    }

    for actor in actors
        .iter()
        .filter(|actor| actor.kind() == ActorKind::Fighter)
    {
        let result = FighterDefinition::new(*actor, &actors).evaluate_at_tick(next_tick);
        desired_tiles.insert(actor.id(), result.actor().tile());
        if let Some(target) = result.removed_arsonist() {
            fighter_collisions.push((actor.id(), target));
        }
    }

    for actor in actors
        .iter()
        .filter(|actor| actor.kind() == ActorKind::Arborist)
    {
        let age = current
            .spawn_ticks
            .get(&actor.id())
            .map(|spawn_tick| arborist_age(next_tick, *spawn_tick))
            .unwrap_or(0);
        let terrain = current
            .tiles
            .get(&actor.tile())
            .map(|layers| layers.terrain())
            .unwrap_or(Terrain::Void);
        let result = ArboristDefinition::new(*actor, terrain, age).evaluate_at_tick(next_tick);
        if result.converted() {
            conversions.insert(actor.tile());
        }
    }

    for tile in wheat_tiles {
        next.set_terrain(tile, Terrain::Wheat);
    }
    for tile in conversions {
        next.set_terrain(tile, Terrain::Forest);
    }

    let advanced_cells = cells
        .iter()
        .map(|cell| {
            let effect = match cell.effect() {
                Effect::None => Effect::None,
                Effect::Fire { age_in_game_ticks } => {
                    Effect::fire(age_in_game_ticks.saturating_add(1))
                }
            };
            HazardCell::new(
                cell.tile(),
                TileLayers::new(cell.terrain(), cell.actor(), effect),
            )
        })
        .collect::<Vec<_>>();
    let fire_result = FireDefinition::new(&advanced_cells).evaluate_at_tick(next_tick);
    let mut spread = BTreeSet::<TileId>::new();

    for outcome in fire_result.outcomes() {
        match outcome {
            FireOutcome::Burning {
                tile,
                terrain,
                effect,
            } => {
                if let Some(layers) = next.tiles.get_mut(tile) {
                    *layers = TileLayers::new(*terrain, layers.actor(), *effect);
                }
            }
            FireOutcome::Destroyed {
                tile,
                terrain,
                spread: outcome_spread,
            } => {
                if let Some(layers) = next.tiles.get_mut(tile) {
                    *layers = TileLayers::new(*terrain, layers.actor(), Effect::None);
                }
                spread.extend(outcome_spread.iter().map(|fire| fire.tile()));
            }
            FireOutcome::UnsupportedAge {
                tile,
                age_in_game_ticks: _,
            } => next.set_effect(*tile, Effect::None),
        }
    }

    for tile in spread.into_iter().chain(ignitions) {
        if next
            .tiles
            .get(&tile)
            .is_some_and(|layers| matches!(layers.terrain(), Terrain::Wheat | Terrain::Forest))
        {
            next.set_effect(tile, Effect::fire(0));
        }
    }

    let mut candidates = BTreeMap::<ActorId, TileId>::new();
    for actor in &actors {
        if !removed.contains(&actor.id()) {
            candidates.insert(
                actor.id(),
                desired_tiles
                    .get(&actor.id())
                    .copied()
                    .unwrap_or(actor.tile()),
            );
        }
    }

    let mut winners = BTreeMap::<TileId, ActorId>::new();
    for (actor_id, tile) in &candidates {
        winners
            .entry(*tile)
            .and_modify(|winner| *winner = (*winner).min(*actor_id))
            .or_insert(*actor_id);
    }

    let mut final_tiles = BTreeMap::<ActorId, TileId>::new();
    for actor in &actors {
        if let Some(candidate) = candidates.get(&actor.id()) {
            let tile = if winners.get(candidate) == Some(&actor.id()) {
                *candidate
            } else {
                actor.tile()
            };
            final_tiles.insert(actor.id(), tile);
        }
    }

    for (fighter, target) in fighter_collisions {
        if final_tiles.get(&fighter).copied()
            == current.actors.get(&target).map(|actor| actor.tile())
        {
            removed.insert(target);
        }
    }

    next.actors = actors
        .iter()
        .filter_map(|actor| {
            if removed.contains(&actor.id()) {
                return None;
            }
            let tile = final_tiles
                .get(&actor.id())
                .copied()
                .unwrap_or(actor.tile());
            Some((actor.id(), Actor::new(actor.id(), actor.kind(), tile)))
        })
        .collect();
    next.spawn_ticks
        .retain(|actor_id, _| next.actors.contains_key(actor_id));
    next.sync_actor_layers();
    next
}

fn single_kind_snapshot(snapshot: &Snapshot, selected: Actor, kind: ActorKind) -> IndexedSnapshot {
    let actors = snapshot
        .actors()
        .iter()
        .filter(|actor| actor.kind() != kind || actor.id() == selected.id())
        .copied()
        .collect::<Vec<_>>();
    IndexedSnapshot::new(
        snapshot.tick_index(),
        snapshot.tiles().iter().copied(),
        actors,
    )
}

fn arborist_age(next_tick: i64, spawn_tick: i64) -> u32 {
    if next_tick <= spawn_tick {
        0
    } else {
        next_tick.saturating_sub(spawn_tick).min(u32::MAX as i64) as u32
    }
}
