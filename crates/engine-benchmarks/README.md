# Engine benchmarks

This is the non-published benchmark package for the fixed Caravan anchor
traces. It is a member of the root workspace and can also be invoked directly
with its manifest path.

## Reproduce

Run the package checks:

```text
cargo test --manifest-path crates/engine-benchmarks/Cargo.toml
cargo fmt --manifest-path crates/engine-benchmarks/Cargo.toml -- --check
cargo clippy --manifest-path crates/engine-benchmarks/Cargo.toml --all-targets -- -D warnings
```

Generate the checked-in report from a release build:

```text
cargo run --release --manifest-path crates/engine-benchmarks/Cargo.toml -- --iterations 10000 --warmup 1000 --report evidence/benchmarks/anchor-report.json
```

The command uses `std::time::Instant`, warms each measurement independently,
then records minimum, mean, and maximum elapsed nanoseconds per operation. The
operation counts, seed, journal horizon, scrub order, and presentation order
are written into the JSON report. Run it from the repository root so the
relative report path has the documented location.

## Fixed workloads

- `empty`: an empty journal.
- `authored`: the demo's `CreateSaucer@0` and `SpawnActor(Farmer)@10` trace.
- `seeded-cafe-horizon-20`: seed `0xCAFE`, horizon 20, seven journal entries.
- `hand-authored-behavior`: the seeded crate's deterministic behavior fixture.
- `authored-branch-family`: the demo's actual, counterfactual, and corrected
  branches, for a total of three immutable branch values.

Direct query timings sample one fixed logical time on each trace. Scrub timing
uses `[30, 0, 20, 10, 25, 5, 30, 15]` on the seeded worldline. Frame timing
uses `engine_presentation::present` with explicit logical-time and tau values
`[0, 5, 10, 5, 2, 10]` on each branch.

No cache, index, evaluator optimization, semantic change, or production-file
edit is part of this package. The measurements are reference-path observations;
their wall-clock values depend on the host and build profile recorded in the
report.