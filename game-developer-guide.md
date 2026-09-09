# Game Developer Guide

This is the current implementation guide for building a target-neutral game
consumer in this workspace. It describes the generic Rust engine and the host
boundary that exists today, while the proposed `nworlds` CLI remains a design
surface.

## Start Here

Use the samples as executable documentation:

- [Voxel sample](crates/voxel-sample/README.md) is the smallest independent
  consumer. Start with its [engine integration](crates/voxel-sample/src/engine_integration.rs)
  and [world model](crates/voxel-sample/src/world.rs).
- [Caravan sample](crates/caravan-sample/README.md) shows the reference game's
  larger domain and application composition. Start with its [engine integration](crates/caravan-sample/src/engine_integration.rs).
- [Controls library proposal](proposals/controls-library.md) defines the
  reusable timeline-control boundary used by the voxel sample.
- [`engine-api`](crates/engine-api/src/lib.rs) is the generic facade. A game
  should normally depend on this facade instead of importing every engine
  crate directly.

Run the independent sample from the workspace root:

```text
cargo run --manifest-path crates/voxel-sample/Cargo.toml
```

The target-neutral library checks are:

```text
cargo test --workspace --locked
cargo test --manifest-path tests/conformance/Cargo.toml --locked
```

## Core Model

The engine evaluates complete values directly. A game represents authoritative
history as an immutable worldline and derives the selected complete state for
each request.

```text
immutable Worldline<C, P> + LogicalTime
    -> GameState<S>
GameState<S> + Tau
    -> Frame<F>
Frame<F>
    -> target RenderSink
```

A `Worldline<C, P>` combines an immutable game context with an immutable
journal of game facts. The game owns `C`, `P`, and `S`; the engine owns the
query envelopes and evaluation boundary. `LogicalTime` selects authoritative
game state. `Tau` selects presentation sampling for an already-selected state.

The two central operations are:

```text
state(worldline, logical_time) -> GameState
present(game_state, tau) -> Frame
```

Use `GameSurface` as the agent-facing composition boundary. The game supplies
its `IndexedQuery` and fact schema; the surface supplies explicit observation,
journal, branch, preview, commit, and discard operations:

```text
GameSurface
  manifest + fact schema
  branch handle + LogicalTime + Tau -> observation
  branch handle + revision + facts -> preview or new revision
```

## Engine Features

| Feature | Use it for | Current owner |
| --- | --- | --- |
| `GameSurface` | Expose explicit-time observation, journal reads, revision-checked authoring, and speculative branch lifecycle to tools and agents. | [`engine-surface`](crates/engine-surface) |
| `LogicalTime` and `Tau` | Keep authoritative time distinct from presentation time. | [`engine-time`](crates/engine-time) |
| `Context`, `Journal`, `Worldline`, `GameState`, and `Frame` | Carry immutable game-owned values through the engine boundaries. | [`engine-sdk`](crates/engine-sdk), [`engine-branches`](crates/engine-branches) |
| `JournalWriter` | Assign monotonic logical timestamps and publish immutable journal snapshots. | [`engine-journal`](crates/engine-journal) |
| `IndexedQuery` and `state` | Reconstruct a complete game state at any requested logical time. | [`engine-index`](crates/engine-index) |
| `Branch` | Produce actual, counterfactual, and corrected immutable histories. | [`engine-branches`](crates/engine-branches) |
| `Renderer` and `present` | Project one `GameState` plus one `Tau` into owned output. | [`engine-presentation`](crates/engine-presentation) |
| `RenderBatch` | Carry target-neutral triangle draw data to a host render sink. | `engine-presentation` and [`nworlds-host`](crates/nworlds-host) |
| `engine-observation` | Sample any `RenderSource` at explicit times and produce deterministic PNG metadata and image bytes. | [`engine-observation`](crates/engine-observation) |
| `engine-observation-mcp` | Expose generic observation and GameSurface operations through MCP stdio tools. | [`engine-observation-mcp`](crates/engine-observation-mcp) |
| `engine-controls` | Map typed screen input to two time sliders, four directional steps, automatic/manual mode, fixed-focus parabolic time reprojection, viewport layout scaling, and owned control geometry. | [`engine-controls`](crates/engine-controls) |
| `GamePackage` and host ports | Connect game meaning to input, storage, lifecycle, and rendering without exposing target types. | [`nworlds-host`](crates/nworlds-host) |

## Build A Game

### 1. Define a query and publish facts

The game supplies context, fact payloads, and an `IndexedQuery`. The query
receives immutable inputs and returns a complete result. This small example
uses byte facts to show the API without a game-specific rule system:

```rust
use engine_api::{Branch, Context, IndexedQuery, Journal, LogicalTime, QueryInput};

struct Facts;

impl IndexedQuery<(), u8> for Facts {
  type Result = Vec<u8>;

  fn query(&self, input: QueryInput<'_, (), u8>) -> Self::Result {
    input.visible_entries().map(|entry| *entry.payload()).collect()
  }
}

fn main() -> Result<(), engine_api::BranchError> {
  let worldline = Branch::new(Context::new(()), Journal::empty());
  let future = engine_api::state(
    worldline.context(),
    worldline.journal(),
    LogicalTime::from_ticks(100),
    Facts,
  );
  let past = engine_api::state(worldline.context(), worldline.journal(), LogicalTime::zero(), Facts);
  assert!(future.payload().is_empty());
  assert!(past.payload().is_empty());
  Ok(())
}
```

The direct query path is the low-level engine contract. `GameSurface` composes
that path with manifest, journal, branch, preview, commit, and discard tools.

### 2. Observe and author through GameSurface

`GameSurface::observe` accepts an explicit branch, `LogicalTime`, and `Tau`.
Each call samples immutable history directly. The voxel package queries the
selected branch through this surface for both interaction and presentation.

Journal authoring is revision-checked. `branch_preview_append` validates facts
against a speculative branch without changing it; `branch_append` commits to a
speculative branch; and `actual_append` is the visibly separate actual-line
write. Each commit returns a new revision. Branch discard restores the actual
line without reconstructing it.

### 3. Use lower-level branches and codecs when needed

Lower-level `Worldline` and branch APIs remain available inside a game surface.
The MCP-facing path names branch handles, fork boundaries, and revisions while
the engine preserves immutable parent history and inclusive-prefix rules.

### Guarantees and costs

`GameSurface` owns the agent-facing session boundary while the engine preserves
immutable history and branch-prefix rules. Game rule bodies, renderers, and
payloads are trusted Rust extension points; ordinary owned data and
deterministic rules provide the intended game authoring discipline.

Publication currently rebuilds a journal snapshot, with work proportional to
history plus new facts. Direct sampling also retains the existing query's
cost. The sample uses direct sampling to keep synchronization obligations out
of the application path; performance remains a separate measured concern.

### 4. Project state into presentation

Implement `Renderer<S>` for the game state and return an owned output. The
renderer receives the selected `GameState` and `Tau` as its semantic input.

The [voxel renderer](crates/voxel-sample/src/engine_integration.rs) implements
`Renderer<VoxelState>` with `Output = RenderBatch`. It demonstrates both the
generic renderer with a fixed camera and the sample's explicit camera/control
projection. The [package](crates/voxel-sample/src/package.rs) samples its
immutable `Worldline` and passes that complete value into the projection.

The sample renderer uses `Tau` for animation while `LogicalTime` selects the
authoritative state. `RenderBatch` is disposable draw intent containing the
vertices and colors required for the current frame. Journal history, input
transport, device resources, and host scheduling remain owned by their
respective layers.

### Observe a render in Chat

The reusable `engine-observation` boundary accepts any source that produces a
`Frame<RenderBatch>` for an explicit `LogicalTime` and `Tau`. It rasterizes the
same target-neutral triangle vocabulary into PNG bytes and reports the sampled
times, dimensions, vertex count, and triangle count alongside the image.

The workspace MCP configuration connects VS Code Chat to the voxel adapter:

```text
metadata(logical_time_ticks, tau_ticks, width, height)
snapshot(logical_time_ticks, tau_ticks, width, height)
```

The source is generic; voxel supplies its worldline, camera, and query. The
same MCP server can observe another game by replacing that source adapter. PNG
snapshots are the first visual artifact; frame sequences and video can build on
the same explicit-time source later.

### View state and animation instances

Not every value that affects a frame belongs in `GameState`. A camera, viewport,
or animation phase is presentation state. Keep it in an explicit package or
orchestrator value and pass a snapshot into a pure client projection:

```rust
fn project(
  state: &GameState<MyState>,
  view: &MyView,
  tau: Tau,
) -> RenderBatch
```

The projection is deterministic for equal inputs and receives the view snapshot
alongside the selected state and `Tau`. Its result is independent of package
fields, global clocks, device state, and prior frames.

An animation instance is an explicit value with its own local presentation
coordinate and immutable parameters. A pure sampling function turns that value
and the selected `GameState` into presentation data. Several animations can
carry independent local `Tau` values in explicit presentation control state.

A camera is a view value by default, not an animation merely because it affects
projection. An animated camera can be an animation instance whose pure sample
returns a camera pose. Interactive orbit and zoom normally produce a new
presentation-only camera value; they become journal facts only when camera
movement is part of game meaning. The Voxel sample's
`render_batch_at(state, camera, tau)` function demonstrates this explicit pure
projection.

### 5. Connect the game to the host

A target-neutral game package implements `nworlds_host::GamePackage`:

- `declaration()` describes package identity, persistence schema, host version,
  and render vocabulary requirements.
- `ingest_batch()` accepts the package's normalized input value.
- `update()` interprets input and publishes or selects immutable game values.
- `present()` returns the frame for the currently selected complete state.
- `save_selected()` and `load_selected()` own the game codec boundary while
  the host transports encoded bytes.

Compose it with `ApplicationHost<P, I, S, R>` when using the generic host ports:

```text
InputIngress -> GamePackage::ingest_batch -> update
GamePackage::present -> RenderSink<Frame>
GamePackage save/load -> StorageTransport<Vec<u8>>
```

The current native desktop composition is in
[`nworlds-desktop`](crates/nworlds-desktop). It owns window, device, backend,
and native event details. The game package remains target neutral.

### 6. Add timeline controls

Use `engine-controls::TimelineControls` when a game needs screen controls for
the two time axes. Configure fixed-focus parabolic projection horizons and
fixed automatic/manual step deltas using `LogicalTimeDelta` and `TauDelta`,
then pass `ScreenPoint` observations through a `Viewport` to `pointer_down`,
`pointer_move`, and `pointer_up`:

```text
TimelineControls
  automatic mode by default
  slider or step input -> Manual mode
  pointer outside controls -> World + Automatic mode
```

On an automatic update, call `advance_automatic`. If it changes
`LogicalTime`, query a new complete `GameState`; a `Tau` change only affects
presentation. Render the returned control geometry alongside the game batch.
The desktop host applies one package update/presentation step per redraw; input
ingestion itself does not advance either time axis.
Finite absolute times remain inside the slider edges through the fixed-focus
parabolic projection; the slider represents a view of time while gameplay
meaning remains in the selected logical sample. Call
`TimelineLayout::auto_scale(viewport)` or `TimelineControls::with_viewport(viewport)`
whenever the viewport changes. The library owns timeline geometry and control
values. The default `SliderFocus` is `350/1000`, so the
currently focused time sits at 35% of each track with extra room ahead of it.
The voxel sample demonstrates the complete adapter in
[`package.rs`](crates/voxel-sample/src/package.rs) and
[`engine_integration.rs`](crates/voxel-sample/src/engine_integration.rs).

## Input And Persistence

Input is a game-facing value pipeline whose accepted transformations become
journal or branch publications:

```text
native event
  -> PlatformInputAdapter
  -> InputIngress
  -> ordered package batch
  -> game interaction logic
  -> Transformation or game action
  -> JournalWriter or branch publication
```

The host translates native events and transports packets. The game decides what
they mean and which accepted result becomes authoritative. See the
[semantic input proposal](proposals/input-and-interaction.md) and the
[transport proposal](proposals/transport-and-journal.md) for the current
boundaries.

Persistence has the same ownership split. The game encodes and decodes its
context and facts; `StorageTransport` moves owned bytes. `GameState` and render
production carry game values and presentation data, while file paths, device
handles, and backend types stay with the host.

## Ownership Rules

Keep these rules visible while adding a feature:

- Put definitions, facts, rules, state derivation, interaction meaning, and
  persistence codecs in the game package.
- Put time types, immutable envelopes, journal mechanics, direct query
  preparation, branch construction, and state-first presentation in the engine.
- Put native event translation, input transport, byte transport, lifecycle, and
  backend execution in the host.
- Query complete state directly from the selected worldline and logical time.
- Keep selected values, view state, and presentation clocks as explicit
  orchestration values alongside `GameState`.
- Keep automatic/manual playback mode and screen-control drag state in
  presentation control values; authoritative facts carry game meaning.
- Carry physical pixels, normalized clip-space coordinates, absolute times, and
  time deltas in their named unit types across the controls boundary.
- Treat render output as downstream and disposable; interaction logic reads the
  selected `GameState`.
- Use the facade in `engine-api` and promote a lower-level crate only when a
  concrete consumer needs it.

## Further Reading

- [Initial specification](spec/initial.md) defines the temporal vocabulary and
  invariants.
- [Library contract](proposals/library-contract.md) defines crate ownership and
  dependency direction.
- [Rendering contract](proposals/rendering-contract.md) defines the current
  state-first presentation boundary.
- [Presentation host](proposals/presentation-host.md) defines target-neutral
  ports and native execution responsibilities.
- [Controls library](proposals/controls-library.md) defines the reusable
  timeline controls and automatic/manual lifecycle.
- [Index](index.md) maps the full workspace, evidence, and commands.
