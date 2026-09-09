#![forbid(unsafe_code)]

use std::{env, fs, hint::black_box, path::PathBuf, time::Instant};

use caravan_domain::{ActorId, ActorKind, GameJournalEntry, Terrain, RADIUS_5_TILES};
use caravan_reference::{actual, state, Journal as CaravanJournal};
use caravan_seeded::hand_authored_behavior_fixture;
use engine_branches::Branch;
use engine_journal::{Journal, JournalWriter};
use engine_sdk::Context;
use engine_time::LogicalTime;

const DEFAULT_ITERATIONS: usize = 1_000;
const DEFAULT_WARMUP: usize = 100;
const FACT_COUNTS: [usize; 4] = [2, 10, 25, 50];
const ACTOR_COUNTS: [usize; 4] = [1, 4, 8, 16];
const CONSEQUENCE_TIMES: [usize; 4] = [1, 2, 4, 10];
const PUBLICATION_FACT_COUNTS: [usize; 4] = [0, 10, 100, 500];

#[derive(Clone, Copy, Debug)]
struct Timing {
    samples: usize,
    total_nanos: u128,
    min_nanos: u128,
    max_nanos: u128,
}

fn main() {
    let (iterations, warmup, report) = arguments();
    let rows = [
        scaling_rows("fact-count", FACT_COUNTS, |count| {
            let worldline = actual(fact_count_journal(count));
            measure(iterations, warmup, || {
                black_box(state(&worldline, LogicalTime::from_ticks(0)));
            })
        }),
        scaling_rows("actor-count", ACTOR_COUNTS, |count| {
            let worldline = actual(actor_count_journal(count));
            measure(iterations, warmup, || {
                black_box(state(&worldline, LogicalTime::from_ticks(0)));
            })
        }),
        scaling_rows("consequence-horizon-proxy", CONSEQUENCE_TIMES, |ticks| {
            let worldline = actual(hand_authored_behavior_fixture());
            measure(iterations, warmup, || {
                black_box(state(
                    &worldline,
                    LogicalTime::from_ticks(ticks as i64 * 1_000),
                ));
            })
        }),
        scaling_rows(
            "publication-history-copy",
            PUBLICATION_FACT_COUNTS,
            |count| {
                let world = publication_base(count);
                measure(iterations, warmup, || {
                    let _ =
                        black_box(world.append_at(LogicalTime::from_ticks(count as i64), [1_u8]));
                })
            },
        ),
    ]
    .concat();

    let mut output = String::new();
    output.push_str("{\n");
    output.push_str("  \"schema\": \"pure-engine-scaling-v1\",\n");
    output.push_str(&format!(
        "  \"conditions\": {{\"profile\":\"{}\",\"iterations\":{},\"warmup\":{},\"fact_counts\":{},\"actor_counts\":{},\"consequence_times\":{},\"publication_fact_counts\":{}}},\n",
        if cfg!(debug_assertions) { "debug" } else { "release" },
        iterations,
        warmup,
        array_usize(&FACT_COUNTS),
        array_usize(&ACTOR_COUNTS),
        array_usize(&CONSEQUENCE_TIMES),
        array_usize(&PUBLICATION_FACT_COUNTS),
    ));
    output.push_str("  \"rows\": [\n");
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            output.push_str(",\n");
        }
        output.push_str(&format!(
            "    {{\"dimension\":\"{}\",\"size\":{},\"samples\":{},\"mean_ns\":{:.2},\"min_ns\":{},\"max_ns\":{}}}",
            row.0,
            row.1,
            row.2.samples,
            row.2.total_nanos as f64 / row.2.samples as f64,
            row.2.min_nanos,
            row.2.max_nanos,
        ));
    }
    output.push_str("\n  ]\n}\n");

    if let Some(parent) = report.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent).expect("scaling report directory should exist");
    }
    fs::write(&report, output).expect("scaling report should be writable");
    println!("pure scaling report: {}", report.display());
}

fn scaling_rows<F>(
    dimension: &'static str,
    sizes: impl IntoIterator<Item = usize>,
    mut run: F,
) -> Vec<(&'static str, usize, Timing)>
where
    F: FnMut(usize) -> Timing,
{
    sizes
        .into_iter()
        .map(|size| (dimension, size, run(size)))
        .collect()
}

fn measure<F>(iterations: usize, warmup: usize, mut operation: F) -> Timing
where
    F: FnMut(),
{
    for _ in 0..warmup {
        operation();
    }
    let mut total_nanos = 0;
    let mut min_nanos = u128::MAX;
    let mut max_nanos = 0;
    for _ in 0..iterations {
        let start = Instant::now();
        operation();
        let elapsed = start.elapsed().as_nanos();
        total_nanos += elapsed;
        min_nanos = min_nanos.min(elapsed);
        max_nanos = max_nanos.max(elapsed);
    }
    Timing {
        samples: iterations,
        total_nanos,
        min_nanos,
        max_nanos,
    }
}

fn fact_count_journal(count: usize) -> CaravanJournal {
    let mut entries = vec![(0, GameJournalEntry::create_saucer())];
    for index in 0..count.saturating_sub(1) {
        entries.push((
            0,
            GameJournalEntry::SetTerrain {
                tile: RADIUS_5_TILES[index % RADIUS_5_TILES.len()],
                terrain: Terrain::Forest,
            },
        ));
    }
    journal(entries)
}

fn actor_count_journal(count: usize) -> CaravanJournal {
    let mut entries = vec![(0, GameJournalEntry::create_saucer())];
    for (index, &tile) in RADIUS_5_TILES
        .iter()
        .enumerate()
        .take(count.min(RADIUS_5_TILES.len()))
    {
        entries.push((
            0,
            GameJournalEntry::SpawnActor {
                id: ActorId::new((index + 1) as u64).expect("scaling actor ID is positive"),
                kind: ActorKind::Forester,
                tile,
            },
        ));
    }
    journal(entries)
}

fn journal(entries: impl IntoIterator<Item = (i64, GameJournalEntry)>) -> CaravanJournal {
    let mut writer = JournalWriter::new();
    for (ticks, payload) in entries {
        writer
            .advance_to(LogicalTime::from_ticks(ticks))
            .expect("scaling timestamps are monotonic");
        writer.record(payload);
    }
    writer.finish()
}

fn publication_base(count: usize) -> Branch<(), u8> {
    let mut world = Branch::new(Context::new(()), Journal::empty());
    for index in 0..count {
        world = world
            .append_at(LogicalTime::from_ticks(index as i64), [index as u8])
            .expect("scaling publication setup is monotonic");
    }
    world
}

fn arguments() -> (usize, usize, PathBuf) {
    let mut iterations = DEFAULT_ITERATIONS;
    let mut warmup = DEFAULT_WARMUP;
    let mut report = PathBuf::from("evidence/benchmarks/pure-scaling-report.json");
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iterations" => iterations = parse_positive(args.next()),
            "--warmup" => warmup = parse_positive(args.next()),
            "--report" => report = PathBuf::from(args.next().expect("--report needs a path")),
            _ => panic!("unknown argument: {arg}"),
        }
    }
    (iterations, warmup, report)
}

fn parse_positive(value: Option<String>) -> usize {
    let value = value.expect("numeric option needs a value");
    let parsed = value.parse().expect("numeric option must be an integer");
    assert!(parsed > 0, "numeric option must be positive");
    parsed
}

fn array_usize(values: &[usize]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}
