# Voxel Showcase: Controls and Matrix Fire

This proposal turns the voxel cottage into a small simulation showcase. Each
new player control is traced through the same architecture so a developer can
see which values belong to input transport, package control, authoritative
journal state, direct query, presentation, and target execution.

## First Slice

The first slice contains two sample-owned tools whose selection is journaled:

- `Remove`: the existing voxel-removal action, now explicitly selected and
  represented by a remove icon;
- `Fire`: a new action that spawns one fire object on the nearest picked voxel.

The default selected tool remains `Remove`, so the existing left-click behavior
is preserved for callers that do not select another tool.

The native control vocabulary is:

```text
1                  select Remove
2                  select Fire
palette icon click select the corresponding tool
left click         apply the selected tool to the picked voxel
```

Tool selection is authoritative sample state. Selecting a tool publishes a
`VoxelFact::SelectTool` fact, changes the immutable worldline, and is folded
into `VoxelState` by the direct query. The palette can therefore show the
selected tool without a package field or an extra redraw input.

## Control Propagation

The two actions cross the same boundaries with different ownership:

```text
native WindowEvent
  -> VoxelInputAdapter
  -> VoxelInputPacket
  -> OrderedInputBatch
  -> VoxelPackage
       tool selection:
         VoxelFact::SelectTool { tool }
         -> JournalWriter
         -> immutable VoxelWorldline

       world action:
         state(worldline, logical_time).tool()
         + Camera::pick(GameState<VoxelState>)
         -> VoxelFact::Remove | VoxelFact::SpawnFire
  -> state(worldline, logical_time)
  -> redraw(worldline, logical_time, camera, tau)
  -> Frame<RenderBatch>
  -> RenderSink
```

The adapter translates native details. The package interprets the packet and
publishes tool selection or world actions. The interaction path owns picking
and action meaning. The journal writer remains the only authoritative
publication path. The renderer receives a complete queried state and the
independent presentation coordinate; the sample camera remains presentation
context, while selected-tool meaning is part of the queried world state.

## Tool Palette

The sample renders a small target-neutral palette in clip-space geometry at the
edge of the existing `RenderBatch`. It contains one icon per tool:

- `Remove` is a high-contrast remove/trash mark;
- `Fire` is a flame mark;
- the selected slot has a deterministic selection treatment.

The palette is disposable frame data. It contains no input queue, worldline,
journal, cursor history, or device handle. Palette hit testing uses the same
fixed normalized slot layout as palette projection, then publishes a
`SelectTool` fact; selecting an icon is a package input decision, not a
renderer decision.

This is intentionally a sample-owned presentation composition. The generic
engine continues to provide `GameState`, `Tau`, `Renderer`, and `Frame`; no
HUD or universal tool system is added to the engine for one showcase.

## Fire State

`SpawnFire { position }` is the authoritative fire fact for the first fire
slice. `SelectTool` is also journaled because it changes how later input is
interpreted and must be reconstructible from the worldline. A queried
`VoxelState` contains the selected tool and derived fire objects alongside
voxels. A fire object carries its position and logical age; it is not a mutable
runtime object and it is not stored in the renderer.

The fire query is pure over the visible journal prefix and requested
`LogicalTime`:

1. Fold ordinary voxel facts into the base voxel map.
2. Collect visible fire spawns with their journal timestamps.
3. Derive fire ages from the requested logical time using the fixed game-tick
   period.
4. Apply the fire spread matrix in deterministic event order.
5. Remove affected voxels and retain live fire objects in the returned state.

No query advances a board, mutates an earlier state, or owns an RNG. Repeating
a query at the same worldline and logical time returns the same voxel and fire
state regardless of query order.

## Fire Matrix

The first matrix is a fixed 3x3 delay matrix centered on the source fire:

```text
2 1 2
1 0 1
2 1 2
```

Zero means no neighboring interaction. `1` means an orthogonal neighbor is
affected after one game tick. `2` means a diagonal neighbor is affected after
two game ticks. An affected occupied voxel is removed and receives a derived
fire object at that event time. Fire expires after three game ticks, and its
source voxel is removed at expiry. Events are deduplicated by position using
the earliest deterministic fire-start time, preventing query-order-dependent
cascades.

This is a deliberately small matrix rule for the showcase. It is not a claim
that all fire simulation belongs in the generic engine, nor does it yet claim
that wave function collapse is the right behavior generator.

## Independent Fire Animation

Simulation and animation use the two existing time variables separately:

```text
VoxelState fire objects = f(worldline, logical_time)
fire animation frame    = g(fire object, tau)
```

The fire's logical age controls whether it exists and when it affects nearby
voxels. Its visual frame is selected from `Tau` with a fixed frame period and a
stable position-derived phase. Two redraws of the same `GameState` at different
`Tau` values may produce different flame geometry or color while leaving the
queried voxel state identical. Equal state and `Tau` inputs produce equal
render batches.

No previous frame, host clock, GPU simulation, or mutable animation object is
required.

## Wave Function Collapse

Wave function collapse is a possible later presentation or spread-pattern
technique, not part of this first implementation. The fixed matrix must first
supply evidence about what feels unnatural and which variation is actually
needed. Any later WFC experiment must preserve these constraints:

- its choices are deterministic from explicit state, `LogicalTime`, `Tau`, and
  stable fire identity or seed values;
- it does not read renderer history or a host RNG;
- it cannot mutate authoritative state during rendering; and
- it has a named owner at the sample/domain boundary before any generic
  extraction is considered.

## Acceptance Evidence

The slice is complete when focused evidence proves:

- selecting `Remove` or `Fire` publishes `VoxelFact::SelectTool` and changes
  the queried state while preserving the parent worldline;
- the palette exposes both icons and marks the selected tool deterministically;
- a left click with `Remove` publishes `VoxelFact::Remove` and preserves the
  parent worldline;
- a left click with `Fire` publishes `VoxelFact::SpawnFire` and returns a fire
  object at the selected logical time;
- matrix effects occur at their declared logical delays and are independent of
  query order;
- equal state and `Tau` inputs produce equal frames;
- changing `Tau` changes fire presentation without changing voxel state;
- changing `LogicalTime` changes simulation state only where the matrix event
  schedule requires it; and
- the complete workspace test suite remains green.

The manual showcase command remains:

```text
cargo run --manifest-path crates/voxel-sample/Cargo.toml
```
