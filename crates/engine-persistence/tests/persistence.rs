use std::{fs, path::PathBuf};

use caravan_domain::{ActorId, ActorKind, GameJournalEntry, Terrain, TileId};
use caravan_reference::{actual, state};
use engine_branches::BranchKind;
use engine_journal::{Journal, JournalWriter};
use engine_persistence::{
    branch_lineage, decode, encode, load, replay, replay_bytes, save, BranchLineage,
    PersistenceError, FORMAT_VERSION,
};
use engine_sdk::LogicalTime;

fn time(ticks: i64) -> LogicalTime {
    LogicalTime::from_ticks(ticks)
}

fn tile(q: i32, r: i32) -> TileId {
    TileId::new(q, r).expect("test coordinate is inside the saucer")
}

fn journal(entries: impl IntoIterator<Item = (i64, GameJournalEntry)>) -> Journal {
    let mut writer = JournalWriter::new();
    for (ticks, payload) in entries {
        writer
            .advance_to(time(ticks))
            .expect("test timestamps are monotonic");
        writer.record(payload);
    }
    writer.finish()
}

fn spawn(id: u64, kind: ActorKind, tile: TileId) -> GameJournalEntry {
    GameJournalEntry::SpawnActor {
        id: ActorId::new(id).expect("test actor IDs are positive"),
        kind,
        tile,
    }
}

fn anchor_worldline() -> caravan_reference::ReferenceWorldline {
    actual(journal([
        (0, GameJournalEntry::create_saucer()),
        (
            0,
            GameJournalEntry::SetTerrain {
                tile: tile(1, 0),
                terrain: Terrain::Wheat,
            },
        ),
        (3, spawn(1, ActorKind::Forester, TileId::origin())),
        (10, spawn(2, ActorKind::Arsonist, tile(1, 0))),
    ]))
}

#[test]
fn anchor_journal_round_trips_and_encoding_is_deterministic() {
    let original = anchor_worldline();
    let encoded = encode(&original).expect("anchor record encodes");
    let decoded = decode(&encoded).expect("anchor record decodes");

    assert_eq!(decoded, original);
    assert_eq!(
        encode(&decoded).expect("decoded record re-encodes"),
        encoded
    );
    assert_eq!(decoded.journal().len(), 4);
    assert_eq!(decoded.context_payload().saucer_radius(), 5);
}

#[test]
fn counterfactual_and_corrected_lineage_round_trip() {
    let parent = actual(journal([
        (0, GameJournalEntry::create_saucer()),
        (
            5,
            GameJournalEntry::SetTerrain {
                tile: TileId::origin(),
                terrain: Terrain::Wheat,
            },
        ),
        (10, spawn(1, ActorKind::Farmer, tile(1, 0))),
    ]));
    let alternate = journal([(7, spawn(2, ActorKind::Forester, tile(-1, 0)))]);
    let replacement = journal([(6, spawn(3, ActorKind::Arborist, tile(0, 1)))]);
    let counterfactual = parent
        .counterfactual(time(5), &alternate)
        .expect("counterfactual suffix is after its boundary");
    let corrected = parent
        .corrected_suffix(time(5), &replacement)
        .expect("corrected suffix is after its boundary");

    for (branch, expected_kind, expected_actor) in [
        (&counterfactual, BranchKind::Counterfactual, 2),
        (&corrected, BranchKind::Corrected, 3),
    ] {
        let decoded =
            decode(&encode(branch).expect("child branch encodes")).expect("child branch decodes");
        assert_eq!(decoded, *branch);
        assert_eq!(decoded.kind(), expected_kind);
        assert_eq!(decoded.fork_boundary(), Some(time(5)));
        assert_eq!(decoded.journal().len(), 3);
        assert_eq!(
            state(&decoded, time(10)).payload().actors()[0].id().get(),
            expected_actor
        );
        assert_eq!(
            branch_lineage(&decoded),
            BranchLineage::new(expected_kind, Some(time(5)))
        );
    }

    assert_eq!(parent.kind(), BranchKind::Actual);
    assert_eq!(parent.journal().len(), 3);
}

#[test]
fn incompatible_versions_fail_explicitly() {
    let mut encoded = encode(&anchor_worldline()).expect("anchor record encodes");
    let incompatible = FORMAT_VERSION + 1;
    encoded[4..6].copy_from_slice(&incompatible.to_le_bytes());

    assert!(matches!(
        decode(&encoded),
        Err(PersistenceError::UnsupportedVersion {
            found,
            supported: FORMAT_VERSION,
        }) if found == incompatible
    ));
}

#[test]
fn unsupported_saucer_radius_fails_during_decode() {
    let worldline = actual(journal([(0, GameJournalEntry::create_saucer())]));
    let mut encoded = encode(&worldline).expect("anchor record encodes");

    encoded[28] = 4;

    assert!(matches!(
        decode(&encoded),
        Err(PersistenceError::InvalidValue {
            field: "journal saucer radius"
        })
    ));
}

#[test]
fn replay_preserves_direct_query_results_in_non_monotonic_order() {
    let worldline = anchor_worldline();
    let times = [time(10), time(2), time(4), time(10), time(-1)];
    let original = replay(&worldline, times);
    let restored = replay_bytes(&encode(&worldline).expect("anchor record encodes"), times)
        .expect("saved replay decodes");

    assert_eq!(restored, original);
    assert_eq!(restored[0].logical_time(), time(10));
    assert_eq!(restored[1].logical_time(), time(2));
    assert_eq!(restored[3], restored[0]);
}

#[test]
fn save_and_load_use_the_same_record_without_frame_history() {
    let worldline = anchor_worldline();
    let path = unique_test_path();

    save(&worldline, &path).expect("worldline saves");
    let loaded = load(&path).expect("worldline loads");
    let _ = fs::remove_file(&path);

    assert_eq!(loaded, worldline);
    assert_eq!(
        replay(&loaded, [time(0), time(10)]),
        replay(&worldline, [time(0), time(10)])
    );
}

#[test]
fn encoding_rejects_unsupported_saucer_radius_before_writing_bytes() {
    let worldline = actual(journal([(0, GameJournalEntry::CreateSaucer { radius: 4 })]));

    assert!(matches!(
        encode(&worldline),
        Err(PersistenceError::InvalidValue {
            field: "journal saucer radius"
        })
    ));
}

fn unique_test_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "caravan-engine-persistence-{}.cspf",
        std::process::id()
    ))
}
