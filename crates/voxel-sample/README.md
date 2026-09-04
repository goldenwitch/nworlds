# Voxel Cottage Sample

An independent sample consumer that puts the generic temporal engine and
target-neutral host into practice. It builds a small cottage from distinct
voxel block kinds, uses the engine's immutable
journal/worldline/query/presentation path, and supplies its own camera,
picking, native input, and `wgpu` rendering.

## Run

From the workspace root:

```text
cargo run --manifest-path crates/voxel-sample/Cargo.toml
```

Controls:

- Left click a visible voxel to publish its removal.
- Use the mouse wheel to publish voxel-scale changes.
- Press `Escape` or close the window to exit.

## Start Here

Read [engine_integration.rs](src/engine_integration.rs) first. It is the
single engine-facing example for this sample. It shows how a game specializes
engine types and uses the recommended boundaries:

- `Worldline<VoxelContext, VoxelFact>` for immutable history;
- `JournalWriter<VoxelFact>` for authoritative fact publication;
- `IndexedQuery` for direct state reconstruction;
- `GameState<VoxelState>` for an owned logical-time sample;
- `Renderer<VoxelState>` for state-first presentation; and
- `Frame<VoxelRenderOutput>` for owned target output.

The rest of the crate is deliberately divided by ownership:

| File | Owns |
| --- | --- |
| [`world.rs`](src/world.rs) | Voxel positions, block kinds, facts, state, scale, and cottage geometry. |
| [`engine_integration.rs`](src/engine_integration.rs) | The engine specialization and publication/query/presentation example. |
| [`camera.rs`](src/camera.rs) | Camera math and CPU ray/AABB picking. |
| [`render.rs`](src/render.rs) | Target-local `wgpu` execution through `RenderSink`. |
| [`main.rs`](src/main.rs) | Native window events and sample composition. |

## Event Path

```text
native click
  -> sample ray picker
  -> VoxelFact::Remove
  -> JournalWriter
  -> new immutable VoxelWorldline
  -> IndexedQuery
  -> GameState<VoxelState>
  -> Renderer + Tau
  -> Frame<VoxelRenderOutput>
  -> target RenderSink
```

The sample owns the meaning of a removal. The engine supplies immutable fact
history and direct query shape. The target supplies event delivery and pixels.

## Documentation

This README is the practical crate guide. The deeper architectural case study
is [proposals/voxel-sample.md](../../proposals/voxel-sample.md); it explains why
these engine features are useful and how the ownership boundary is intended to
scale to another game.
