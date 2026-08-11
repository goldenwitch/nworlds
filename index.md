# Caravan of Seasons

Caravan of Seasons is a Rust workspace for a deterministic, directly indexed
temporal game engine. Its reference query is:

```text
state(worldline, t_) -> game_state
```

Queries read an immutable worldline at any logical time. Presentation receives
the selected logical and presentation times and renders the resulting state;
it does not advance or mutate a hidden current state.

## Start Here

- [Initial specification](spec/initial.md) - vocabulary, invariants, journal
  semantics, lookahead, branches, and presentation.
- [Roadmap](roadmap.md) - settled constraints, the completed anchor, remaining
  work, and deferred decisions.
- [Cellular automata anchor](spec/cellular-automata-anchor.md) - the
  concrete radius-5, 91-tile fixture used by the implementation and tests.
- [Discontinuity projection](spec/discontinuity-projection.md) - the
  immutable breakpoint, piece-selection, and projection contract.
- [Build graph](build.vine) - the dependency-ordered implementation packets
  and their acceptance criteria.
- [Rendering plan](rendering.vine) - the focused backend-neutral rendering
  slice planned on top of the completed anchor.
- [Stage layer proposal](proposals/stage-layer.md) - the boundary between the
  canonical logical game experience and platform presentation plumbing.
- [Presentation host proposal](proposals/presentation-host.md) - the platform
  and execution responsibilities surrounding Stage.
- [Platform support matrix](proposals/platform-support-matrix.md) - adapter
  axes, target cells, and activation gates for host composition.
- [Host adapter wiring graph](host.vine) - the dependency-ordered plan for
  the in-memory host proof and later platform adapters.
- [Input and interaction boundary](proposals/input-and-interaction.md) - the
  abstract packet set, interaction-definition, and buffering boundary.
- [Caravan Orchestrator anchor](proposals/caravan-orchestrator-anchor.md) -
  the completed application checkpoint for exercising Stage and Orchestrator
  around the existing immutable engine seams.
- [Orchestrator build graph](orchestrator.vine) - the completed vertical slice
  and its acceptance evidence; future extraction remains conditional.

## Workspace Components

### Engine

These crates provide the generic temporal engine and its public boundaries.

| Crate | Description |
| --- | --- |
| [`engine-time`](crates/engine-time) | Distinct `LogicalTime` and `Tau` fixed-point time types, checked arithmetic, and tick conversions. |
| [`engine-sdk`](crates/engine-sdk) | Generic immutable envelopes for contexts, journals, worldlines, game states, and frames. |
| [`engine-journal`](crates/engine-journal) | Immutable journal storage and the journal-owned monotonic `JournalWriter`. |
| [`engine-branches`](crates/engine-branches) | Immutable actual, counterfactual, and corrected branch construction from journal prefixes and suffixes. |
| [`engine-index`](crates/engine-index) | Direct indexed-query kernel plus engine-neutral discontinuity breakpoints and half-open pieces. |
| [`engine-lookahead`](crates/engine-lookahead) | Future queries and read-only branch views using the same direct query path. |
| [`engine-presentation`](crates/engine-presentation) | State-plus-`Tau` render composition and frame values. |
| [`engine-persistence`](crates/engine-persistence) | Game-facing versioned worldline encoding, branch lineage, save/load, and deterministic replay; host byte transport remains separate. |
| [`engine-api`](crates/engine-api) | Game-facing facade that exposes the supported query, journal, branch, time, and domain APIs. |

### Caravan Domain

These crates define the concrete game fixture and its indexed rules.

| Crate | Description |
| --- | --- |
| [`caravan-domain`](crates/caravan-domain) | Radius-5 axial saucer geometry, tiles, terrain, actors, effects, resources, identifiers, and journal payloads. |
| [`caravan-vegetation`](crates/caravan-vegetation) | Indexed Farmer, Wheat, Forest, and Forester definitions, including movement and resource production. |
| [`caravan-hazards`](crates/caravan-hazards) | Indexed Arsonist, Fire, Fighter, and Arborist rules, including spread, destruction, collisions, and conversion. |
| [`caravan-seeded`](crates/caravan-seeded) | Deterministic seeded journal generation performed before evaluation. |
| [`caravan-reference`](crates/caravan-reference) | The reference `state(worldline, t_)` oracle, discontinuity index, piecewise projection, snapshots, and bounded parity baseline. |

### Executables and Validation

| Component | Description |
| --- | --- |
| [`caravan-demo`](crates/caravan-demo) | Runnable terminal demonstration of the anchor, arbitrary sampling, lookahead, branches, and presentation. |
| [`engine-benchmarks`](crates/engine-benchmarks) | Non-published release-build measurements for direct queries, scrubbing, branches, and frame production. |
| [`purity-tests`](crates/purity-tests) | Runtime and `trybuild` compiler-boundary tests for immutable, data-only authoritative APIs. |
| [`tests/conformance`](tests/conformance) | Separate workspace containing the executable conformance catalog and report generator. |

## Evidence and Reports

- [Conformance matrix](evidence/clause-to-test.md) - maps specification clauses
  to standalone conformance cases, root-workspace evidence, and explicit gaps.
- [Conformance report](evidence/conformance-report.json) - machine-readable
  results for the standalone catalog; root-workspace evidence is mapped in the
  conformance matrix.
- [Benchmark report](evidence/benchmarks/anchor-report.json) - conditions and
  measurements for the fixed anchor workloads.
- [Demo trace](crates/caravan-demo/snapshots/anchor-trace.txt) - checked-in
  deterministic output from the terminal demo.
- [Projection parity snapshot](crates/caravan-reference/snapshots/discontinuity-parity.json)
  - frozen observations and the scope of the historical-fold comparison.
- [Productization review](proposals/productization-review.md) - current
  product boundary and concerns intentionally deferred until requirements bind.

## Common Commands

Run these from the repository root:

```text
# Build and test the main workspace
cargo test --workspace

# Run the terminal anchor demo
cargo run --manifest-path crates/caravan-demo/Cargo.toml

# Run the independent conformance workspace
cargo test --manifest-path tests/conformance/Cargo.toml
cargo run --manifest-path tests/conformance/Cargo.toml -- --report evidence/conformance-report.json

# Run compiler-boundary tests
cargo test -p purity-tests

# Reproduce the checked-in benchmark report
cargo run --release --manifest-path crates/engine-benchmarks/Cargo.toml -- --iterations 10000 --warmup 1000 --report evidence/benchmarks/anchor-report.json
```

The `target/` directories contain generated build output and are not part of
the component map.