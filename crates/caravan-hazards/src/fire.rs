use caravan_domain::{Effect, Terrain, TileId};
use engine_index::{IndexedQuery, QueryInput};

use crate::HazardCell;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FireStart {
    tile: TileId,
    effect: Effect,
}

impl FireStart {
    pub const fn new(tile: TileId) -> Self {
        Self {
            tile,
            effect: Effect::fire(0),
        }
    }

    pub const fn tile(self) -> TileId {
        self.tile
    }

    pub const fn effect(self) -> Effect {
        self.effect
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum FireOutcome {
    Burning {
        tile: TileId,
        terrain: Terrain,
        effect: Effect,
    },
    Destroyed {
        tile: TileId,
        terrain: Terrain,
        spread: Vec<FireStart>,
    },
    UnsupportedAge {
        tile: TileId,
        age_in_game_ticks: u32,
    },
}

impl FireOutcome {
    pub const fn tile(&self) -> TileId {
        match self {
            Self::Burning { tile, .. }
            | Self::Destroyed { tile, .. }
            | Self::UnsupportedAge { tile, .. } => *tile,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FireResult {
    game_tick_index: i64,
    outcomes: Vec<FireOutcome>,
}

impl FireResult {
    pub const fn game_tick_index(&self) -> i64 {
        self.game_tick_index
    }

    pub fn outcomes(&self) -> &[FireOutcome] {
        &self.outcomes
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FireDefinition<'a> {
    cells: &'a [HazardCell],
}

impl<'a> FireDefinition<'a> {
    pub const fn new(cells: &'a [HazardCell]) -> Self {
        Self { cells }
    }

    pub fn evaluate_at_tick(&self, game_tick_index: i64) -> FireResult {
        let mut cells = self.cells.iter().collect::<Vec<_>>();
        cells.sort_by_key(|cell| cell.tile());

        let outcomes = cells
            .iter()
            .filter_map(|cell| match cell.effect() {
                Effect::None => None,
                Effect::Fire {
                    age_in_game_ticks: 0..=2,
                } => Some(FireOutcome::Burning {
                    tile: cell.tile(),
                    terrain: cell.terrain(),
                    effect: cell.effect(),
                }),
                Effect::Fire {
                    age_in_game_ticks: 3,
                } => Some(FireOutcome::Destroyed {
                    tile: cell.tile(),
                    terrain: destroyed_terrain(cell.terrain()),
                    spread: spread_from(&cells, cell.tile()),
                }),
                Effect::Fire { age_in_game_ticks } => Some(FireOutcome::UnsupportedAge {
                    tile: cell.tile(),
                    age_in_game_ticks,
                }),
            })
            .collect();

        FireResult {
            game_tick_index,
            outcomes,
        }
    }
}

impl<C, P> IndexedQuery<C, P> for FireDefinition<'_> {
    type Result = FireResult;

    fn query(&self, input: QueryInput<'_, C, P>) -> Self::Result {
        self.evaluate_at_tick(input.game_tick_index())
    }
}

pub(crate) const fn is_burnable(terrain: Terrain) -> bool {
    matches!(terrain, Terrain::Wheat | Terrain::Forest)
}

const fn destroyed_terrain(terrain: Terrain) -> Terrain {
    if is_burnable(terrain) {
        Terrain::Void
    } else {
        terrain
    }
}

fn spread_from(cells: &[&HazardCell], source: TileId) -> Vec<FireStart> {
    source
        .neighbors()
        .into_iter()
        .flatten()
        .filter_map(|tile| {
            cells
                .iter()
                .find(|cell| cell.tile() == tile)
                .filter(|cell| is_burnable(cell.terrain()) && matches!(cell.effect(), Effect::None))
                .map(|_| FireStart::new(tile))
        })
        .collect()
}
