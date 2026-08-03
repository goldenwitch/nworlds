use caravan_domain::{
    Actor, ActorId, ActorKind, Effect, GameJournalEntry, Terrain, TileId, TileLayers,
};
use caravan_hazards::{
    ArboristDefinition, ArsonistDefinition, FighterDefinition, FireDefinition, FireOutcome,
    HazardCell,
};
use engine_index::{state, IndexedQuery};
use engine_sdk::{Context, GameState, Journal as SdkJournal};
use engine_time::LogicalTime;

fn actor_id(value: u64) -> ActorId {
    ActorId::new(value).expect("test actor IDs are positive")
}

fn tile(q: i32, r: i32) -> TileId {
    TileId::new(q, r).expect("test tile is inside the saucer")
}

fn actor(id: u64, kind: ActorKind, tile: TileId) -> Actor {
    Actor::new(actor_id(id), kind, tile)
}

fn cell(tile: TileId, terrain: Terrain, effect: Effect) -> HazardCell {
    HazardCell::new(tile, TileLayers::new(terrain, None, effect))
}

fn evaluate<Q>(query: Q, tick: i64) -> GameState<Q::Result>
where
    Q: IndexedQuery<(), GameJournalEntry>,
{
    let context = Context::new(());
    let journal = SdkJournal::<GameJournalEntry>::empty();

    state(
        &context,
        &journal,
        LogicalTime::from_game_ticks(tick).expect("hazard test game-tick times are representable"),
        query,
    )
}

#[test]
fn arsonist_uses_coordinate_then_identifier_order_and_emits_fixed_order_ignitions() {
    let arsonist = actor(9, ActorKind::Arsonist, TileId::origin());
    let actors = [
        arsonist,
        actor(6, ActorKind::Farmer, tile(-1, 1)),
        actor(3, ActorKind::Forester, tile(-1, 1)),
        actor(4, ActorKind::Fighter, tile(1, 0)),
    ];
    let cells = [
        cell(tile(1, 0), Terrain::Wheat, Effect::None),
        cell(tile(1, -1), Terrain::Void, Effect::None),
        cell(tile(0, -1), Terrain::Forest, Effect::None),
        cell(tile(-1, 0), Terrain::Wheat, Effect::None),
        cell(tile(-1, 1), Terrain::Forest, Effect::None),
        cell(tile(0, 1), Terrain::Void, Effect::None),
    ];
    let original_actors = actors;
    let original_cells = cells;

    let state = evaluate(ArsonistDefinition::new(arsonist, &actors, &cells), 4);
    let result = state.payload();

    assert_eq!(result.game_tick_index(), 4);
    assert_eq!(result.target(), Some(actor_id(3)));
    assert!(result.arsonist_removed());
    assert_eq!(
        result
            .ignitions()
            .iter()
            .map(|fire| fire.tile())
            .collect::<Vec<_>>(),
        vec![tile(1, 0), tile(0, -1), tile(-1, 0), tile(-1, 1)]
    );
    assert_eq!(actors, original_actors);
    assert_eq!(cells, original_cells);
}

#[test]
fn arsonist_without_a_target_remains_and_does_not_ignite() {
    let arsonist = actor(9, ActorKind::Arsonist, TileId::origin());
    let actors = [arsonist];
    let cells = [cell(tile(1, 0), Terrain::Wheat, Effect::None)];

    let state = evaluate(ArsonistDefinition::new(arsonist, &actors, &cells), 0);

    assert_eq!(state.payload().target(), None);
    assert!(!state.payload().arsonist_removed());
    assert!(state.payload().ignitions().is_empty());
}

#[test]
fn fire_reports_ages_zero_through_two_and_age_three_destruction_without_mutation() {
    let cells = [
        cell(tile(-2, 0), Terrain::Wheat, Effect::fire(0)),
        cell(tile(-1, 0), Terrain::Forest, Effect::fire(1)),
        cell(tile(0, -1), Terrain::Wheat, Effect::fire(2)),
        cell(TileId::origin(), Terrain::Forest, Effect::fire(3)),
        cell(tile(1, 0), Terrain::Wheat, Effect::None),
        cell(tile(1, -1), Terrain::Void, Effect::None),
        cell(tile(0, 1), Terrain::Forest, Effect::None),
    ];
    let original_cells = cells;

    let first = evaluate(FireDefinition::new(&cells), 12);
    let repeated = evaluate(FireDefinition::new(&cells), 12);

    assert_eq!(first, repeated);
    assert_eq!(first.payload().game_tick_index(), 12);
    assert!(first.payload().outcomes().iter().any(|outcome| {
        matches!(
            outcome,
            FireOutcome::Burning {
                tile: outcome_tile,
                terrain: Terrain::Wheat,
                effect: Effect::Fire {
                    age_in_game_ticks: 0
                }
            } if *outcome_tile == tile(-2, 0)
        )
    }));
    assert!(first.payload().outcomes().iter().any(|outcome| {
        matches!(
            outcome,
            FireOutcome::Burning {
                tile: outcome_tile,
                terrain: Terrain::Forest,
                effect: Effect::Fire {
                    age_in_game_ticks: 1
                }
            } if *outcome_tile == tile(-1, 0)
        )
    }));
    assert!(first.payload().outcomes().iter().any(|outcome| {
        matches!(
            outcome,
            FireOutcome::Burning {
                tile: outcome_tile,
                terrain: Terrain::Wheat,
                effect: Effect::Fire {
                    age_in_game_ticks: 2
                }
            } if *outcome_tile == tile(0, -1)
        )
    }));

    let destroyed = first
        .payload()
        .outcomes()
        .iter()
        .find(|outcome| outcome.tile() == TileId::origin())
        .expect("age-three fire has a result");
    match destroyed {
        FireOutcome::Destroyed {
            terrain, spread, ..
        } => {
            assert_eq!(*terrain, Terrain::Void);
            assert_eq!(
                spread.iter().map(|fire| fire.tile()).collect::<Vec<_>>(),
                vec![tile(1, 0), tile(0, 1)]
            );
            assert!(spread.iter().all(|fire| fire.effect() == Effect::fire(0)));
        }
        other => panic!("expected destruction, got {other:?}"),
    }
    assert_eq!(cells, original_cells);
}

#[test]
fn fighter_pursuit_uses_lowest_arsonist_id_and_neighbor_order() {
    let fighter = actor(1, ActorKind::Fighter, TileId::origin());
    let actors = [
        actor(9, ActorKind::Arsonist, tile(2, -1)),
        actor(3, ActorKind::Arsonist, tile(2, -1)),
    ];
    let original_actors = actors;

    let state = evaluate(FighterDefinition::new(fighter, &actors), 8);
    let result = state.payload();

    assert_eq!(result.selected_arsonist(), Some(actor_id(3)));
    assert_eq!(result.actor().tile(), tile(1, 0));
    assert_eq!(result.removed_arsonist(), None);
    assert_eq!(actors, original_actors);
}

#[test]
fn fighter_collision_returns_the_removed_arsonist_without_mutating_actors() {
    let fighter = actor(1, ActorKind::Fighter, TileId::origin());
    let actors = [actor(4, ActorKind::Arsonist, tile(1, 0))];
    let original_actors = actors;

    let state = evaluate(FighterDefinition::new(fighter, &actors), 9);

    assert_eq!(state.payload().actor().tile(), tile(1, 0));
    assert_eq!(state.payload().removed_arsonist(), Some(actor_id(4)));
    assert_eq!(actors, original_actors);
}

#[test]
fn arborist_converts_on_tick_three_and_stays_complete() {
    let arborist = actor(7, ActorKind::Arborist, tile(-2, 1));

    for age in 0..=4 {
        let state = evaluate(
            ArboristDefinition::new(arborist, Terrain::Wheat, age),
            age as i64,
        );
        let result = state.payload();

        assert_eq!(result.actor(), arborist);
        assert_eq!(result.game_tick_index(), age as i64);
        assert_eq!(result.converted(), age >= 3);
        assert_eq!(
            result.terrain(),
            if age >= 3 {
                Terrain::Forest
            } else {
                Terrain::Wheat
            }
        );
        assert_eq!(result.conversion_age_in_game_ticks(), age.min(3));
    }
}
