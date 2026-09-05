# Voxel Cottage Sample

An independent sample consumer that puts the generic temporal engine and
target-neutral host into practice. It builds a small cottage from distinct
voxel block kinds, uses the engine's immutable
journal/worldline/query/presentation path, and supplies its own camera,
picking, semantic input, and `RenderBatch` projection. The reusable desktop
host supplies native lifecycle and `wgpu` execution.

## Run

From the workspace root:

```text
cargo run --manifest-path crates/voxel-sample/Cargo.toml
```

Controls:

- Left click a visible voxel to publish its removal.
- Right-drag to orbit the presentation camera around the cottage.
- Use the mouse wheel to publish voxel-scale changes.
- Press `R` to reset the camera; use `+`/`-` to adjust camera distance.
- Press `Escape` or close the window to exit.

The window includes a target-owned developer console showing the package
identity, package version, host requirement, render vocabulary, and the Git
build revision used to compile the executable. A `-DIRTY` suffix means the
binary was built while the source worktree had uncommitted changes.

Press the backquote/tilde key to toggle the console without changing game
state or presentation camera state.

## Start Here

Read [engine_integration.rs](src/engine_integration.rs) first. It is the
single engine-facing example for this sample. It shows how a game specializes
engine types and uses the recommended boundaries:

- `Worldline<VoxelContext, VoxelFact>` for immutable history;
- `JournalWriter<VoxelFact>` for authoritative fact publication;
- `IndexedQuery` for direct state reconstruction;
- `GameState<VoxelState>` for an owned logical-time sample;
- `Renderer<VoxelState>` for state-first presentation; and
- `Frame<RenderBatch>` for owned target output.

The rest of the crate is deliberately divided by ownership:

| File | Owns |
| --- | --- |
| [`world.rs`](src/world.rs) | Voxel positions, block kinds, facts, state, scale, and cottage geometry. |
| [`engine_integration.rs`](src/engine_integration.rs) | The engine specialization and publication/query/presentation example. |
| [`camera.rs`](src/camera.rs) | Camera math and CPU ray/AABB picking. |
| [`package.rs`](src/package.rs) | Voxel `GamePackage`, semantic click/wheel/resize handling, and immutable publication. |
| [`input.rs`](src/input.rs) | Native `WindowEvent` translation into voxel-owned packets. |
| [`main.rs`](src/main.rs) | Thin composition of the voxel package with `nworlds-desktop`. |

## Event Path

```text
native click
  -> VoxelInputAdapter
  -> VoxelPackage
  -> sample ray picker
  -> VoxelFact::Remove
  -> JournalWriter
  -> new immutable VoxelWorldline
  -> IndexedQuery
  -> GameState<VoxelState>
  -> Renderer + Tau
  -> Frame<RenderBatch>
  -> target RenderSink
```

The sample owns the meaning of a removal, picking, and scale adjustment. The
engine supplies immutable fact history and direct query shape. The generic
desktop target supplies event delivery and pixels without inspecting voxel
state or block kinds.

## Documentation

This README is the practical crate guide. The deeper architectural case study
is [proposals/voxel-sample.md](../../proposals/voxel-sample.md); it explains why
these engine features are useful and how the ownership boundary is intended to
scale to another game.
