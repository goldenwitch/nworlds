# Voxel Cottage Sample

An independent sample consumer that puts the generic temporal engine and
target-neutral host into practice. It builds a small cottage from distinct
voxel block kinds, uses the engine's immutable
journal/worldline/query/presentation path, and supplies its own camera,
tool palette, picking, semantic input, and `RenderBatch` projection. The
reusable desktop host supplies native lifecycle and `wgpu` execution.

## Run

From the workspace root:

```text
cargo run --manifest-path crates/voxel-sample/Cargo.toml
```

Controls:

- Press `1` or click the remove icon to select the Remove tool.
- Press `2` or click the fire icon to select the Fire tool.
- Left click a visible voxel to apply the selected tool.
- The two bottom sliders scrub `LogicalTime` (cyan) and `Tau` (orange).
- The left/right arrow controls step each time axis backward or forward.
- Time advances automatically by default; touching a slider or step control
  pauses it, and clicking into the world resumes it.
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
- `redraw(worldline, logical_time, camera, tau)` for pure dual-time sampling; and
- `Frame<RenderBatch>` for owned target output.

The rest of the crate is deliberately divided by ownership:

| File | Owns |
| --- | --- |
| [`world.rs`](src/world.rs) | Voxel positions, block kinds, tools, facts, state, scale, and cottage geometry. |
| [`engine_integration.rs`](src/engine_integration.rs) | The engine specialization, publication/query path, and pure redraw example. |
| [`camera.rs`](src/camera.rs) | Camera math and CPU ray/AABB picking. |
| [`tool.rs`](src/tool.rs) | Palette slots, hit testing, and disposable tool icons. |
| [`package.rs`](src/package.rs) | Voxel `GamePackage`, semantic click/wheel/resize handling, and immutable publication. |
| [`input.rs`](src/input.rs) | Native `WindowEvent` translation into voxel-owned packets. |
| [`main.rs`](src/main.rs) | Thin composition of the voxel package with `nworlds-desktop`. |

## Event Path

```text
native click, slider, or tool key
  -> VoxelInputAdapter
  -> VoxelPackage
  -> TimelineControls or VoxelFact
  -> JournalWriter when authoritative
  -> selected LogicalTime
  -> state(worldline, logical_time)
  -> GameState<VoxelState> + Tau + Camera
  -> redraw
  -> Frame<RenderBatch>
  -> target RenderSink
```

The sample owns tool meaning, picking, removal, fire spawning, and scale
adjustment. Tool selection is an authoritative `VoxelFact::SelectTool` and is
therefore reconstructed into `VoxelState` at `LogicalTime`. The engine
supplies immutable fact history and direct query shape. The generic desktop
target supplies event delivery and pixels without inspecting voxel state or
block kinds.

`redraw` is the pure presentation composition. `LogicalTime` selects the
complete authoritative voxel state from the immutable worldline; `Tau` labels
the independent presentation sample. `VoxelPackage::present_at` exposes the
same path for explicit scrubbing without changing the package's selected
sample or camera.

The camera is explicit presentation state, not part of `VoxelState` or the
journal. `VoxelPackage::present` projects its selected complete state with the
current camera and visual `Tau` through the pure `frame_with_camera` path.
Interactive camera changes update that view value only. A visual animation
with an independent phase should carry its own presentation-time value and be
sampled by a pure function; a camera becomes such an animation only when it
has a time-varying trajectory.

The bottom controls are presentation state. `TimelineControls` starts in
automatic mode and advances both axes by fixed package-configured deltas during
idle updates. Slider and step input switches to manual mode. A pointer-down
outside the control rectangles reports a world interaction and resumes
automatic mode before the voxel package applies its selected tool. Neither the
control mode, slider drag, camera, nor `Tau` is placed in `VoxelState`.

The controls use `Viewport` and `ScreenPoint` for native pixel input,
`LogicalTimeDelta` and `TauDelta` for movement units, and
`TimelineLayout::auto_scale` for resize-safe bottom-row geometry. The time
thumbs use a fixed-focus parabolic reprojection, so advancing absolute time
does not run the thumb into a hard visual endpoint. Rendering and hit-testing
use the same auto-scaled layout, so the visible slider and the interactive
slider remain the same control.

## Fire Simulation

Selecting Fire and clicking a voxel publishes `VoxelFact::SpawnFire`. The
query derives fire objects and surrounding voxel effects from that fact at the
requested `LogicalTime`; it does not run a mutable simulation loop. The first
spread matrix is:

```text
2 1 2
1 0 1
2 1 2
```

Orthogonal occupied neighbors are affected after one game tick and diagonal
occupied neighbors after two. Fires expire independently after three ticks,
and derived secondary fires can continue the deterministic cascade.

Fire existence and spread are logical-time behavior. Flame geometry cycles
through presentation frames from `Tau`, so redraws can animate the same
complete `GameState<VoxelState>` without changing the voxel query result.
Wave function collapse is deliberately deferred until the fixed matrix has
produced evidence about which visual or spread variation is missing.

## Documentation

This README is the practical crate guide. The deeper architectural case study
is [proposals/voxel-sample.md](../../proposals/voxel-sample.md); it explains why
these engine features are useful and how the ownership boundary is intended to
scale to another game.
