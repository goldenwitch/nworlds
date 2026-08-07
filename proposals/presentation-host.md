# Presentation Host

This proposal defines the platform-facing host boundary around a canonical
Stage. It is intentionally agnostic about operating systems, windowing
libraries, devices, render backends, and host clock types.

The **application host** is the executable composition root. It constructs a
Stage, its Orchestrator, and narrow host ports. The **presentation host** is the
platform plumbing behind those ports. Neither defines the game, owns game
time, interprets input, or becomes a second source of authoritative state.

## Vocabulary

This proposal uses **bold** for conceptual vocabulary and backticks for exact
Rust/API spellings. **Application host** means the executable composition root.
**Presentation host** means the passive platform plumbing behind narrow ports;
`PresentationHost` is only a possible prototype/API spelling. **Input
ingress** means the conceptual interrupt boundary that receives abstract input
packets; `InputChannel` is one possible implementation. **Stage**,
**Orchestrator**, **worldline**, **Tau**, **LogicalTime**, **game state**, and
**frame** retain the meanings owned by the Stage and initial specifications.

## Boundary

```text
Application host
    constructs concrete Stage/Orchestrator and narrow host ports

Presentation host ports
    input ingress
    render sink
    storage
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

The host owns the environment. **Stage** owns the meaning of the experience.
The game **context** remains a separate game-model value carried by the
worldline; host ports are not context.

## Host Responsibilities

The **application host** and its **presentation host** plumbing are responsible
for:

- operating-system lifecycle and shutdown;
- platform input acquisition and translation into the **input ingress**;
- window, surface, display, and presentation integration;
- hardware, device, and backend lifecycle;
- resource loading and device-facing resource ownership;
- submission of Stage-produced render output to the selected backend; and
- platform file, storage, and persistence I/O facilities.

These responsibilities are plumbing. They should be replaceable without
changing what the Caravan Stage means by a worldline, branch, logical time,
input packet, transformation, game state, or frame.

## Host Non-Responsibilities

The host does not decide:

- which `Worldline` or branch the Stage is viewing;
- which independent `LogicalTime` and `Tau` samples the Stage selects;
- whether presentation time advances, pauses, reverses, or scrubs;
- which input packets are retained or consumed by Stage orchestration;
- what an `InputPacketSet` means for the game;
- whether an interaction produces a `Transformation`;
- whether a transformation is admitted to the journal or a branch;
- what timestamp the `JournalWriter` assigns;
- what a Caravan `GameState` means; or
- which values the Orchestrator chooses to save or present.

The host ports provide mechanisms for those decisions. They do not own the
decisions.

## Control Flow

The Orchestrator owns control flow. There is no host-owned outer pump in this
boundary. An Orchestrator may run a literal loop, a pull loop over narrow host
ports, a replay driver, or another application-specific control shape. The
Orchestrator calls a port when it needs input, storage, or backend work.

Host activity is not game time:

```text
Orchestrator control flow
    -> drains input ingress or calls a narrow host port
    -> receives a platform value or performs platform work
```

The first host boundary does not require a fixed loop frequency, a static clock,
or a host-defined `Tau`. No clock port is part of this first host composition.

Explicit samples remain valid independently of host execution:

```text
stage.present_at(logical_time, tau)
```

This keeps replay, scrubbing, tests, and deterministic presentation independent
of a host scheduler.

## Input Ingress

**Input ingress** is the conceptual boundary where platform interrupts become
abstract `InputPacket` values available to the Orchestrator. A channel is one
implementation:

```text
platform interrupt/event
    -> platform adapter
    -> InputIngress / InputChannel
    -> Orchestrator drains packets
    -> InputPacketSet
```

The ingress is transport plumbing. It does not interpret packets, choose
`Tau`, select `LogicalTime`, or create journal entries. Its buffering is not
the Stage's semantic buffered/unbuffered behavior; the Orchestrator constructs
the packet set it gives to `InteractionDefinition`.

## Input Crossing

The input path has one host conversion and one Stage interpretation boundary:

```text
platform event
    -> InputIngress
    -> Orchestrator packet-set orchestration
    -> InputPacketSet
    -> selected GameState at LogicalTime with Tau
    -> InteractionDefinition
    -> Transformation
```

The platform adapter may normalize device-specific details such as button
identity or pointer representation into the application's closed `InputPacket`
type. It must not decide the packet's game meaning or directly construct a
journal entry.

Buffered and unbuffered packet construction is Stage behavior. The host may
queue platform events internally, but the semantic packet set passed to
`InteractionDefinition` is constructed by the Stage/Orchestrator.

## Rendering Crossing

Stage owns the logical renderer composition:

```text
GameState + Tau
    -> Renderer
    -> Frame or backend-neutral render output
```

The host owns device execution:

```text
Frame/render output
    -> PresentationHost backend/surface
```

The first host proposal does not choose whether the Stage renderer returns a
fully backend-neutral scene, a compact render value, or another owned output.
That decision belongs to the rendering proposal. The host only consumes the
selected output through an application-composed backend.

The host must not add frame history or a persistent device simulation that
changes authoritative game state. GPU or backend state may exist as plumbing,
but it is not a replacement for the indexed query or Stage journal path.

## Persistence Crossing

The Orchestrator decides which immutable values to save and when. The host
provides the environment needed to perform I/O:

```text
Orchestrator selects worldline
    -> persistence encoding
    -> host-provided path/storage capability
```

A host path or storage handle must not become an alternate authority for the
journal. Loading produces a new immutable worldline value; it does not mutate a
currently published worldline in place.

## Static Composition

The preferred implementation is ordinary Rust static composition:

```rust
struct ApplicationHost<S, InputIngress, RenderSink, Storage> {
    stage: S,
    input_ingress: InputIngress,
    render_sink: RenderSink,
    storage: Storage,
}
```

This is a boundary sketch, not an immediate engine type. Concrete application
host and Stage implementations should be composed as early as practical.
Traits and generics belong at real substitution boundaries such as input
transport, render sink, or storage variation; the host does not need a runtime
dependency injection container or a broad capabilities object.

## Time Boundary

No clock port is part of the first host composition. If a later Orchestrator
needs a platform timing observation, that is a separate narrow port decision.
Host time is not authoritative game time. The current time values remain:

```text
Tau          Stage presentation sample
LogicalTime  indexed game-state sample
Journal time JournalWriter-assigned authoring timestamp
```

No `HostTime` or `InputTime` type is required by this proposal.

## Non-Goals

This proposal does not:

- define a concrete operating-system or windowing API;
- settle the `wgpu` choice or platform support matrix; those require a separate
    external design gate;
- define a host clock type or fixed update frequency;
- define raw input packet variants;
- define camera, HUD, widgets, or interactable objects;
- define a render scene or coordinate projection;
- add a generic engine-level `PresentationHost` type;
- move `Worldline`, `Journal`, `GameState`, or `Transformation` ownership into
  the host; or
- introduce a runtime plugin/object-typing model.

## Open Questions

1. What is the narrowest input-ingress interface for the first demo?
2. What is the narrowest backend/surface sink for the first render observable?
3. Which storage port is needed by the first Orchestrator demo?
4. Which host/Stage patterns repeat enough to extract into reusable engine APIs?
