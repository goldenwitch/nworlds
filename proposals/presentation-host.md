# Presentation Host

This proposal defines the reusable presentation-host plumbing around an
application-owned canonical Stage. It supplies the environmental adapters for
operating systems, windowing libraries, devices, render backends, and host
scheduling while keeping those details at the host boundary.

The reusable temporal library contract is defined in
[library-contract.md](library-contract.md). This proposal owns the adjacent
host-port boundary; Stage, Orchestrator, and target-specific entrypoints are
reference/application compositions and are not generic engine APIs.

A **target-specific entrypoint** is the OS- or runtime-specific executable
composition root minted or selected by the nworlds target factory. It
constructs a Stage and a selected set of independent presentation-host ports.
A target may group those ports in a local `ApplicationHost` convenience value.
The target entrypoint is host-owned composition; the game package supplies
Stage and game meaning. Stage selects game time and authoritative state, while
the presentation host supplies input, storage, lifecycle, and render ports.

## Vocabulary

This proposal uses **bold** for conceptual vocabulary and backticks for exact
Rust/API spellings. **Target-specific entrypoint** means the executable
composition root that varies by OS or runtime. **Presentation host** means the
reusable passive plumbing boundary made of independent ports. **Input ingress**
means the transport boundary that receives abstract input packets. **Port**
means a Stage-facing capability; **adapter** means a concrete implementation of
one port. **Stage**,
**Orchestrator**, **worldline**, **Tau**, **LogicalTime**, **game state**, and
**frame** retain the meanings owned by the Stage and initial specifications.

## Boundary

```text
Target-specific entrypoint
    constructs concrete Stage/Orchestrator and independent host ports
    may group them in a local ApplicationHost convenience value

Presentation host
    independent ports
    input ingress
    render sink
    storage transport
    platform lifecycle/resource plumbing

    are composed with and called by

Stage
    canonical logical game experience
    Orchestrator
    selected Worldline and branch
    Tau/LogicalTime policy
    input interpretation
    journal publication
    rendering composition
```

The entrypoint and presentation host own the environment. **Stage** owns the
meaning of the experience. The game **context** remains a separate game-model
value carried by the worldline; host ports are not context.

## Adapter Set

The host boundary has several distinct adapters. A **port** is the narrow
Stage-facing capability; an **adapter** is the platform-specific implementation
behind that port. Translation, transport, execution, and storage are separate
responsibilities even when one concrete application type composes them
together.

### Platform input adapter

The platform input adapter translates native operating-system or device events
into the application's closed `InputPacket` vocabulary:

```text
native event
    -> PlatformInputAdapter
    -> InputPacket
```

It may normalize platform identity and select the device details represented by
the application's abstract packet vocabulary.

### Input ingress adapter

The input ingress transports already translated packets to the Orchestrator:

```text
InputPacket
    -> InputIngress
    -> Orchestrator
```

Its queueing is transport buffering; Stage/Orchestrator owns packet retention,
flush, consume, and expiry. The first concrete ingress may be in-memory and
carries abstract packets.

### Storage transport adapter

The game-facing persistence composition decides which immutable worldline or
save value to retain and uses a simple codec to produce an encoded record. The
presentation-host storage port transports that record as bytes:

```text
Worldline/save value
    -> game-facing persistence codec
    -> encoded bytes
    -> storage transport port
    -> host file, database, or other storage
```

The Orchestrator owns save/load decisions. The codec owns the semantic
worldline-to-record boundary; the host adapter owns byte transport.
Loading returns bytes to the game-facing persistence composition, which decodes
a new immutable worldline value.

### Render execution adapter

The render execution adapter consumes Stage-produced render output and performs
backend or surface work:

```text
Frame<RenderBatch>
    -> RenderSinkAdapter
    -> backend/device/surface
```

The generic proof boundary is the `Renderer<S>::render(GameState<S>, Tau) ->
RenderBatch` result carried by `Frame<RenderBatch>`; this adapter owns only the
backend/device/surface execution step. A client may retain a semantic
intermediate such as Caravan's `RenderOutput`, but it must project that value
to the shared batch before crossing the host boundary.

### Platform lifecycle and resource adapters

Lifecycle and resource adapters cover window/surface availability, device
lifecycle, display integration, and host-owned resource loading. They report
platform conditions and perform platform work.

The concrete backend adapter for a `wgpu` composition belongs behind the render
execution boundary. Stage, `GameState`, `InputPacket`, and backend-neutral
render output use the target-neutral side of that boundary.

The adapter roles are summarized here:

```text
PlatformInputAdapter       native events -> InputPacket
InputIngress               InputPacket transport -> Orchestrator
Storage transport          encoded bytes <-> host storage
RenderSinkAdapter           Frame<RenderBatch> -> backend execution
LifecycleResourceAdapter   platform lifecycle/resources -> host conditions
```

The target-factory ownership and target-neutral developer workflow are recorded
in [target-factory.md](target-factory.md). The support-matrix axes and target cells are recorded separately in the
[platform support matrix](platform-support-matrix.md). This proposal defines
where adaptation lives. The first native row is Windows
(`x86_64-pc-windows-msvc`) with `winit` lifecycle/input plumbing and a `wgpu`
render sink; additional platform rows remain pending in that matrix.

The first-proof composition is implemented by the generic desktop host.
The public target-minting workflow is owned by the
target-factory proposal; this host proposal owns the runtime port boundary.

## Host Responsibilities

The target-specific entrypoint and its **presentation host** plumbing are
responsible for:

- operating-system lifecycle and shutdown;
- platform input acquisition and translation into the **input ingress**;
- window, surface, display, and presentation integration;
- hardware, device, and backend lifecycle;
- resource loading and device-facing resource ownership;
- submission of Stage-produced render output to the selected backend; and
- platform file, storage, and persistence I/O facilities.

These responsibilities are plumbing. They can be replaced while the reference
game's Stage retains its meanings for worldline, branch, logical time, input
packet, transformation, game state, and frame.

## Ownership

Stage and Orchestrator ownership is defined in
[stage-layer.md](stage-layer.md) and
[input-and-interaction.md](input-and-interaction.md). Host ports supply the
platform mechanisms used by those decisions. Stage selects worldlines,
interprets packets, assigns journal time, and defines game values.

## Control Flow

The Orchestrator owns semantic control flow and pulls from independent host
ports. A target-specific entrypoint owns process and native runtime bootstrap;
the Orchestrator drives game decisions through its chosen loop shape. An
Orchestrator may run a literal loop, a replay driver, or another
application-specific shape, and calls a port when it needs input, storage, or
backend work.

Host activity is not game time:

```text
Orchestrator control flow
    -> drains input ingress or calls a narrow host port
    -> receives a platform value or performs platform work
```

The first host boundary is scheduler-independent. The Orchestrator selects
`Tau`, while host scheduling supplies opportunities to run the composition.

Explicit samples remain valid independently of host execution:

```text
stage.present_at(logical_time, tau)
```

This keeps replay, scrubbing, tests, and deterministic presentation independent
of a host scheduler.

## Input Ingress

**Input ingress** is the boundary where translated platform observations become
abstract `InputPacket` values available to the Orchestrator:

```text
platform event
    -> platform input adapter
    -> InputIngress
    -> Orchestrator drains packets
    -> SemanticInputBatch
```

The ingress transports packets. `Tau`, `LogicalTime`, packet-set meaning, and
journal authoring remain in the Stage/Orchestrator path; ingress queueing is
transport buffering while the Orchestrator constructs the packet set it gives
to `InteractionDefinition`. Reusable identity, ordering, duplicate handling,
and normalization semantics are owned by the
[transport and journal layer](transport-and-journal.md).

## Input Crossing

The host-side input crossing ends when translated packets reach `InputIngress`.
The packet-set, interaction, and journal-publication semantics are owned by
[input-and-interaction.md](input-and-interaction.md) and the Stage/Orchestrator
composition. The host normalizes native detail; Stage assigns packet meaning
and retains semantic input state.

## Rendering Crossing

The generic Stage-side `GameState + Tau -> Frame<RenderBatch>` composition is
owned by the completed rendering contract on top of the generic renderer/frame
boundary. Gameplay-specific presentation may retain a semantic intermediate,
and the minimal batch projected from `GameState` crosses to the host; the host
owns device execution:

```text
Frame<RenderBatch>
    -> RenderSink port
    -> target backend/surface
```

The rendering contract defines the shared `RenderBatch` representation as
owned normalized triangle data. A client-specific semantic projection remains
opaque to generic host reasoning; backend commands, device state, and surface
work remain below this boundary.

Frame history and persistent device simulation belong to the target execution
layer. GPU or backend state is plumbing around the indexed query and Stage
journal path.

## Persistence Crossing

The Orchestrator decides which immutable values to save and when. The
game-facing persistence composition applies a simple codec; the host provides
the byte transport environment needed to perform I/O:

```text
Orchestrator selects worldline
    -> game-facing persistence codec
    -> encoded bytes
    -> storage transport port
    -> target storage
```

The storage adapter transports bytes. The game-facing persistence composition
decodes them into a new immutable worldline value.

## Static Composition

The preferred implementation is ordinary Rust static composition:

```rust
struct ApplicationHost<S, InputIngress, RenderSink, StorageTransport> {
    stage: S,
    input_ingress: InputIngress,
    render_sink: RenderSink,
    storage: StorageTransport,
}
```

This is a target-local convenience sketch for composing independent ports.
Concrete entrypoints compose Stage with those ports as early as practical.
Traits and generics belong at real substitution boundaries such as input
transport, render sink, or storage variation. The composition remains ordinary
static Rust around independent typed ports.

The first executable host composition uses in-memory adapters to prove the
crossings independently of a platform:

```text
MemoryInputIngress
MemoryStorage
CollectingRenderSink
    -> ApplicationHost<CaravanStage, ...>
```

This composition is test infrastructure and a wiring proof. The selected
Windows composition adds native input, windowing,
device resources, and `wgpu` behind the same ports and leaves the game-facing
Stage and renderer unchanged.

The target-factory selects a generated static composition crate as the public
mechanism. That crate owns the target entrypoint and composes the package with
these ports; the package source tree contributes the game package while the
generated composition contributes target `main` and platform adapters. The
existing native executable remains historical proof of the port behavior.

## Reusable Desktop Composition

The reusable desktop composition is ordinary static Rust around the
target-neutral host contract. A generated composition supplies concrete type
arguments and constructors for:

```text
GamePackage
InputIngress
PlatformInputAdapter<NativeEvent, Packet>
StorageTransport
RenderSink<Frame<RenderBatch>>
```

The generated composition owns the `winit` application handler and event loop,
window creation, native event delivery to the package-defined input adapter,
resize handling, `wgpu` surface/device/queue setup, surface-loss recovery,
shutdown, redraw scheduling, and final frame submission. Its control order is
target-local and mechanical:

```text
native event
    -> PlatformInputAdapter -> InputIngress
    -> ApplicationHost::update
    -> GamePackage::ingest_batch / update

native redraw
    -> ApplicationHost::present
    -> GamePackage::present
    -> RenderSink<Frame<RenderBatch>>
```

The desktop host supplies lifecycle, input transport, storage, and render
execution around the package. Stage selects the worldline, assigns logical
time, interprets facts, performs game-specific picking, and presents package
state. Host time may drive redraw scheduling while the package retains its
authoritative time path.

Package clients supply the `GamePackage` implementation, semantic packet and
batch types, the native-event-to-packet adapter used by their generated
composition, and the `GameState + Tau -> Frame<RenderBatch>` projection. The
selected target composition supplies the concrete lifecycle, storage, and
backend adapters. A client may add target-neutral semantic observations such
as cursor coordinates; target execution still treats them as opaque package
packets.

The target may redraw continuously. Redraw is presentation demand: a package
update may publish or select a new immutable worldline/state, and package
presentation reads the currently selected complete state and visual `Tau`.
Host redraw frequency remains a scheduling input around that explicit state.

`nworlds-desktop` now contains the reusable generic lifecycle and a synthetic
package compile proof using generic host dependencies. The historical
Caravan desktop proof remains independently preserved as client evidence, and
both Caravan and voxel samples now have package-owned compositions through
this host. Native launch evidence and voxel persistence remain separate
follow-up claims.

## Time Boundary

Stage and the package use these explicit time values:

```text
Tau          Stage presentation sample
LogicalTime  indexed game-state sample
Journal time JournalWriter-assigned authoring timestamp
```

Host scheduling surrounds these values through redraw and lifecycle
opportunities; the presentation-host contract remains centered on Stage time.

## Current Scope

The selected first target is Windows (`x86_64-pc-windows-msvc`) with `winit`
and `wgpu`, as recorded in the platform matrix and completed host graph. The
current host packet leaves these selections for later packets or their owning
proposals:

- file-backed storage and a shipped persistence workflow;
- host scheduling and frame-pacing policy beyond the current redraw loop;
- device-loss recovery and automated GPU/device acceptance;
- raw input packet variants;
- camera, HUD, widgets, and interactable objects;
- render scene and coordinate projection;
- runtime plugin/object-typing model; and
- additional target profiles.

## Open Questions

1. What storage transport should follow `MemoryStorage` for the Windows target?
2. Which host scheduling and frame-pacing requirements bind beyond the current
    redraw loop while Stage time remains authoritative?
3. Which device-loss and GPU acceptance observations should become automated?
4. Which host/Stage patterns repeat enough to extract into reusable engine APIs?
