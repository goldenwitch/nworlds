use caravan_domain::{Actor, ActorKind, Resources, Terrain, TileId};

use crate::helpers::{actor_of_kind, layers_at, occupied_by};
use crate::{VegetationQueryInput, VegetationSnapshot};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Forester;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ForesterResult {
    NoForester,
    Present(ForesterAction),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ForesterAction {
    actor: Actor,
    destination: TileId,
    moved: bool,
    wood_produced: u64,
    indexed_wood_total: u64,
}

impl Forester {
    pub fn query<I>(&self, input: &I) -> ForesterResult
    where
        I: VegetationQueryInput,
    {
        let Some(forester) = actor_of_kind(input, ActorKind::Forester) else {
            return ForesterResult::NoForester;
        };

        let origin = forester.tile();
        let on_forest = layers_at(input, origin).terrain() == Terrain::Forest;
        let destination = if on_forest {
            origin
        } else {
            origin
                .neighbors()
                .into_iter()
                .flatten()
                .find(|tile| !occupied_by(input, *tile, Some(forester.id())))
                .unwrap_or(origin)
        };
        let indexed_wood_total = indexed_wood_total(input);

        ForesterResult::Present(ForesterAction {
            actor: forester,
            destination,
            moved: destination != origin,
            wood_produced: u64::from(on_forest),
            indexed_wood_total,
        })
    }
}

impl ForesterResult {
    pub fn action(&self) -> Option<&ForesterAction> {
        match self {
            Self::NoForester => None,
            Self::Present(action) => Some(action),
        }
    }
}

impl ForesterAction {
    pub const fn actor(&self) -> Actor {
        self.actor
    }

    pub const fn origin(&self) -> TileId {
        self.actor.tile()
    }

    pub const fn destination(&self) -> TileId {
        self.destination
    }

    pub const fn moved(&self) -> bool {
        self.moved
    }

    pub const fn wood_produced(&self) -> u64 {
        self.wood_produced
    }

    pub const fn indexed_wood_total(&self) -> u64 {
        self.indexed_wood_total
    }

    pub const fn resources(&self) -> Resources {
        Resources::new(0, self.indexed_wood_total)
    }
}

fn indexed_wood_total<I>(input: &I) -> u64
where
    I: VegetationQueryInput,
{
    let mut total: u64 = 0;
    for snapshot in input.indexed_snapshots() {
        let tick_index = snapshot.tick_index();
        if tick_index >= 0 && tick_index <= input.game_tick_index() {
            total = total
                .checked_add(forester_count_on_forest(snapshot))
                .expect("indexed wood total overflowed u64");
        }
    }
    total
}

fn forester_count_on_forest<S>(snapshot: &S) -> u64
where
    S: VegetationSnapshot,
{
    snapshot
        .actors()
        .iter()
        .filter(|actor| {
            actor.kind() == ActorKind::Forester
                && snapshot
                    .layers_at(actor.tile())
                    .unwrap_or_default()
                    .terrain()
                    == Terrain::Forest
        })
        .count() as u64
}
