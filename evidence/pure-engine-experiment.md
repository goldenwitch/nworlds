# Pure Engine Experiment

This is historical evidence consumed by `game-surface.vine`. It tests
whether the existing direct-query path is understandable and semantically
independent for one concrete behavior. It does not claim that every game rule
has the same cost or that arbitrary Rust code is pure.

## Selected Regime

The workload is the existing Caravan moving Forester fixture:

- context: Caravan radius-5 reference context;
- facts: `CreateSaucer` and one Forester spawn;
- behavior: the existing indexed Forester trajectory and derived resource path;
- samples: logical time zero, game tick 100,000, game tick 1,000,000, reverse
  sampling, and repeated sampling;
- branch scope: the existing actual/counterfactual/corrected presentation tests;
- developer path: existing `IndexedQuery`/`Worldline` API composed through
  `GameSurface`, with no new DSL or game semantics.

The chosen experiment is deliberately small. It measures one concrete moving
behavior before any generic trajectory or rule DSL is proposed.

## Correctness Evidence

Existing tests provide the semantic checks:

- `projection_samples_a_long_moving_trajectory_without_tick_replay` samples a
  moving trajectory at 100,000 and 1,000,000 game ticks, then repeats the later
  query after an earlier query and compares the complete result.
- `projection_samples_a_long_stationary_trajectory_without_tick_replay`
  covers the corresponding long-horizon stationary case.
- `index_results_are_independent_of_query_order_and_source_storage` covers the
  reusable index query-order boundary.
- `external_consumer_queries_an_immutable_branch_without_query_history`
  covers parent/child isolation and reverse sampling for a generic consumer.
- `engine-surface` branch-session tests cover value-producing publication,
  parent isolation, revision checks, preview immutability, and discard recovery.

The direct-query property under test is:

```text
same immutable worldline + same LogicalTime -> same GameState
```

The order of earlier queries is not an input. A presentation `Tau` sample is
also downstream and does not alter the logical query.

## Measured Conditions

The checked-in benchmark report was produced with:

```text
cargo run --release --manifest-path crates/engine-benchmarks/Cargo.toml -- --iterations 10000 --warmup 1000 --report evidence/benchmarks/anchor-report.json
```

Conditions:

- release profile;
- `Instant` timing;
- 10,000 measured iterations;
- 1,000 warmup iterations;
- moving Forester horizon: 1,000,000 game ticks;
- seeded non-monotonic scrub: `[30, 0, 20, 10, 25, 5, 30, 15]`;
- no cache or optimization added by the benchmark crate.

Recorded mean nanoseconds per query:

| Workload | Journal facts | Mean ns/query |
| --- | ---: | ---: |
| empty at 7 ms | 0 | 1,048.60 |
| authored at 10 ms | 2 | 17,356.42 |
| seeded at game tick 30 | 7 | 60,292.55 |
| hand-authored behavior at game tick 4 | 10 | 44,266.01 |
| moving Forester at game tick 1,000,000 | 2 | 37,675.09 |

The non-monotonic seeded scrub recorded 43,841.24 ns/query averaged over eight
queries per sample. These are repository-local observations, not a production
latency budget or a cross-machine comparison.

## Developer-Effort Observation

The completed immutable-surface slice removes these application obligations:

- retaining a mutable `JournalWriter` beside the published world;
- refreshing a cached `GameState` after control-time changes;
- deciding whether a failed append mutated the source;
- manually rebuilding a child while preserving branch metadata.

The game still owns its context, fact vocabulary, query body, interaction
admission policy, camera, controls, and renderer. The API does not make those
meanings automatic.

## Open Residuals

- The measured trajectory uses the existing Caravan reference evaluator; it is
  not yet a new external developer implementation from a blank package.
- Publication copies journal history. No constant-time publication claim is
  made.
- Query cost is measured for fixed small fixtures and one long horizon; scaling
  with fact count, actor count, and consequence depth remains open.
- Query and renderer bodies remain trusted Rust. The API structurally limits
  world mutation but cannot prove arbitrary callback purity.

The next decision is whether this evidence is enough to select a second,
more composition-heavy behavior, or whether the measured costs bind an
optimization question first.
