# Timeline Controls Library

This proposal records the first reusable screen-control boundary for nworlds.
It is a target-neutral presentation/control library, not a widget toolkit and
not an authoritative game-state layer.

## Boundary

The library owns:

- explicit screen units (`Pixels`, `ScreenPoint`, and `Viewport`) and normalized
    pointer conversion;
- normalized screen rectangles and pointer hit-testing;
- two timeline slider tracks, one for `LogicalTime` and one for `Tau`;
- directional step controls for both axes;
- distinct logical-time and Tau delta units for automatic and manual movement;
- viewport-aware `TimelineLayout::auto_scale` for responsive control geometry;
- fixed-focus parabolic reprojection of unbounded absolute times onto the
    bounded slider tracks;
- automatic versus manual timeline mode; and
- owned target-neutral control geometry.

A game owns the meaning of a time change, the selected `Worldline`, the
selected `GameState`, camera/view values, input vocabulary, and any journal
publication. A target translates native events into pointer observations and
executes the returned `RenderBatch`.

```text
native pointer observation
    -> target-neutral control event
    -> engine-controls timeline state
    -> selected LogicalTime/Tau values
    -> game query and presentation composition
    -> owned RenderBatch
```

The library does not know a game domain, journal payload, worldline, renderer
backend, window type, device, or host clock.

## Timeline Behavior

A new timeline starts in `Automatic` mode. One host redraw step applies
configured fixed deltas to both `LogicalTime` and `Tau`. Input ingestion alone
does not advance either axis. The deltas are values supplied by the package;
they are not measured from a host clock.

A slider interaction or any directional step enters `Manual` mode. Automatic
updates then become no-ops until an explicit resume. A pointer-down outside the
controls reports `World`, resumes `Automatic` mode, and lets the game handle
the world interaction. This makes a world click both a game action opportunity
and the resume gesture without making the control library understand the game.

Logical time and Tau remain separate values:

- changing `LogicalTime` requires the game to query a new complete `GameState`;
- changing `Tau` only changes downstream presentation sampling; and
- neither control operation mutates a published worldline.

A slider track is a projection surface, not a gameplay bound. `LogicalTime` and
`Tau` may progress indefinitely without changing their absolute values. The
fixed-focus projection maps finite times strictly inside the slider edges and
approaches either edge asymptotically. Exact pointer endpoints map to the
representable numeric limits rather than introducing a finite gameplay bound.

For a focus time `t0`, horizon `H`, relative time distance `d = t - t0`, and
slider focus fraction `F`, let `r` be the normalized distance from the focus.
The projection uses:

```text
abs(d) = H * r^2 / (1 - r)

d < 0: fraction = F - r * F
d > 0: fraction = F + r * (1 - F)
```

This is a presentation projection only;
it does not change the absolute time stored by the control or the time accepted
by the game query. The default focus is `350/1000`, placing the current time at
35% of the track and leaving additional room in the forward direction; a game
can select another bounded `SliderFocus` when its presentation needs differ.

Time movement uses explicit delta units. `LogicalTimeDelta` and `TauDelta` are
signed distances measured in engine ticks; they are not absolute
`LogicalTime`/`Tau` values and are not interchangeable. Screen input uses
`ScreenPoint` and `Viewport` rather than bare coordinate tuples.

`TimelineLayout::auto_scale(viewport)` starts from a 960x720 design in pixel
units, scales it uniformly to the available viewport, anchors it to the bottom,
and returns normalized clip-space rectangles. This is screen-layout scaling;
it is separate from the fixed-focus time reprojection. The same layout drives
rendering and hit-testing after a resize, so a control does not drift away from
its visible geometry.

## Presentation Rule

Control state is presentation/control state. It must not be folded into a
game's authoritative `GameState` unless the game explicitly decides that the
control itself is game meaning and publishes a domain fact.

The control geometry is disposable output. Equal control values and equal layout
inputs produce equal geometry. The generic renderer receives only
`GameState + Tau`; a client projection may append the controls library's owned
geometry alongside other explicit presentation values such as a camera.

## First Consumer

The voxel sample is the first integration consumer. It supplies fixed demo
ranges and deltas, maps native pointer packets to the library, queries its
selected `VoxelState` at the library's selected `LogicalTime`, and appends the
library's geometry to the voxel `RenderBatch`. The sample remains responsible
for tool selection, world picking, journal publication, and camera state.

This first slice deliberately does not introduce text labels, layout discovery,
focus navigation, accessibility semantics, or a general retained UI tree.
Those concerns require a concrete second consumer before they become library
contracts.
