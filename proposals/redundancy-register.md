# Redundancy Register

This is the measured register for redundancy across the repository. It
separates repeated contracts from distinct consumers, historical proof, and
supporting evidence. A row is a candidate for collapse only when the artifacts
assert the same contract or implement the same responsibility.

## Canonical Ownership Map

| Concept or artifact | Canonical owner | Other records are |
| --- | --- | --- |
| Fixed-point time and temporal SDK envelopes | `engine-time`, `engine-sdk`, exposed through `engine-api` | Tests, conformance, and sample integrations |
| Journals, timestamp authoring, branches, direct indexed queries | `engine-journal`, `engine-branches`, `engine-index`, exposed through `engine-api` | Game specializations and evidence |
| State-first presentation contract | `engine-presentation` and the rendering contract | Game render projections and target sinks |
| Passive input, storage, render, and package ports | `nworlds-host` | Target compositions and in-memory proofs |
| Stage/Orchestrator application semantics | `caravan-sample`'s `engine_integration.rs`, `stage.rs`, `orchestrator.rs` | `stage-layer.md`, `caravan-orchestrator-anchor.md`, implementation and evidence records |
| Semantic input and transformation boundary | `caravan-sample`'s `input.rs`, `interaction.rs`, `transformation.rs` plus transport primitives | `input-and-interaction.md`, `transport-and-journal.md`, implementation and evidence records |
| Renderer-agnostic render vocabulary | `engine-presentation::RenderBatch`, exposed through `engine-api` | Caravan/voxel projections and target sinks are clients; target lifecycle remains separate |
| Target/package/artifact resolution | `target-factory.md` | Support matrix, host proposal, CI evidence |
| Declared target regimes and support evidence | `support.vine` and `platform-support-matrix.md` | Target-factory resolution and CI records |
| Native lifecycle/backend execution | Generic target composition (`nworlds-desktop`) | Caravan/voxel client render projections and historical host graph |
| Reference-game meaning | `caravan-domain`, `caravan-vegetation`, `caravan-hazards`, `caravan-seeded`, `caravan-reference`, `caravan-persistence` | Demo, conformance, benchmarks |
| Primary evidence mapping | `evidence/clause-to-test.md` and the owning test/CI command | Snapshots, reports, and secondary observations |
| Execution planning | The active VINE for the live workstream | Completed VINEs as historical records |

## Duplication Groups

### R1: Temporal library contract

**Observed artifacts:** `library-contract.md`, `spec/initial.md`,
`roadmap.md`, `engine-api`, `engine-sdk`, and the generic
consumer/purity tests.

**Classification:** Mostly layered, not redundant. `spec/initial.md` owns the
mathematical model; `library-contract.md` owns the product boundary;
implementation crates own code; evidence owns proof.

**Decision:** Keep all four audiences, but remove copied normative equations
from summaries and link to the owning record. `engine-api` remains the only
advertised facade.

### R2: Host ports and presentation host

**Observed artifacts:** `presentation-host.md`, `stage-layer.md`, `target-factory.md`,
the host and target-factory proposals, `nworlds-host`, and Caravan host aliases.

**Classification:** True prose overlap; implementation layering is distinct.
`nworlds-host` is the implementation owner. `presentation-host.md` owns port
roles; `target-factory.md` owns minting/resolution; `stage-layer.md` owns the
application Stage boundary; the existing desktop host is historical proof.

**Decision:** `presentation-host.md` owns port definitions. Secondary records
link to it and keep only their boundary-specific constraints. Keep
target-factory text focused on package/artifact resolution. Do not create
another host library.

### R3: Stage and Orchestrator

**Observed artifacts:** `proposals/stage-layer.md`,
`proposals/caravan-orchestrator-anchor.md`, `nworlds-desktop`, `caravan-sample/src/stage.rs`,
`caravan-sample/src/orchestrator.rs`, and `engine_integration.rs`.

**Classification:** One application implementation with several historical and
contract records. The current Caravan integration file is the sample's example
seam; the generic engine does not own Stage or Orchestrator.

**Decision:** Keep `stage-layer.md` as the normative application boundary,
`caravan-orchestrator-anchor.md` as Caravan rationale/evidence, and completed
VINEs as history. Remove duplicate full ownership prose from future docs.

### R4: Input and transport

**Observed artifacts:** `input-and-interaction.md`,
`transport-and-journal.md`, `nworlds-host`,
`caravan-sample/src/input.rs`, `src/interaction.rs`, `src/host/input.rs`, and
`nworlds-host` input ports.

**Classification:** Two real boundaries are being described: reusable identity/
ordering transport and game-facing semantic interaction. The current code
matches that split. The host input adapter is transport; Caravan input is
meaning.

**Decision:** Keep `transport-and-journal.md` for reusable transport and
`input-and-interaction.md` for semantic interpretation. Summaries link rather
than restate both. No new input abstraction is introduced.

### R5: Rendering and RenderBatch

**Observed artifacts:** `rendering-contract.md`,
`presentation-host.md`, `target-factory.md`,
`engine-presentation`, Caravan `render.rs`, the former voxel target sink, and
`nworlds-desktop/src/wgpu.rs`.

**Classification:** The engine presentation contract, two sample projections,
and the reusable desktop lifecycle are distinct. The shared target vocabulary
is implemented as `engine-presentation::RenderBatch`; the generic desktop
lifecycle is now the shared implementation, while historical/client evidence
remains separate.

**Decision:** `rendering-contract.md` owns current state-first presentation;
`engine-presentation::RenderBatch` owns the target vocabulary;
`target-factory.md` records the factory-facing implementation constraint.
Samples keep state-to-batch projection; `nworlds-desktop` owns generic lifecycle
execution and both client compositions use it.

### R6: Target factory, support, and host target records

**Observed artifacts:** `target-factory.md`,
`platform-support-matrix.md`, `support.vine`, `presentation-host.md`, and the
completed Windows nodes in `nworlds-desktop`.

**Classification:** Four axes are distinct: target minting, declared support
regimes, runtime ports, and historical proof. The risk is repeated prose that
makes them look like competing contracts.

**Decision:** `target-factory.md` is the only active future target/package
plan. `support.vine` owns target status/evidence. `presentation-host.md` owns
runtime port roles. The existing desktop host remains historical proof and is not extended.

### R7: Code-level target composition

**Observed artifacts:** `nworlds-desktop`, Caravan's in-memory host aliases,
voxel-sample's native `winit`/`wgpu` target, and `nworlds-host::ApplicationHost`.

**Classification:** `nworlds-host` is reusable passive composition;
Caravan host aliases are test convenience; `nworlds-desktop` now owns reusable
generic lifecycle execution with a synthetic package proof, and both Caravan
and voxel samples provide package-owned client compositions. The historical
Caravan proof remains evidence; native runtime/device and voxel persistence
claims remain distinct, while game-to-target render vocabulary is shared
through `RenderBatch`.

**Decision:** Keep target execution in `nworlds-desktop`, keep game meaning in
the clients, and retain only historical/native evidence or package-owned
client code with a distinct responsibility. The target-factory remediation
wave has completed both client compositions; persistence and runtime evidence
remain separate requirements.

### R8: Caravan reference crates

**Observed artifacts:** `caravan-domain`, vegetation, hazards, seeded,
reference, persistence, demo, and their evidence consumers.

**Classification:** Layered reference-game implementation, not redundant crates.
Each crate has a distinct production responsibility and active consumers.

**Decision:** Keep the crate split. Revisit only when a concrete consumer shows
that two rule/fixture/codec boundaries are the same responsibility.

### R9: Public facades and compatibility views

**Observed artifacts:** `engine-api`, direct crate imports in tests, Caravan
aliases, `InputPacketSet`, and host-local type aliases.

**Classification:** Generic `engine-api` is canonical. Caravan aliases are
consumer conveniences. `InputPacketSet` is an explicitly retained compatibility
membership view. Direct test imports are evidence consumers, not public facade
claims.

**Decision:** Do not add another facade. Mark compatibility views as such and
keep them out of the generic contract unless a second consumer requires them.

### R10: Evidence and status claims

**Observed artifacts:** `evidence/clause-to-test.md`, workspace tests,
conformance tests, purity tests, benchmarks, snapshots, CI, CodeQL, and manual
Windows evidence.

**Classification:** Multiple regimes are legitimate, but several summaries
repeat the same “green” claim without naming scope. The primary matrix should
point to commands; reports/snapshots are artifacts.

**Decision:** `evidence/clause-to-test.md` is the primary map. CI owns automated
execution; CodeQL owns security analysis; target/device records remain separate
from compile evidence. Secondary docs link to the matrix.

### R11: Filesystem/package residue

**Observed artifact:** `crates/caravan-windows/src` existed as an empty,
untracked directory with no manifest, workspace membership, or references.

**Classification:** Pure residue, not a package or architectural layer.

**Decision:** Removed during the audit. No `caravan-windows` crate is created.

## Current State

- Active production crate redundancy: **no confirmed duplicate**.
- Generic host implementation count: **one** (`nworlds-host`).
- Target-local client compositions: **two**, both using the shared desktop
  lifecycle and `RenderBatch` vocabulary.
- Normative prose overlap: **reduced**; host and rendering summaries link to
  their owners, while the distinct input, transport, Stage, and target-factory
  contracts remain separate.
- Empty/unowned filesystem residue: **removed** (`caravan-windows`).
- Remaining gaps are implementation or evidence work, not additional
  abstractions: target runtime/device coverage, persistence evidence, and the
  selected demo gameplay slice.
