# Caravan Sample

A reference-game sample showing how a concrete world uses the temporal engine
and target-neutral host. The native `caravan-sample` executable is the manual
sample; the separate `caravan-trace` executable produces deterministic stdout
evidence for the Caravan anchor.

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
| [`world-specific domain crates`](../caravan-domain) | Caravan geometry, values, journal entries, vegetation, hazards, fixtures, and reference projection. These are sample/reference-game meaning, not engine code. |
| [`engine_integration.rs`](src/engine_integration.rs) | The explicit crossing into generic engine types: worldline aliases, direct queries, journal publication, presentation, and persistence adapters. |
| [`interaction.rs`](src/interaction.rs) | Pure meaning for the first Caravan input action. |
| [`input.rs`](src/input.rs) | Sample input packets, semantic batches, windows, buffering, and interaction traits. |
| [`transformation.rs`](src/transformation.rs) | The closed sample transformation vocabulary between interaction and journal facts. |
| [`orchestrator.rs`](src/orchestrator.rs) | Mutable sample application control over selected immutable values. |
| [`stage.rs`](src/stage.rs) | The Caravan Stage composition and `GamePackage` boundary. |
| [`render.rs`](src/render.rs) | Caravan-owned backend-neutral render output. |
| [`host/`](src/host) | In-memory adapters used by sample composition proofs; canonical host ports live in `nworlds-host`. |
| [`package.rs`](src/package.rs) | The target-neutral `CaravanPackage` supplied to a host. |
| [`main.rs`](src/main.rs) | Native composition of `CaravanPackage` with `nworlds-desktop`; this is the `caravan-sample` executable. |
| [`bin/caravan-trace.rs`](src/bin/caravan-trace.rs) | Deterministic terminal walkthrough used by the snapshot and conformance evidence. |

## Engine Path

```text
Caravan reference crates
  -> engine_integration::CaravanWorldline
  -> Stage / Orchestrator
  -> CaravanPackage
  -> GameState<Snapshot> + Tau
  -> Frame<RenderBatch>
  -> nworlds-host / nworlds-desktop
```

The engine supplies generic time, journal, worldline, state, frame, query, and
renderer structures. Caravan supplies the domain payloads, rules, interaction
meaning, Stage/Orchestrator control, and render projection. The target receives
only the owned `Frame<RenderBatch>` through the host boundary.

`RenderOutput` remains a Caravan-owned semantic inspection view produced by
`project_output`; it is not an engine type and does not cross into the target.

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

The Orchestrator coordinates this cycle. It is sample application control code,
not an engine abstraction. The engine integration module is the place to study
the library composition.

## Run

From the workspace root:

```text
cargo run -p caravan-sample
```

The native sample opens the Caravan scene. Press `Space` to publish the first
sample interaction, press backquote/tilde to toggle the target-owned developer
console, and press `Escape` or close the window to exit.

The deterministic console evidence is a separate binary:

```text
cargo run -p caravan-sample --bin caravan-trace
```

The trace covers empty and created worlds, time boundaries, indexed Caravan
behavior, seeded journals, lookahead, branches, input publication, and
presentation. Its output is checked by the package snapshot test and the
standalone conformance evidence. It is a proof tool, not a second gameplay
surface.

Caravan production code remains target neutral: it supplies `CaravanPackage`,
semantic `InputPacket` values, and `Frame<RenderBatch>` production while
`nworlds-desktop` owns native lifecycle and `wgpu` execution.

## Design Records

- [Caravan Orchestrator anchor](../../proposals/caravan-orchestrator-anchor.md)
  describes the application-level Stage and Orchestrator experiment.
- [Input and interaction](../../proposals/input-and-interaction.md) describes
  the semantic input crossing.
- [Rendering contract](../../proposals/rendering-contract.md) describes the
  owned backend-neutral output.
- [Library contract](../../proposals/library-contract.md) describes the
  generic engine boundary that this sample specializes.
