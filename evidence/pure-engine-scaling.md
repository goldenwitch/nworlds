# Pure Engine Scaling Packet

This packet records the first scaling measurements requested by
`game-surface.vine`. It measures existing reference rules and immutable branch
publication; it does not select an optimization or claim a
production performance budget.

## Reproduce

```text
cargo run --release --bin pure-scaling --manifest-path crates/engine-benchmarks/Cargo.toml -- --iterations 1000 --warmup 100 --report evidence/benchmarks/pure-scaling-report.json
```

Conditions:

- release profile;
- `std::time::Instant`;
- 1,000 measured iterations and 100 warmup iterations;
- one dimension varied per row;
- complete `GameState` result black-boxed;
- publication result black-boxed;
- no cache or derived index added by this binary.

The report is [pure-scaling-report.json](benchmarks/pure-scaling-report.json).

## Rows

| Dimension | Values | Observation |
| --- | --- | --- |
| Visible fact count | 2, 10, 25, 50 | Mean query time: 23,984.60, 55,535.30, 51,809.90, 76,718.70 ns/sample. This rises overall with the number of visible facts in this fixture, with run variance. |
| Actor count | 1, 4, 8, 16 | Mean query time: 28,366.90, 29,378.20, 29,540.10, 201,233.30 ns/sample. The 16-actor row has a large tail and needs a controlled follow-up before a conclusion. |
| Consequence-horizon proxy | game ticks 1, 2, 4, 10 | Mean query time: 109,183.10, 57,446.90, 100,625.90, 133,417.80 ns/sample. This is a temporal-horizon proxy over the existing hand-authored behavior fixture, not a clean causal-depth isolation. |
| Publication history copy | 0, 10, 100, 500 facts | Mean publication time: 508.00, 2,106.30, 771.60, 3,259.20 ns/sample. This measures the current snapshot-copying publication path; the small rows are noisy. |

The actor and consequence rows are observations, not verdicts. Their fixture
shape and variance are insufficient to authorize a new derived-index or rule
abstraction.

## Interpretation Boundary

The data supports these statements:

- semantic query independence is already tested separately;
- visible fact count and publication history length bind directly to measured
  work in the current implementation;
- the current small fixtures do not establish a production latency budget;
- actor-count and consequence-depth scaling need a better-controlled regime if
  either becomes a concrete requirement.

The data does not support:

- a constant-time query claim;
- a general performance promise for arbitrary games;
- a conclusion that a cache, index, or alternate evaluator is required; or
- a claim that the current causal-depth proxy isolates one rule-chain cost.

The next decision is human: keep the current boundary for the declared small
regime, narrow supported claims, or authorize a better-controlled scaling
experiment. Any optimization remains downstream of that ruling and must retain
immutable parent/branch semantics.
