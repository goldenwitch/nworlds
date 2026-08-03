use caravan_domain::{Actor, Terrain};
use engine_index::{IndexedQuery, QueryInput};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ArboristResult {
    game_tick_index: i64,
    actor: Actor,
    terrain: Terrain,
    conversion_age_in_game_ticks: u32,
    converted: bool,
}

impl ArboristResult {
    pub const fn game_tick_index(&self) -> i64 {
        self.game_tick_index
    }

    pub const fn actor(&self) -> Actor {
        self.actor
    }

    pub const fn terrain(&self) -> Terrain {
        self.terrain
    }

    pub const fn conversion_age_in_game_ticks(&self) -> u32 {
        self.conversion_age_in_game_ticks
    }

    pub const fn converted(&self) -> bool {
        self.converted
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ArboristDefinition {
    arborist: Actor,
    terrain: Terrain,
    conversion_age_in_game_ticks: u32,
}

impl ArboristDefinition {
    pub const fn new(arborist: Actor, terrain: Terrain, conversion_age_in_game_ticks: u32) -> Self {
        Self {
            arborist,
            terrain,
            conversion_age_in_game_ticks,
        }
    }

    pub fn evaluate_at_tick(&self, game_tick_index: i64) -> ArboristResult {
        let converted = self.conversion_age_in_game_ticks >= 3;

        ArboristResult {
            game_tick_index,
            actor: self.arborist,
            terrain: if converted {
                Terrain::Forest
            } else {
                self.terrain
            },
            conversion_age_in_game_ticks: self.conversion_age_in_game_ticks.min(3),
            converted,
        }
    }
}

impl<C, P> IndexedQuery<C, P> for ArboristDefinition {
    type Result = ArboristResult;

    fn query(&self, input: QueryInput<'_, C, P>) -> Self::Result {
        self.evaluate_at_tick(input.game_tick_index())
    }
}
