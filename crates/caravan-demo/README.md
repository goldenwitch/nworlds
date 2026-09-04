# Caravan Demo

A reference-game sample showing how a concrete world uses the temporal engine
and target-neutral host. The demo makes the Caravan anchor observable through a
terminal trace and host-facing tests.

## Start Here

Read [engine_integration.rs](src/engine_integration.rs) first. It is the
single engine-facing example for this sample. It shows how the Caravan game
specializes the generic surfaces for:

- immutable `ReferenceWorldline` and `Journal` values;
- journal-owned `JournalWriter` publication;
- direct reference-state queries at arbitrary `LogicalTime` values;
- immutable counterfactual and corrected branches;
- state-first `GameState + Tau -> Frame` presentation; and
- the target-neutral `GamePackage` composition used by the host.

The rest of the crate is organized around ownership:

| File | Owns |
| --- | --- |
| [`world-specific domain crates`](../caravan-domain) | Caravan geometry, values, journal entries, vegetation, hazards, fixtures, and reference projection. |
| [`engine_integration.rs`](src/engine_integration.rs) | Caravan's engine specializations, query/presentation wrappers, journal publication, and persistence crossing. |
| [`interaction.rs`](src/interaction.rs) | The meaning of the first Caravan input action. |
| [`input.rs`](src/input.rs) | Input packets, semantic batches, windows, buffering, and interaction traits. |
| [`transformation.rs`](src/transformation.rs) | The closed transformation vocabulary between interaction and journal facts. |
| [`orchestrator.rs`](src/orchestrator.rs) | Mutable application control over selected immutable values. |
| [`stage.rs`](src/stage.rs) | Stage and `GamePackage` composition. |
| [`render.rs`](src/render.rs) | Caravan-owned backend-neutral render output. |
| [`host/`](src/host) | In-memory host adapters used by composition proofs. |
| [`package.rs`](src/package.rs) | The initial Caravan package fixture supplied to a host. |
| [`main.rs`](src/main.rs) | The deterministic terminal walkthrough. |

## Engine Path

```text
Caravan facts and context
  -> engine_integration::CaravanWorldline
  -> engine_integration::state
  -> GameState<Snapshot>
  -> engine_integration::present_state
  -> Frame<RenderBatch>
  -> host RenderSink
```

`RenderOutput` remains a Caravan-owned semantic inspection view produced by
`project_output`; the target boundary carries the shared `RenderBatch`.

A primary-button input follows the same shape as the voxel sample:

```text
semantic input
  -> CaravanInteraction
  -> Transformation::SetTerrain
  -> engine_integration::append_actual
  -> new immutable worldline
  -> direct query
  -> owned frame
```

The Orchestrator coordinates this cycle. It is application control code; the
engine integration module is the place to study the library composition.

## Run

From the workspace root:

```text
cargo run --manifest-path crates/caravan-demo/Cargo.toml
```

The terminal trace covers empty and created worlds, time boundaries, indexed
Caravan behavior, seeded journals, lookahead, branches, input publication, and
presentation.

The historical native proof is now reproduced through the reusable desktop
host with a sample-owned input adapter:

```text
cargo run --manifest-path crates/caravan-demo/Cargo.toml --example desktop
```

The example is a dev composition. Caravan production code remains target
neutral; it supplies `CaravanPackage`, semantic `InputPacket` values, and
`Frame<RenderBatch>` production while `nworlds-desktop` owns native lifecycle
and `wgpu` execution.

## Design Records

- [Caravan Orchestrator anchor](../../proposals/caravan-orchestrator-anchor.md)
  describes the application-level Stage and Orchestrator experiment.
- [Input and interaction](../../proposals/input-and-interaction.md) describes
  the semantic input crossing.
- [Rendering contract](../../proposals/rendering-contract.md) describes the
  owned backend-neutral output.
- [Library contract](../../proposals/library-contract.md) describes the
  generic engine boundary that this sample specializes.
