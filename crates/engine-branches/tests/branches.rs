use caravan_domain::{ActorId, ActorKind, GameJournalEntry, Terrain, TileId};
use engine_branches::{Branch, BranchError, BranchKind, Worldline};
use engine_journal::{Journal, JournalWriter};
use engine_sdk::{Context, LogicalTime};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Definitions {
    marker: u8,
}

fn time(ticks: i64) -> LogicalTime {
    LogicalTime::from_ticks(ticks)
}

fn journal(entries: &[(i64, GameJournalEntry)]) -> Journal {
    let mut writer = JournalWriter::new();

    for (ticks, payload) in entries {
        writer
            .advance_to(time(*ticks))
            .expect("test journals use nondecreasing nonnegative times");
        writer.record(*payload);
    }

    writer.finish()
}

fn set_terrain(ticks: i64, terrain: Terrain) -> (i64, GameJournalEntry) {
    (
        ticks,
        GameJournalEntry::SetTerrain {
            tile: TileId::origin(),
            terrain,
        },
    )
}

fn expected_set_terrain(ticks: i64, terrain: Terrain) -> (LogicalTime, GameJournalEntry) {
    let (ticks, payload) = set_terrain(ticks, terrain);
    (time(ticks), payload)
}

fn spawn(ticks: i64, id: u64) -> (i64, GameJournalEntry) {
    (
        ticks,
        GameJournalEntry::SpawnActor {
            id: ActorId::new(id).expect("test actor IDs are positive"),
            kind: ActorKind::Farmer,
            tile: TileId::origin(),
        },
    )
}

fn expected_spawn(ticks: i64, id: u64) -> (LogicalTime, GameJournalEntry) {
    let (ticks, payload) = spawn(ticks, id);
    (time(ticks), payload)
}

fn entries(branch: &Branch<Definitions>) -> Vec<(LogicalTime, GameJournalEntry)> {
    branch
        .journal()
        .iter()
        .map(|entry| (entry.logical_time(), *entry.payload()))
        .collect()
}

#[test]
fn worldline_alias_and_opaque_context_are_query_facing() {
    let worldline: Worldline<Definitions> = Branch::new(
        Context::new(Definitions { marker: 7 }),
        journal(&[(0, GameJournalEntry::create_saucer())]),
    );

    assert!(worldline.is_actual());
    assert_eq!(worldline.kind(), BranchKind::Actual);
    assert_eq!(worldline.context_payload().marker, 7);
    assert_eq!(worldline.fork_boundary(), None);
}

#[test]
fn counterfactual_keeps_every_parent_entry_at_the_inclusive_boundary() {
    let parent = Branch::new(
        Context::new(Definitions { marker: 1 }),
        journal(&[
            (0, GameJournalEntry::create_saucer()),
            set_terrain(5, Terrain::Wheat),
            set_terrain(5, Terrain::Forest),
            set_terrain(10, Terrain::Wheat),
        ]),
    );
    let suffix = journal(&[spawn(7, 1)]);

    let child = parent
        .counterfactual(time(5), &suffix)
        .expect("strict counterfactual suffix is valid");

    assert_eq!(child.kind(), BranchKind::Counterfactual);
    assert_eq!(child.fork_boundary(), Some(time(5)));
    assert_eq!(
        entries(&child),
        vec![
            (time(0), GameJournalEntry::create_saucer()),
            expected_set_terrain(5, Terrain::Wheat),
            expected_set_terrain(5, Terrain::Forest),
            expected_spawn(7, 1),
        ]
    );
}

#[test]
fn corrected_suffix_replaces_parent_tail_without_rewriting_parent() {
    let parent = Branch::new(
        Context::new(Definitions { marker: 2 }),
        journal(&[
            (0, GameJournalEntry::create_saucer()),
            set_terrain(5, Terrain::Wheat),
            set_terrain(10, Terrain::Forest),
        ]),
    );
    let original_parent_entries = entries(&parent);
    let replacement = journal(&[spawn(6, 2)]);

    let corrected = parent
        .corrected_suffix(time(5), &replacement)
        .expect("corrected suffix is valid");

    assert_eq!(corrected.kind(), BranchKind::Corrected);
    assert_eq!(corrected.fork_boundary(), Some(time(5)));
    assert_eq!(
        entries(&corrected),
        vec![
            (time(0), GameJournalEntry::create_saucer()),
            expected_set_terrain(5, Terrain::Wheat),
            expected_spawn(6, 2),
        ]
    );
    assert_eq!(entries(&parent), original_parent_entries);
    assert_eq!(parent.kind(), BranchKind::Actual);
    assert_eq!(parent.fork_boundary(), None);
}

#[test]
fn nested_branches_use_the_child_snapshot_and_retain_the_new_boundary() {
    let parent = Branch::new(
        Context::new(Definitions { marker: 3 }),
        journal(&[
            (0, GameJournalEntry::create_saucer()),
            set_terrain(5, Terrain::Wheat),
        ]),
    );
    let first_suffix = journal(&[spawn(7, 3)]);
    let first_child = parent
        .counterfactual(time(5), &first_suffix)
        .expect("first branch is valid");
    let second_suffix = journal(&[set_terrain(9, Terrain::Forest)]);

    let nested = first_child
        .corrected_suffix(time(7), &second_suffix)
        .expect("nested branch is valid");

    assert_eq!(nested.fork_boundary(), Some(time(7)));
    assert_eq!(nested.kind(), BranchKind::Corrected);
    assert_eq!(
        entries(&nested),
        vec![
            (time(0), GameJournalEntry::create_saucer()),
            expected_set_terrain(5, Terrain::Wheat),
            expected_spawn(7, 3),
            expected_set_terrain(9, Terrain::Forest),
        ]
    );
    assert_eq!(
        entries(&first_child),
        vec![
            (time(0), GameJournalEntry::create_saucer()),
            expected_set_terrain(5, Terrain::Wheat),
            expected_spawn(7, 3),
        ]
    );
    assert_eq!(parent.journal().len(), 2);
}

#[test]
fn suffix_entries_at_or_before_boundary_are_rejected() {
    let parent = Branch::new(
        Context::new(Definitions { marker: 4 }),
        journal(&[(0, GameJournalEntry::create_saucer())]),
    );
    let suffix = journal(&[set_terrain(5, Terrain::Wheat)]);

    let error = parent
        .counterfactual(time(5), &suffix)
        .expect_err("the suffix must be strictly after the inclusive prefix");

    assert_eq!(
        error,
        BranchError::SuffixNotAfterFork {
            fork_boundary: time(5),
            entry_time: time(5),
        }
    );
    assert_eq!(parent.journal().len(), 1);
}
