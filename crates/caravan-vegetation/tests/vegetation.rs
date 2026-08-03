use caravan_domain::{Actor, ActorId, ActorKind, Effect, Resources, Terrain, TileId, TileLayers};
use caravan_vegetation::{
    Farmer, FarmerResult, Forest, Forester, IndexedInput, IndexedTile, Snapshot,
    VegetationSnapshot, Wheat,
};

fn tile(q: i32, r: i32) -> TileId {
    TileId::new(q, r).expect("test coordinate is inside the saucer")
}

fn actor(id: u64, kind: ActorKind, tile: TileId) -> Actor {
    Actor::new(
        ActorId::new(id).expect("test actor IDs are positive"),
        kind,
        tile,
    )
}

fn indexed_tile(
    tile: TileId,
    terrain: Terrain,
    actor: Option<ActorId>,
    effect: Effect,
) -> IndexedTile {
    IndexedTile::new(tile, TileLayers::new(terrain, actor, effect))
}

#[test]
fn farmer_uses_fixed_order_disappears_and_places_wheat_around_destination() {
    let farmer = actor(1, ActorKind::Farmer, tile(0, 0));
    let blocker = actor(2, ActorKind::Forester, tile(1, -1));
    let snapshot = Snapshot::new(
        0,
        [
            indexed_tile(tile(0, 0), Terrain::Void, Some(farmer.id()), Effect::None),
            indexed_tile(tile(1, 0), Terrain::Void, None, Effect::None),
            indexed_tile(tile(2, 0), Terrain::Forest, None, Effect::None),
            indexed_tile(tile(2, -1), Terrain::Void, None, Effect::fire(0)),
            indexed_tile(tile(1, -1), Terrain::Void, Some(blocker.id()), Effect::None),
            indexed_tile(tile(0, 1), Terrain::Void, None, Effect::None),
            indexed_tile(tile(1, 1), Terrain::Void, None, Effect::None),
        ],
        [farmer, blocker],
    );
    let input = IndexedInput::new(&snapshot, std::slice::from_ref(&snapshot));

    let result = Farmer.query(&input);
    let FarmerResult::Completed(action) = result else {
        panic!("the fixture contains a farmer");
    };

    assert_eq!(action.origin(), tile(0, 0));
    assert_eq!(action.destination(), tile(1, 0));
    assert!(action.moved());
    assert_eq!(action.wheat_tiles(), &[tile(0, 0), tile(0, 1), tile(1, 1)]);
    assert_eq!(action.actor_after_action(), None);
    assert_eq!(snapshot.actors(), &[farmer, blocker]);
    assert_eq!(
        snapshot
            .layers_at(tile(0, 0))
            .expect("origin layer")
            .terrain(),
        Terrain::Void
    );
}

#[test]
fn wheat_counts_each_indexed_tick_without_carrying_query_state() {
    let snapshots = [
        Snapshot::new(
            0,
            [
                indexed_tile(tile(0, 0), Terrain::Wheat, None, Effect::None),
                indexed_tile(tile(1, 0), Terrain::Wheat, None, Effect::None),
            ],
            [],
        ),
        Snapshot::new(
            1,
            [indexed_tile(tile(0, 0), Terrain::Wheat, None, Effect::None)],
            [],
        ),
        Snapshot::new(2, [], []),
    ];
    let input = IndexedInput::new(&snapshots[2], &snapshots);

    let result = Wheat.query(&input);

    assert_eq!(result.current_tile_count(), 0);
    assert_eq!(result.current_tiles(), &[]);
    assert_eq!(result.indexed_total(), 3);
    assert_eq!(result.resources(), Resources::new(3, 0));
}

#[test]
fn forest_returns_only_the_indexed_forest_tiles_in_stable_order() {
    let snapshot = Snapshot::new(
        0,
        [
            indexed_tile(tile(2, -1), Terrain::Forest, None, Effect::None),
            indexed_tile(tile(-1, 0), Terrain::Wheat, None, Effect::None),
            indexed_tile(tile(0, 1), Terrain::Forest, None, Effect::None),
        ],
        [],
    );
    let input = IndexedInput::new(&snapshot, std::slice::from_ref(&snapshot));

    let result = Forest.query(&input);

    assert_eq!(result.tiles(), &[tile(0, 1), tile(2, -1)]);
    assert_eq!(result.tile_count(), 2);
}

#[test]
fn forester_uses_fixed_order_when_off_forest() {
    let forester = actor(1, ActorKind::Forester, tile(0, 0));
    let blocker = actor(2, ActorKind::Farmer, tile(1, 0));
    let snapshot = Snapshot::new(
        0,
        [
            indexed_tile(tile(0, 0), Terrain::Void, Some(forester.id()), Effect::None),
            indexed_tile(tile(1, 0), Terrain::Void, Some(blocker.id()), Effect::None),
            indexed_tile(tile(1, -1), Terrain::Void, None, Effect::None),
        ],
        [forester, blocker],
    );
    let input = IndexedInput::new(&snapshot, std::slice::from_ref(&snapshot));

    let result = Forester.query(&input);
    let action = result.action().expect("the fixture contains a forester");

    assert_eq!(action.destination(), tile(1, -1));
    assert!(action.moved());
    assert_eq!(action.wood_produced(), 0);
}

#[test]
fn forester_stays_on_forest_and_wood_is_an_indexed_total() {
    let forester = actor(1, ActorKind::Forester, tile(0, 0));
    let snapshots = [
        Snapshot::new(
            0,
            [indexed_tile(
                tile(0, 0),
                Terrain::Forest,
                Some(forester.id()),
                Effect::None,
            )],
            [forester],
        ),
        Snapshot::new(
            1,
            [indexed_tile(
                tile(0, 0),
                Terrain::Void,
                Some(forester.id()),
                Effect::None,
            )],
            [forester],
        ),
        Snapshot::new(
            2,
            [indexed_tile(
                tile(0, 0),
                Terrain::Forest,
                Some(forester.id()),
                Effect::None,
            )],
            [forester],
        ),
    ];
    let input = IndexedInput::new(&snapshots[2], &snapshots);

    let result = Forester.query(&input);
    let action = result.action().expect("the fixture contains a forester");

    assert_eq!(action.destination(), tile(0, 0));
    assert!(!action.moved());
    assert_eq!(action.wood_produced(), 1);
    assert_eq!(action.indexed_wood_total(), 2);
    assert_eq!(action.resources(), Resources::new(0, 2));
}
