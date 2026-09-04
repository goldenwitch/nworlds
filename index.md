# nworlds

nworlds is a library-first Rust workspace for a deterministic, directly
indexed temporal game engine. The reusable temporal library and target-neutral
host are the product boundary. Caravan of Seasons is the contained reference
game and sample consumer used to make the engine model and evidence concrete.
This repository is a research implementation, not a production game or
released engine SDK.

Its reference query is:

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
- [Library boundary graph](library-boundary.vine) - the library-first contract,
  ownership classification, and contamination-remediation gates.
- [Redundancy pass](redundancy.vine) - the all-up inventory and canonical-owner
  plan for collapsing duplicate plans, prose, code, and evidence.
- [Library contract](proposals/library-contract.md) - the supported temporal
  surface, ownership classes, and dependency-direction rules.
- [Rendering plan](rendering.vine) - the completed backend-neutral rendering
  projection and its target-facing host boundary.
- [Stage layer proposal](proposals/stage-layer.md) - the boundary between the
  canonical logical game experience and platform presentation plumbing.
- [Presentation host proposal](proposals/presentation-host.md) - the platform
  and execution responsibilities surrounding Stage.
- [Target factory proposal](proposals/target-factory.md) - the target-neutral
  developer path and host-owned artifact minting contract.
- [Target factory plan](target-factory.vine) - the ordered design work for
  HostContract, RenderBatch, resolution, CLI, and artifact minting.
- [Platform support matrix](proposals/platform-support-matrix.md) - adapter
  axes, target cells, and activation gates for host composition.
- [Host adapter wiring graph](host.vine) - the first host proof composition;
  target minting is owned by the target factory proposal.
- [Target support graph](support.vine) - the closed desktop target universe,
  explicit gaps, and native or appropriate CI plan.
- [Demo gameplay plan](gameplay.vine) - the design-derived inventory and
  ordered plan for the remaining player-facing demo loop.
- [Voxel sample case study](proposals/voxel-sample.md) - the engine features
  used by the independent voxel sample and its ownership boundary.
- [Input and interaction boundary](proposals/input-and-interaction.md) - the
  semantic batch, interaction-definition, and buffering boundary.
- [Transport and journal layer](proposals/transport-and-journal.md) - the
  reusable identity, ordering, transport-envelope, membership-view, and
  immutable journal-operation pattern for local, replay, and future network
  input.
- [Transport and journal graph](transport.vine) - the completed contract,
  ordered-batch, journal-bridge, and cross-source evidence packets.
- [Caravan Orchestrator anchor](proposals/caravan-orchestrator-anchor.md) -
  the completed application checkpoint for exercising Stage and Orchestrator
  around the existing immutable engine seams.
- [Orchestrator build graph](orchestrator.vine) - the completed vertical slice,
  host-facing composition, and acceptance evidence; future extraction remains
  conditional.

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
| [`engine-presentation`](crates/engine-presentation) | State-plus-`Tau` render composition and frame values. |
| [`engine-api`](crates/engine-api) | Generic facade for the supported temporal query, journal, branch, time, and presentation APIs. |
| [`nworlds-host`](crates/nworlds-host) | Target-neutral `GamePackage` contract, independent host ports, and generic package/port composition. |

### Caravan Domain

These crates define the concrete game fixture and its indexed rules.

| Crate | Description |
| --- | --- |
| [`caravan-domain`](crates/caravan-domain) | Radius-5 axial saucer geometry, tiles, terrain, actors, effects, resources, identifiers, and journal payloads. |
| [`caravan-vegetation`](crates/caravan-vegetation) | Indexed Farmer, Wheat, Forest, and Forester definitions, including movement and resource production. |
| [`caravan-hazards`](crates/caravan-hazards) | Indexed Arsonist, Fire, Fighter, and Arborist rules, including spread, destruction, collisions, and conversion. |
| [`caravan-seeded`](crates/caravan-seeded) | Deterministic seeded journal generation performed before evaluation. |
| [`caravan-reference`](crates/caravan-reference) | The reference `state(worldline, t_)` oracle, discontinuity index, piecewise projection, snapshots, and bounded parity baseline. |
| [`caravan-persistence`](crates/caravan-persistence) | Caravan-specific versioned worldline encoding, branch lineage, save/load, and deterministic replay. |

### Executables and Validation

| Component | Description |
| --- | --- |
| [`caravan-demo`](crates/caravan-demo) | Runnable terminal demonstration; its [colocated README](crates/caravan-demo/README.md) presents the engine integration example and file ownership. |
| [`nworlds-desktop`](crates/nworlds-desktop/Cargo.toml) | Target-local desktop adapter mapped over `nworlds-host`; it contains no package-owned state construction. |
| [`voxel-sample`](crates/voxel-sample/Cargo.toml) | Independent voxel cottage consumer; its [colocated README](crates/voxel-sample/README.md) is the practical guide, and [`engine_integration.rs`](crates/voxel-sample/src/engine_integration.rs) demonstrates generic state, journal, query, branch, and presentation usage with sample-defined types. |
| [`engine-benchmarks`](crates/engine-benchmarks) | Non-published release-build measurements for direct queries, scrubbing, branches, and frame production. |
| [`purity-tests`](crates/purity-tests) | Runtime and `trybuild` compiler-boundary tests for immutable, data-only authoritative APIs. |
| [`tests/conformance`](tests/conformance) | Separate workspace containing the executable conformance catalog and report generator. |

## Evidence and Reports

- [Conformance matrix](evidence/clause-to-test.md) - maps specification clauses
  to standalone conformance cases, root-workspace evidence, and explicit gaps.
- [Library boundary evidence](evidence/clause-to-test.md#library-boundary-evidence)
  - maps public-consumer, dependency, host, purity, and library-only build proof.
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

Repository-maintenance commands, not the public game-development interface:

```text
# Build and test the main workspace
cargo test --workspace

# Run the independent conformance workspace
cargo test --manifest-path tests/conformance/Cargo.toml
cargo run --manifest-path tests/conformance/Cargo.toml -- --report evidence/conformance-report.json

# Run compiler-boundary tests
cargo test -p purity-tests

# Check only the reusable temporal and host packages
cargo check -p engine-time -p engine-sdk -p engine-journal -p engine-branches -p engine-index -p engine-presentation -p engine-api -p nworlds-host --locked

# Run the independent voxel sample
cargo run --manifest-path crates/voxel-sample/Cargo.toml

# Reproduce the checked-in benchmark report
cargo run --release --manifest-path crates/engine-benchmarks/Cargo.toml -- --iterations 10000 --warmup 1000 --report evidence/benchmarks/anchor-report.json
```

The `target/` directories contain generated build output and are not part of
the component map.