use core::{fmt, num::NonZeroU64};

use crate::TileId;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActorId(NonZeroU64);

impl ActorId {
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for ActorId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(formatter)
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActorKind {
    Farmer,
    Forester,
    Arsonist,
    Fighter,
    Arborist,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Actor {
    id: ActorId,
    kind: ActorKind,
    tile: TileId,
}

impl Actor {
    pub const fn new(id: ActorId, kind: ActorKind, tile: TileId) -> Self {
        Self { id, kind, tile }
    }

    pub const fn id(self) -> ActorId {
        self.id
    }

    pub const fn kind(self) -> ActorKind {
        self.kind
    }

    pub const fn tile(self) -> TileId {
        self.tile
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Terrain {
    #[default]
    Void,
    Wheat,
    Forest,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Effect {
    #[default]
    None,
    Fire {
        age_in_game_ticks: u32,
    },
}

impl Effect {
    pub const fn fire(age_in_game_ticks: u32) -> Self {
        Self::Fire { age_in_game_ticks }
    }

    pub const fn fire_age(self) -> Option<u32> {
        match self {
            Self::None => None,
            Self::Fire { age_in_game_ticks } => Some(age_in_game_ticks),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TileLayers {
    terrain: Terrain,
    actor: Option<ActorId>,
    effect: Effect,
}

impl TileLayers {
    pub const fn new(terrain: Terrain, actor: Option<ActorId>, effect: Effect) -> Self {
        Self {
            terrain,
            actor,
            effect,
        }
    }

    pub const fn terrain(self) -> Terrain {
        self.terrain
    }

    pub const fn actor(self) -> Option<ActorId> {
        self.actor
    }

    pub const fn effect(self) -> Effect {
        self.effect
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Resources {
    wheat: u64,
    wood: u64,
}

impl Resources {
    pub const fn new(wheat: u64, wood: u64) -> Self {
        Self { wheat, wood }
    }

    pub const fn wheat(self) -> u64 {
        self.wheat
    }

    pub const fn wood(self) -> u64 {
        self.wood
    }

    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match (
            self.wheat.checked_add(other.wheat),
            self.wood.checked_add(other.wood),
        ) {
            (Some(wheat), Some(wood)) => Some(Self { wheat, wood }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Axial, TileId};

    use super::{Actor, ActorId, ActorKind, Effect, Resources, Terrain, TileLayers};

    #[test]
    fn actor_ids_are_positive_and_stably_ordered() {
        let first = ActorId::new(1).expect("positive IDs are valid");
        let second = ActorId::new(2).expect("positive IDs are valid");

        assert!(ActorId::new(0).is_none());
        assert_eq!(first.get(), 1);
        assert!(first < second);
    }

    #[test]
    fn actor_values_keep_identity_kind_and_tile_together() {
        let tile = TileId::from_axial(Axial::new(2, -1)).expect("tile is inside the saucer");
        let actor = Actor::new(
            ActorId::new(7).expect("positive IDs are valid"),
            ActorKind::Arborist,
            tile,
        );

        assert_eq!(actor.id().get(), 7);
        assert_eq!(actor.kind(), ActorKind::Arborist);
        assert_eq!(actor.tile(), tile);
    }

    #[test]
    fn terrain_effect_and_actor_slots_are_independent_layers() {
        let actor_id = ActorId::new(3).expect("positive IDs are valid");
        let layers = TileLayers::new(Terrain::Forest, Some(actor_id), Effect::fire(2));

        assert_eq!(layers.terrain(), Terrain::Forest);
        assert_eq!(layers.actor(), Some(actor_id));
        assert_eq!(
            layers.effect(),
            Effect::Fire {
                age_in_game_ticks: 2
            }
        );
        assert_eq!(Effect::None.fire_age(), None);
    }

    #[test]
    fn resources_are_nonnegative_and_checked() {
        let resources = Resources::new(4, 9);

        assert_eq!(resources.wheat(), 4);
        assert_eq!(resources.wood(), 9);
        assert_eq!(
            resources.checked_add(Resources::new(2, 3)),
            Some(Resources::new(6, 12))
        );
        assert_eq!(
            Resources::new(u64::MAX, 0).checked_add(Resources::new(1, 0)),
            None
        );
    }
}
