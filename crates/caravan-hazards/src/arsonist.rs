use caravan_domain::{Actor, ActorId};
use engine_index::{IndexedQuery, QueryInput};

use crate::fire::{is_burnable, FireStart};
use crate::HazardCell;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ArsonistResult {
    game_tick_index: i64,
    arsonist: ActorId,
    target: Option<ActorId>,
    arsonist_removed: bool,
    ignitions: Vec<FireStart>,
}

impl ArsonistResult {
    pub const fn game_tick_index(&self) -> i64 {
        self.game_tick_index
    }

    pub const fn arsonist(&self) -> ActorId {
        self.arsonist
    }

    pub const fn target(&self) -> Option<ActorId> {
        self.target
    }

    pub const fn arsonist_removed(&self) -> bool {
        self.arsonist_removed
    }

    pub fn ignitions(&self) -> &[FireStart] {
        &self.ignitions
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ArsonistDefinition<'a> {
    arsonist: Actor,
    actors: &'a [Actor],
    cells: &'a [HazardCell],
}

impl<'a> ArsonistDefinition<'a> {
    pub const fn new(arsonist: Actor, actors: &'a [Actor], cells: &'a [HazardCell]) -> Self {
        Self {
            arsonist,
            actors,
            cells,
        }
    }

    pub fn evaluate_at_tick(&self, game_tick_index: i64) -> ArsonistResult {
        let target = self
            .actors
            .iter()
            .filter(|actor| actor.id() != self.arsonist.id())
            .min_by_key(|actor| (actor.tile(), actor.id()));

        let Some(target) = target else {
            return ArsonistResult {
                game_tick_index,
                arsonist: self.arsonist.id(),
                target: None,
                arsonist_removed: false,
                ignitions: Vec::new(),
            };
        };

        let ignitions = self
            .arsonist
            .tile()
            .neighbors()
            .into_iter()
            .flatten()
            .filter_map(|tile| {
                self.cells
                    .iter()
                    .find(|cell| cell.tile() == tile)
                    .filter(|cell| is_burnable(cell.terrain()))
                    .map(|_| FireStart::new(tile))
            })
            .collect();

        ArsonistResult {
            game_tick_index,
            arsonist: self.arsonist.id(),
            target: Some(target.id()),
            arsonist_removed: true,
            ignitions,
        }
    }
}

impl<C, P> IndexedQuery<C, P> for ArsonistDefinition<'_> {
    type Result = ArsonistResult;

    fn query(&self, input: QueryInput<'_, C, P>) -> Self::Result {
        self.evaluate_at_tick(input.game_tick_index())
    }
}
