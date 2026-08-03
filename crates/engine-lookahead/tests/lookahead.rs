use caravan_domain::{ActorId, ActorKind, GameJournalEntry, Terrain, TileId};
use caravan_reference::actual;
use engine_branches::BranchKind;
use engine_journal::{Journal, JournalWriter};
use engine_lookahead::{branch_view, future, ViewKind};
use engine_time::LogicalTime;

fn time(ticks: i64) -> LogicalTime {
    LogicalTime::from_ticks(ticks)
}

fn actor_id(value: u64) -> ActorId {
    ActorId::new(value).expect("test actor IDs are positive")
}

fn journal(entries: &[(i64, GameJournalEntry)]) -> Journal {
    let mut writer = JournalWriter::new();
    for (ticks, entry) in entries {
        writer
            .advance_to(time(*ticks))
            .expect("test journal timestamps are monotonic");
        writer.record(*entry);
    }
    writer.finish()
}

fn spawn(id: u64, kind: ActorKind, tile: TileId) -> GameJournalEntry {
    GameJournalEntry::SpawnActor {
        id: actor_id(id),
        kind,
        tile,
    }
}

#[test]
fn earlier_lookahead_survives_later_authoring_on_another_branch() {
    let parent = actual(journal(&[(0, GameJournalEntry::create_saucer())]));
    let earlier = future(&parent, time(4));

    let later_suffix = journal(&[(
        8,
        GameJournalEntry::SetTerrain {
            tile: TileId::origin(),
            terrain: Terrain::Wheat,
        },
    )]);
    let counterfactual = parent
        .counterfactual(time(0), &later_suffix)
        .expect("suffix is after the fork boundary");

    assert_eq!(earlier, future(&parent, time(4)));
    assert_eq!(earlier.logical_time(), time(4));
    assert_eq!(
        future(&counterfactual, time(8))
            .payload()
            .resources()
            .wheat(),
        1
    );
    assert_eq!(future(&parent, time(8)).payload().resources().wheat(), 0);
}

#[test]
fn actual_counterfactual_and_corrected_views_use_the_same_query_path() {
    let parent = actual(journal(&[
        (0, GameJournalEntry::create_saucer()),
        (
            5,
            GameJournalEntry::SetTerrain {
                tile: TileId::origin(),
                terrain: Terrain::Wheat,
            },
        ),
        (10, spawn(1, ActorKind::Farmer, TileId::origin())),
    ]));
    let alternate = journal(&[(
        7,
        spawn(
            2,
            ActorKind::Forester,
            TileId::new(-1, 0).expect("tile is inside the saucer"),
        ),
    )]);
    let replacement = journal(&[(
        6,
        spawn(
            3,
            ActorKind::Arborist,
            TileId::new(0, 1).expect("tile is inside the saucer"),
        ),
    )]);
    let counterfactual = parent
        .counterfactual(time(5), &alternate)
        .expect("counterfactual suffix is after the fork boundary");
    let corrected = parent
        .corrected_suffix(time(5), &replacement)
        .expect("corrected suffix is after the fork boundary");

    let views = [
        branch_view(&parent),
        branch_view(&counterfactual),
        branch_view(&corrected),
    ];
    let states = views.map(|view| view.query(time(10)));

    assert_eq!(views[0].kind(), BranchKind::Actual);
    assert_eq!(views[1].kind(), BranchKind::Counterfactual);
    assert_eq!(views[2].kind(), BranchKind::Corrected);
    assert_eq!(views[0].kind(), ViewKind::Actual);
    assert_eq!(views[0].fork_boundary(), None);
    assert_eq!(views[1].fork_boundary(), Some(time(5)));
    assert_eq!(views[2].fork_boundary(), Some(time(5)));
    assert_eq!(states[0].payload().actors()[0].id(), actor_id(1));
    assert_eq!(states[1].payload().actors()[0].id(), actor_id(2));
    assert_eq!(states[2].payload().actors()[0].id(), actor_id(3));
    assert!(states.iter().all(|state| state.logical_time() == time(10)));
}

#[test]
fn arbitrary_future_times_are_direct_queries_without_generated_entries() {
    let worldline = actual(journal(&[(0, GameJournalEntry::create_saucer())]));

    let state = future(&worldline, time(1_000));

    assert_eq!(state.logical_time(), time(1_000));
    assert_eq!(state.payload().tick_index(), 1_000);
    assert_eq!(state.payload().tiles().len(), 91);
    assert!(state.payload().actors().is_empty());
}
