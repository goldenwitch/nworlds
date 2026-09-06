# Game Developer Guide

This is the current implementation guide for building a target-neutral game
consumer in this workspace. It describes the generic Rust engine and the host
boundary that exists today. It does not define a new engine contract or imply
that the proposed `nworlds` CLI is shipped.

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

The engine evaluates complete values directly. A game does not keep a hidden
mutable board that is advanced, rewound, or synchronized with rendering.

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

The actual generic query function receives the worldline's context and journal
separately so the game can supply an `IndexedQuery` implementation:

```rust
let sampled = state(
    worldline.context(),
    worldline.journal(),
    logical_time,
    MyQuery,
);
let frame = present::<MyState, MyRenderer>(&sampled, tau);
```

## Engine Features

| Feature | Use it for | Current owner |
| --- | --- | --- |
| `LogicalTime` and `Tau` | Keep authoritative time distinct from presentation time. | [`engine-time`](crates/engine-time) |
| `Context`, `Journal`, `Worldline`, `GameState`, and `Frame` | Carry immutable game-owned values through the engine boundaries. | [`engine-sdk`](crates/engine-sdk), [`engine-branches`](crates/engine-branches) |
| `JournalWriter` | Assign monotonic logical timestamps and publish immutable journal snapshots. | [`engine-journal`](crates/engine-journal) |
| `IndexedQuery` and `state` | Reconstruct a complete game state at any requested logical time. | [`engine-index`](crates/engine-index) |
| `Branch` | Produce actual, counterfactual, and corrected immutable histories. | [`engine-branches`](crates/engine-branches) |
| `Renderer` and `present` | Project one `GameState` plus one `Tau` into owned output. | [`engine-presentation`](crates/engine-presentation) |
| `RenderBatch` | Carry target-neutral triangle draw data to a host render sink. | `engine-presentation` and [`nworlds-host`](crates/nworlds-host) |
| `engine-controls` | Map typed screen input to two time sliders, four directional steps, automatic/manual mode, fixed-focus parabolic time reprojection, viewport layout scaling, and owned control geometry. | [`engine-controls`](crates/engine-controls) |
| `GamePackage` and host ports | Connect game meaning to input, storage, lifecycle, and rendering without exposing target types. | [`nworlds-host`](crates/nworlds-host) |

## Build A Game

### 1. Define game-owned values

A game starts by defining its own context, journal fact vocabulary, and queried
state. These are not engine primitives.

```rust
struct MyContext {
    // Immutable definitions and configuration owned by the game.
}

enum MyFact {
    // Authoritative events such as creation, input, or an action.
}

struct MyState {
    // The complete game result at one LogicalTime.
}
```

The engine never invents a fact or interprets a domain value. The query is the
place where a game turns its context and visible facts into its complete state.

### 2. Author facts through `JournalWriter`

Use `JournalWriter` for game-facing authoring. It owns timestamp assignment and
preserves append order for equal-time facts.

```rust
let mut writer = JournalWriter::<MyFact>::new();
writer.record(MyFact::CreateWorld);
writer.advance_to(LogicalTime::from_ticks(1))?;
writer.record(MyFact::PlaceActor);

let worldline = Branch::new(Context::new(MyContext {}), writer.snapshot());
```

`advance_to` can move the cursor forward or keep it at the same time. Moving
backward is an explicit error. `JournalEntry::from_assigned_time` and direct
journal construction are low-level interoperability paths, not the normal game
API.

### 3. Implement direct state evaluation

Implement `IndexedQuery<C, P>` for the game's query type. It receives an
immutable `QueryInput` containing the context, exact logical time, and journal
entries visible at or before that time.

```rust
struct MyQuery;

impl IndexedQuery<MyContext, MyFact> for MyQuery {
    type Result = MyState;

    fn query(&self, input: QueryInput<'_, MyContext, MyFact>) -> Self::Result {
        let mut state = MyState::default();
        for entry in input.visible_entries() {
            state = state.apply(entry.payload(), entry.logical_time());
        }
        state
    }
}
```

The `apply` operation above is game code. It may derive effects, movement,
resources, or other values from the visible facts, but it should return the
complete result for the selected time rather than mutate a shared board.

Evaluate it with the generic `state` function:

```rust
let at_start = state(worldline.context(), worldline.journal(), LogicalTime::zero(), MyQuery);
let in_the_future = state(
    worldline.context(),
    worldline.journal(),
    LogicalTime::from_ticks(10),
    MyQuery,
);
```

The second query is independent of the first. Sampling backward, repeating a
sample, or querying beyond the latest authored fact follows the same path.

### 4. Publish new immutable values

A new authoritative action produces a new journal snapshot and a new
worldline value. It does not mutate a previously published worldline.

```rust
writer.record(MyFact::PlayerAction);
let next_worldline = Branch::new(
    worldline.context().clone(),
    writer.snapshot(),
);
```

Keep the writer and selected worldline together in the game application layer.
The writer is mutable authoring control; the worldline and every `GameState`
are immutable values.

### 5. Create counterfactual or corrected histories

Use the branch APIs when the parent history must remain available:

```rust
let mut suffix_writer = JournalWriter::<MyFact>::new();
suffix_writer.advance_to(LogicalTime::from_ticks(2))?;
suffix_writer.record(MyFact::AlternateAction);
let suffix = suffix_writer.finish();

let counterfactual = worldline.counterfactual(
    LogicalTime::from_ticks(1),
    &suffix,
)?;
```

A counterfactual keeps the parent's inclusive prefix and adds a strict suffix.
A corrected branch uses `corrected_suffix` with the same boundary rules. Neither
operation rewrites the parent.

### 6. Project state into presentation

Implement `Renderer<S>` for the game state and return an owned output. The
renderer receives only the selected `GameState` and `Tau`.

```rust
struct MyRenderer;

impl Renderer<MyState> for MyRenderer {
    type Output = RenderBatch;

    fn render(state: &GameState<MyState>, tau: Tau) -> Self::Output {
        let vertex = RenderVertex::new(
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0, 1.0],
        );
        let _ = (state, tau);
        RenderBatch::new([vertex, vertex, vertex])
    }
}

let frame = present::<MyState, MyRenderer>(&sampled, Tau::zero());
```

The sample renderer may use `Tau` for animation, but it must not use `Tau` to
select another logical state, publish a fact, or depend on a previous frame.
`RenderBatch` is disposable draw intent: it contains vertices and colors, not a
journal, worldline, input queue, device, or host clock.

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

The projection must be deterministic for equal inputs. It must not read or
mutate package fields, global clocks, device state, or prior frames.

An animation instance is an explicit value with its own local presentation
coordinate and immutable parameters. A pure sampling function turns that value
and the selected `GameState` into presentation data. If several animations
need independent clocks, store their local `Tau` values in explicit
presentation control state. Do not put a hidden mutable clock inside a
renderer.

A camera is a view value by default, not an animation merely because it affects
projection. An animated camera can be an animation instance whose pure sample
returns a camera pose. Interactive orbit and zoom normally produce a new
presentation-only camera value; they become journal facts only when camera
movement is part of game meaning. The Voxel sample's
`render_batch_at(state, camera, tau)` function demonstrates this explicit pure
projection.

### 7. Connect the game to the host

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

### 8. Add timeline controls

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
parabolic projection; the slider is never used as a gameplay bound. Call
`TimelineLayout::auto_scale(viewport)` or `TimelineControls::with_viewport(viewport)`
whenever the viewport changes. The library owns no worldline, journal, host
clock, or game meaning. The default `SliderFocus` is `350/1000`, so the
currently focused time sits at 35% of each track with extra room ahead of it.
The voxel sample demonstrates the complete adapter in
[`package.rs`](crates/voxel-sample/src/package.rs) and
[`engine_integration.rs`](crates/voxel-sample/src/engine_integration.rs).

## Input And Persistence

Input is a game-facing value pipeline, not an automatic journal mutation:

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
context and facts; `StorageTransport` only moves owned bytes. A host file path,
device handle, or backend type must not enter `GameState` or render production.

## Ownership Rules

Keep these rules visible while adding a feature:

- Put definitions, facts, rules, state derivation, interaction meaning, and
  persistence codecs in the game package.
- Put time types, immutable envelopes, journal mechanics, direct query
  preparation, branch construction, and state-first presentation in the engine.
- Put native event translation, input transport, byte transport, lifecycle, and
  backend execution in the host.
- Query complete state directly; do not add a mutable shadow state for speed or
  rendering convenience.
- Mutable orchestration state is allowed for selected values, view state, and
  presentation clocks, but it must remain explicit and never replace
  `GameState`.
- Automatic/manual playback mode and screen-control drag state are explicit
  presentation control state; they do not belong in authoritative facts unless
  the game deliberately makes them game meaning.
- Keep physical pixels, normalized clip-space coordinates, absolute times, and
  time deltas in their named unit types; do not pass bare tuples or raw tick
  integers across the controls boundary.
- Treat render output as downstream and disposable; interaction logic reads
  `GameState`, not a frame.
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
