use caravan_domain::{Actor, ActorId, ActorKind, TileId};
use engine_index::{IndexedQuery, QueryInput};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FighterResult {
    game_tick_index: i64,
    actor: Actor,
    selected_arsonist: Option<ActorId>,
    removed_arsonist: Option<ActorId>,
}

impl FighterResult {
    pub const fn game_tick_index(&self) -> i64 {
        self.game_tick_index
    }

    pub const fn actor(&self) -> Actor {
        self.actor
    }

    pub const fn selected_arsonist(&self) -> Option<ActorId> {
        self.selected_arsonist
    }

    pub const fn removed_arsonist(&self) -> Option<ActorId> {
        self.removed_arsonist
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FighterDefinition<'a> {
    fighter: Actor,
    actors: &'a [Actor],
}

impl<'a> FighterDefinition<'a> {
    pub const fn new(fighter: Actor, actors: &'a [Actor]) -> Self {
        Self { fighter, actors }
    }

    pub fn evaluate_at_tick(&self, game_tick_index: i64) -> FighterResult {
        let target = self
            .actors
            .iter()
            .filter(|actor| actor.kind() == ActorKind::Arsonist)
            .min_by_key(|actor| actor.id());

        let destination = target
            .map(|target| next_destination(self.fighter.tile(), target.tile()))
            .unwrap_or(self.fighter.tile());
        let actor = Actor::new(self.fighter.id(), self.fighter.kind(), destination);
        let removed_arsonist = target
            .filter(|target| target.tile() == destination)
            .map(|target| target.id());

        FighterResult {
            game_tick_index,
            actor,
            selected_arsonist: target.map(|target| target.id()),
            removed_arsonist,
        }
    }
}

impl<C, P> IndexedQuery<C, P> for FighterDefinition<'_> {
    type Result = FighterResult;

    fn query(&self, input: QueryInput<'_, C, P>) -> Self::Result {
        self.evaluate_at_tick(input.game_tick_index())
    }
}

fn next_destination(current: TileId, target: TileId) -> TileId {
    let target_distance = current.axial().distance_to(target.axial());
    let mut destination = current;
    let mut best_distance = target_distance;

    for neighbor in current.neighbors().into_iter().flatten() {
        let distance = neighbor.axial().distance_to(target.axial());
        if distance < best_distance {
            destination = neighbor;
            best_distance = distance;
        }
    }

    destination
}
