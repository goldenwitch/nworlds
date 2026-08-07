#![forbid(unsafe_code)]

use std::{hint::black_box, time::Instant};

use caravan_domain::{ActorId, ActorKind, GameJournalEntry, TileId};
use caravan_reference::{actual, state, try_state, ReferenceWorldline, Snapshot};
use caravan_seeded::{generate_spawn_journal, hand_authored_behavior_fixture};
use engine_branches::BranchKind;
use engine_journal::{Journal, JournalWriter};
use engine_presentation::{present, Renderer};
use engine_sdk::GameState;
use engine_time::{LogicalTime, Tau};

pub const DEFAULT_ITERATIONS: usize = 10_000;
pub const DEFAULT_WARMUP_ITERATIONS: usize = 1_000;
pub const SEEDED_TRACE_SEED: u64 = 0xCAFE;
pub const SEEDED_TRACE_HORIZON: u64 = 20;
pub const SCRUB_TRACE_TICKS: [i64; 8] = [30, 0, 20, 10, 25, 5, 30, 15];
pub const FRAME_TAU_TICKS: [i64; 6] = [0, 5, 10, 5, 2, 10];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BenchmarkConfig {
    pub iterations: usize,
    pub warmup_iterations: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: DEFAULT_ITERATIONS,
            warmup_iterations: DEFAULT_WARMUP_ITERATIONS,
        }
    }
}

pub struct FixedTraces {
    pub empty: ReferenceWorldline,
    pub authored: ReferenceWorldline,
    pub seeded: ReferenceWorldline,
    pub behavior: ReferenceWorldline,
    pub branches: BranchTraces,
}

impl FixedTraces {
    pub fn new() -> Self {
        let authored = actual(authored_journal());

        Self {
            empty: actual(Journal::empty()),
            branches: BranchTraces::new(&authored),
            authored,
            seeded: actual(generate_spawn_journal(
                SEEDED_TRACE_SEED,
                SEEDED_TRACE_HORIZON,
            )),
            behavior: actual(hand_authored_behavior_fixture()),
        }
    }
}

impl Default for FixedTraces {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BranchTraces {
    pub actual: ReferenceWorldline,
    pub counterfactual: ReferenceWorldline,
    pub corrected: ReferenceWorldline,
}

impl BranchTraces {
    fn new(parent: &ReferenceWorldline) -> Self {
        let alternate = journal([(7, spawn(2, ActorKind::Forester, TileId::origin()))]);
        let replacement = journal([(6, spawn(3, ActorKind::Arborist, tile(0, 1)))]);

        Self {
            actual: parent.clone(),
            counterfactual: parent
                .counterfactual(time(5), &alternate)
                .expect("the fixed counterfactual suffix follows its fork"),
            corrected: parent
                .corrected_suffix(time(5), &replacement)
                .expect("the fixed corrected suffix follows its fork"),
        }
    }

    pub fn count(&self) -> usize {
        3
    }

    pub fn iter(&self) -> impl Iterator<Item = &ReferenceWorldline> {
        [&self.actual, &self.counterfactual, &self.corrected].into_iter()
    }

    pub fn kinds(&self) -> [BranchKind; 3] {
        [
            self.actual.kind(),
            self.counterfactual.kind(),
            self.corrected.kind(),
        ]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixtureSummary {
    pub name: &'static str,
    pub journal_length: usize,
    pub branch_count: usize,
    pub branch_journal_lengths: &'static [usize],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timing {
    pub samples: usize,
    pub total_nanos: u128,
    pub min_nanos: u128,
    pub max_nanos: u128,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NamedTiming {
    pub name: &'static str,
    pub operations_per_sample: usize,
    pub timing: Timing,
}

impl NamedTiming {
    fn new(name: &'static str, operations_per_sample: usize, timing: Timing) -> Self {
        Self {
            name,
            operations_per_sample,
            timing,
        }
    }

    pub fn mean_nanos_per_operation(&self) -> f64 {
        self.timing.total_nanos as f64
            / self.timing.samples as f64
            / self.operations_per_sample as f64
    }

    pub fn min_nanos_per_operation(&self) -> f64 {
        self.timing.min_nanos as f64 / self.operations_per_sample as f64
    }

    pub fn max_nanos_per_operation(&self) -> f64 {
        self.timing.max_nanos as f64 / self.operations_per_sample as f64
    }
}

pub struct BenchmarkReport {
    pub config: BenchmarkConfig,
    pub profile: &'static str,
    pub fixtures: Vec<FixtureSummary>,
    pub direct_reference_queries: Vec<NamedTiming>,
    pub scrub_query_latency: NamedTiming,
    pub frame_production: Vec<NamedTiming>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TraceRenderValue {
    pub sampled_time: i64,
    pub tau: i64,
    pub actor_ids: Vec<u64>,
    pub wheat: u64,
    pub wood: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TraceRenderer;

impl Renderer<Snapshot> for TraceRenderer {
    type Output = TraceRenderValue;

    fn render(&self, state: &GameState<Snapshot>, tau: Tau) -> Self::Output {
        TraceRenderValue {
            sampled_time: state.logical_time().ticks(),
            tau: tau.ticks(),
            actor_ids: state
                .payload()
                .actors()
                .iter()
                .map(|actor| actor.id().get())
                .collect(),
            wheat: state.payload().resources().wheat(),
            wood: state.payload().resources().wood(),
        }
    }
}

pub fn run(config: BenchmarkConfig) -> BenchmarkReport {
    assert!(
        config.iterations > 0,
        "benchmark iterations must be positive"
    );

    let traces = FixedTraces::new();
    let direct_reference_queries = vec![
        named_query("empty@7", &traces.empty, 7, config),
        named_query("authored@10", &traces.authored, 10, config),
        named_query("seeded@30", &traces.seeded, 30, config),
        named_query("behavior@4", &traces.behavior, 4, config),
    ];

    let scrub_query_latency = NamedTiming::new(
        "seeded-non-monotonic-scrub",
        SCRUB_TRACE_TICKS.len(),
        measure(config, || {
            for ticks in SCRUB_TRACE_TICKS {
                std::hint::black_box(state(&traces.seeded, time(ticks)));
            }
        }),
    );

    let renderer = TraceRenderer;
    let frame_production = vec![
        named_frames("actual", &traces.branches.actual, &renderer, config),
        named_frames(
            "counterfactual",
            &traces.branches.counterfactual,
            &renderer,
            config,
        ),
        named_frames("corrected", &traces.branches.corrected, &renderer, config),
    ];

    BenchmarkReport {
        config,
        profile: build_profile(),
        fixtures: vec![
            FixtureSummary {
                name: "empty",
                journal_length: traces.empty.journal().len(),
                branch_count: 1,
                branch_journal_lengths: &[0],
            },
            FixtureSummary {
                name: "authored",
                journal_length: traces.authored.journal().len(),
                branch_count: 1,
                branch_journal_lengths: &[2],
            },
            FixtureSummary {
                name: "seeded-cafe-horizon-20",
                journal_length: traces.seeded.journal().len(),
                branch_count: 1,
                branch_journal_lengths: &[7],
            },
            FixtureSummary {
                name: "hand-authored-behavior",
                journal_length: traces.behavior.journal().len(),
                branch_count: 1,
                branch_journal_lengths: &[10],
            },
            FixtureSummary {
                name: "authored-branch-family",
                journal_length: traces.branches.actual.journal().len(),
                branch_count: traces.branches.count(),
                branch_journal_lengths: &[2, 2, 2],
            },
        ],
        direct_reference_queries,
        scrub_query_latency,
        frame_production,
    }
}

pub fn render_json(report: &BenchmarkReport, command: &str) -> String {
    let mut output = String::new();
    output.push_str("{\n");
    output.push_str("  \"schema\": \"caravan-benchmarks-v1\",\n");
    output.push_str(&format!("  \"command\": \"{}\",\n", escape_json(command)));
    output.push_str("  \"conditions\": {\n");
    output.push_str(&format!(
        "    \"profile\": \"{}\",\n    \"timer\": \"std::time::Instant\",\n    \"iterations\": {},\n    \"warmup_iterations\": {},\n    \"seed\": \"0xCAFE\",\n    \"seeded_horizon_game_ticks\": {},\n    \"cache_policy\": \"No cache or optimization is added by this crate\",\n    \"scrub_ticks\": {},\n    \"frame_tau_ticks\": {}\n",
        report.profile,
        report.config.iterations,
        report.config.warmup_iterations,
        SEEDED_TRACE_HORIZON,
        format_i64_array(&SCRUB_TRACE_TICKS),
        format_i64_array(&FRAME_TAU_TICKS),
    ));
    output.push_str("  },\n  \"fixtures\": [\n");
    for (index, fixture) in report.fixtures.iter().enumerate() {
        if index > 0 {
            output.push_str(",\n");
        }
        output.push_str(&format!(
            "    {{\"name\":\"{}\",\"journal_length\":{},\"branch_count\":{},\"branch_journal_lengths\":{}}}",
            fixture.name,
            fixture.journal_length,
            fixture.branch_count,
            format_usize_array(fixture.branch_journal_lengths),
        ));
    }
    output.push_str("\n  ],\n  \"results\": {\n");
    output.push_str("    \"direct_reference_query\": [\n");
    for (index, timing) in report.direct_reference_queries.iter().enumerate() {
        if index > 0 {
            output.push_str(",\n");
        }
        output.push_str(&format_named_timing(timing));
    }
    output.push_str("\n    ],\n    \"scrub_query_latency\": ");
    output.push_str(&format_named_timing(&report.scrub_query_latency));
    output.push_str(",\n    \"frame_production\": [\n");
    for (index, timing) in report.frame_production.iter().enumerate() {
        if index > 0 {
            output.push_str(",\n");
        }
        output.push_str(&format_named_timing(timing));
    }
    output.push_str("\n    ]\n  }\n}\n");
    output
}

fn named_query(
    name: &'static str,
    worldline: &ReferenceWorldline,
    ticks: i64,
    config: BenchmarkConfig,
) -> NamedTiming {
    NamedTiming::new(
        name,
        1,
        measure(config, || {
            std::hint::black_box(state(worldline, time(ticks)))
        }),
    )
}

fn named_frames(
    name: &'static str,
    worldline: &ReferenceWorldline,
    renderer: &TraceRenderer,
    config: BenchmarkConfig,
) -> NamedTiming {
    NamedTiming::new(
        name,
        FRAME_TAU_TICKS.len(),
        measure(config, || {
            for ticks in FRAME_TAU_TICKS {
                let state = try_state(worldline, time(ticks))
                    .expect("fixed benchmark sample should project");
                std::hint::black_box(present(&state, renderer, tau(ticks)));
            }
        }),
    )
}

fn measure<T, F>(config: BenchmarkConfig, mut operation: F) -> Timing
where
    F: FnMut() -> T,
{
    for _ in 0..config.warmup_iterations {
        black_box(operation());
    }

    let mut total_nanos = 0;
    let mut min_nanos = u128::MAX;
    let mut max_nanos = 0;

    for _ in 0..config.iterations {
        let started = Instant::now();
        black_box(operation());
        let elapsed_nanos = started.elapsed().as_nanos();
        total_nanos += elapsed_nanos;
        min_nanos = min_nanos.min(elapsed_nanos);
        max_nanos = max_nanos.max(elapsed_nanos);
    }

    Timing {
        samples: config.iterations,
        total_nanos,
        min_nanos,
        max_nanos,
    }
}

fn authored_journal() -> Journal {
    journal([
        (0, GameJournalEntry::create_saucer()),
        (10, spawn(1, ActorKind::Farmer, TileId::origin())),
    ])
}

fn journal(entries: impl IntoIterator<Item = (i64, GameJournalEntry)>) -> Journal {
    let mut writer = JournalWriter::new();
    for (ticks, payload) in entries {
        writer
            .advance_to(time(ticks))
            .expect("fixed benchmark journal timestamps are monotonic");
        writer.record(payload);
    }
    writer.finish()
}

fn spawn(id: u64, kind: ActorKind, tile: TileId) -> GameJournalEntry {
    GameJournalEntry::SpawnActor {
        id: ActorId::new(id).expect("fixed benchmark actor IDs are positive"),
        kind,
        tile,
    }
}

fn time(ticks: i64) -> LogicalTime {
    LogicalTime::from_ticks(ticks)
}

fn tau(ticks: i64) -> Tau {
    Tau::from_ticks(ticks)
}

fn tile(q: i32, r: i32) -> TileId {
    TileId::new(q, r).expect("fixed benchmark coordinates are inside the saucer")
}

fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

fn format_named_timing(timing: &NamedTiming) -> String {
    format!(
        "{{\"name\":\"{}\",\"samples\":{},\"operations_per_sample\":{},\"total_nanos\":{},\"min_ns_per_operation\":{:.2},\"mean_ns_per_operation\":{:.2},\"max_ns_per_operation\":{:.2}}}",
        timing.name,
        timing.timing.samples,
        timing.operations_per_sample,
        timing.timing.total_nanos,
        timing.min_nanos_per_operation(),
        timing.mean_nanos_per_operation(),
        timing.max_nanos_per_operation(),
    )
}

fn format_i64_array(values: &[i64]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn format_usize_array(values: &[usize]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::{
        render_json, run, BenchmarkConfig, FixedTraces, TraceRenderValue, FRAME_TAU_TICKS,
        SEEDED_TRACE_HORIZON, SEEDED_TRACE_SEED,
    };
    use caravan_domain::ActorKind;
    use engine_branches::BranchKind;

    #[test]
    fn fixed_traces_match_anchor_workload_shapes() {
        let traces = FixedTraces::new();

        assert_eq!(traces.empty.journal().len(), 0);
        assert_eq!(traces.authored.journal().len(), 2);
        assert_eq!(traces.seeded.journal().len(), 7);
        assert_eq!(traces.behavior.journal().len(), 10);
        assert_eq!(traces.branches.count(), 3);
        assert_eq!(
            traces.branches.kinds(),
            [
                BranchKind::Actual,
                BranchKind::Counterfactual,
                BranchKind::Corrected,
            ]
        );
        assert_eq!(SEEDED_TRACE_SEED, 0xCAFE);
        assert_eq!(SEEDED_TRACE_HORIZON, 20);
        assert_eq!(FRAME_TAU_TICKS.len(), 6);
    }

    #[test]
    fn branch_fixture_keeps_distinct_demo_results() {
        let traces = FixedTraces::new();
        let actual = super::state(&traces.branches.actual, super::time(10));
        let counterfactual = super::state(&traces.branches.counterfactual, super::time(10));
        let corrected = super::state(&traces.branches.corrected, super::time(10));

        assert_eq!(actual.payload().actors()[0].kind(), ActorKind::Farmer);
        assert_eq!(
            counterfactual.payload().actors()[0].kind(),
            ActorKind::Forester
        );
        assert_eq!(corrected.payload().actors()[0].kind(), ActorKind::Arborist);
    }

    #[test]
    fn renderer_output_remains_an_owned_frame_payload() {
        let traces = FixedTraces::new();
        let renderer = super::TraceRenderer;
        let state = super::try_state(&traces.branches.actual, super::time(10))
            .expect("benchmark renderer sample should project");
        let frame = super::present(&state, &renderer, super::tau(10));
        let expected = TraceRenderValue {
            sampled_time: 10,
            tau: 10,
            actor_ids: vec![1],
            wheat: 0,
            wood: 0,
        };

        assert_eq!(frame.payload(), &expected);
    }

    #[test]
    fn report_serialization_contains_conditions_and_result_groups() {
        let report = run(BenchmarkConfig {
            iterations: 2,
            warmup_iterations: 1,
        });
        let json = render_json(&report, "benchmark command");

        assert!(json.contains("caravan-benchmarks-v1"));
        assert!(json.contains("scrub_query_latency"));
        assert!(json.contains("frame_production"));
        assert!(json.contains("No cache or optimization is added by this crate"));
    }
}
