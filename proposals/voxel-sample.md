# Voxel Sample: Engine Features in Practice

For the practical crate guide and run instructions, see the colocated
[voxel-sample README](../crates/voxel-sample/README.md). This proposal is the
deeper architectural case study: it explains which engine features the sample
uses, why they are useful, and where game and target ownership remain.

The voxel cottage is an independent consumer of the temporal engine. It exists
as a small, concrete answer to two questions:

1. What does a game receive from the engine?
2. What does the game still own for itself?

The sample's engine-facing example is
[`engine_integration.rs`](../crates/voxel-sample/src/engine_integration.rs).
It is the first file to read when evaluating the recommended composition.

## Boundary

```text
voxel-sample game model
  VoxelFact + VoxelContext + VoxelState
        |
        v
engine integration example
  Worldline + JournalWriter + IndexedQuery + GameState + Frame
        |
        v
sample target
  generic desktop lifecycle + native event delivery + RenderSink<Frame<RenderBatch>>
```

The sample owns voxels, cottages, blocks, and clicking while reusing the
engine's journal, worldline, query envelope, branch container, and frame
envelope. The positive composition is:

```text
voxel-sample -> engine-api / nworlds-host / target libraries
```

## Presentation Camera

The camera is sample-owned presentation state. It is not a `VoxelFact`, is not
stored in `VoxelState`, and never enters the authoritative voxel worldline.
One `Camera` value is shared by both presentation and picking:

```text
WindowEvent
  -> VoxelInputAdapter
  -> camera orbit/zoom/reset or voxel semantic packet
  -> Camera
       |-> VoxelPackage picking
       `-> VoxelState -> RenderBatch projection
```

The first controls are right-drag orbit, `R` reset, `+/-` camera distance,
mouse-wheel voxel scale, and viewport aspect updates. Camera pitch and distance
are clamped deterministically. Camera-only packets change presentation context
without publishing facts; click/scale facts select a new complete voxel state,
reset visual `Tau`, and preserve the camera.

## Features Used

| Engine feature | How the sample uses it | Why it matters |
| --- | --- | --- |
| Opaque typed payloads | `VoxelContext`, `VoxelFact`, and `VoxelState` are supplied by the sample and carried by generic engine types. | The engine provides structure without taking ownership of game vocabulary. |
| `JournalWriter` | The cottage is authored as `Place` and `SetScale` facts. A click appends a `Remove` fact through the writer. | Timestamp authority stays in one place; the caller never manufactures authoritative timestamps. |
| Immutable `Worldline` | `VoxelWorldline` specializes `Worldline<VoxelContext, VoxelFact>`. | The selected game history is a value that can be retained, queried, compared, or replaced without a mutable board. |
| `IndexedQuery` | `VoxelQuery` interprets visible facts into the current `VoxelState`. | State is reconstructed directly from the requested worldline/time instead of being advanced from a hidden current state. |
| Direct state query | `state(worldline, logical_time)` is the only authoritative state lookup. | The sample can query arbitrary logical times with results stable under query order and frame history. |
| `GameState` | The query returns `GameState<VoxelState>`, including the exact sampled logical time. | Game state is an owned, time-labelled result rather than an ambient mutable context. |
| State-first presentation | `VoxelRenderer` implements `Renderer<VoxelState>`, and `frame` uses `present(GameState, Tau)`. | Rendering is downstream of authoritative state and cannot become an interaction authority. |
| Owned `Frame` | `VoxelFrame` is the engine `Frame<RenderBatch>` envelope consumed by the shared target sink. | The target receives disposable render data, not a worldline, journal, input queue, or mutable game object. |
| Host render port | The generic desktop composition implements `RenderSink<Frame<RenderBatch>>` for the voxel package. | `wgpu` execution remains target code; it does not leak into the game model or engine contract. |

## The Click Path

Clicking a voxel is deliberately a publication cycle, not a direct mutation:

1. `VoxelInputAdapter` records cursor position and emits a package-owned click
  packet through the generic host ingress.
2. `VoxelPackage` asks the sample camera to cast a ray through the current
  `VoxelState` and select
   the nearest voxel box.
3. The sample creates `VoxelFact::Remove { position }`.
4. `engine_integration::publish` records that fact through `JournalWriter`.
5. The sample replaces its selected `VoxelWorldline` with a newly published
   immutable value.
6. `engine_integration::state` queries the selected worldline.
7. `engine_integration::frame` presents the resulting `GameState`.
8. The shared desktop render sink translates the owned batch into `wgpu`
  commands.

The click handler knows what a removal means for the game. The engine knows how
to retain and query the fact. The target knows how to turn the resulting frame
into pixels.

## The Scale Path

Voxel scale is a game state parameter represented by `VoxelFact::SetScale` and
`VoxelScale`. The value is fixed-point in the authoritative sample state, with
a bounded continuous range from `0.350` to `1.650` in increments of `0.001`.

Mouse-wheel input changes the parameter by publishing another fact. The query
selects the latest visible scale, and presentation uses that value when
constructing render geometry. There is no renderer-side scale cursor and no
hidden target-local scale state.

This is a useful small example of the engine boundary: a value can be adjusted
continuously by the user while remaining part of an immutable, queryable
worldline.

## What The Engine Makes Possible

### The sample has no mutable board

The cottage starts as journal facts. Removing a block creates another journal
fact. Any state observed by the renderer is derived from the selected immutable
worldline.

That makes several behaviors natural rather than special cases:

- a prior worldline can be retained for comparison;
- the same logical time can be queried repeatedly;
- a future or past logical time can be inspected without rewinding a board;
- a branch can be introduced later without changing the sample's state model;
- persistence can store facts rather than GPU state or frame history.

### Game meaning stays local

`world.rs` owns block kinds, voxel positions, facts, state, scale rules, and
cottage geometry. `engine-api` supplies the generic structures that carry and
query those values.

The integration file is consequently readable as an example of composition,
not as a second game engine. It shows the small amount of code a new game needs
to specialize the generic engine.

### Presentation stays honest

`VoxelRenderer` receives `GameState<VoxelState>` and `Tau`. Camera, input,
device, and host-clock concerns remain in their target/application owners, so
the camera and renderer can be replaced without changing authoritative voxel
state production.

The generic desktop target is the target adapter. It consumes the owned
`Frame<RenderBatch>` and performs backend work for the already-selected voxels;
the voxel package never sees a device, surface, or backend command.

### The sample can teach the library

The sample is intentionally small enough to serve as a public composition
example:

- `world.rs` answers: what does this game mean?
- `engine_integration.rs` answers: how does the game use the engine?
- `camera.rs` answers: how does the sample orbit, project, and select a voxel?
- `package.rs` answers: how do semantic events become immutable voxel facts?
- `input.rs` answers: how are native events translated into voxel packets?
- `main.rs` answers: how is the package connected to the generic desktop host?

That separation is the point of the sample. It keeps the engine contract
visible without pretending that every useful game or target abstraction belongs
in the engine.

## Scope

The sample currently demonstrates:

- many distinct block kinds in a small cottage scene;
- journal-authored creation and removal;
- immutable worldline replacement;
- direct indexed state queries;
- fixed-point scale as game state;
- CPU ray picking;
- state-first owned rendering; and
- generic `winit`/`wgpu` execution through `nworlds-desktop`.

Its current scope is the cottage, immutable block removal, adjustable scale,
ray picking, and state-first rendering. General voxel-engine concerns such as
editing, synchronization, mesh optimization, chunk streaming, and broader
input remain future requirements with their own ownership and evidence.

## Try It

From the workspace root:

```text
cargo run --manifest-path crates/voxel-sample/Cargo.toml
```

Click a visible voxel to publish its removal. Use the mouse wheel to publish
scale changes. Press `Escape` or close the window to exit.

The sample currently leaves persistence unavailable explicitly; a voxel
worldline codec is a separate game-owned requirement rather than a target-host
concern.
