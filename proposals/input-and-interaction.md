# Input and Interaction Boundary

This proposal records the smallest Stage-facing input boundary currently
agreed. It keeps input abstract and value-producing; it does not define an OS
event model, a device protocol, or journal authoring commands.

The reusable transport and journal semantics are owned by the
[transport and journal layer proposal](transport-and-journal.md). This
proposal owns the current Stage interaction seam and its small prototype
vocabulary.

## Vocabulary

This proposal uses **bold** for conceptual vocabulary and backticks for exact
Rust/API spellings:

> **input packet**: One abstract platform observation supplied to Stage. Its
> prototype/API spelling is `InputPacket`.
>
> **input packet set**: The finite set supplied to one pure interaction query.
> Its current prototype/API spelling is `InputPacketSet`. In the reusable
> transport layer, this is a derived membership view of an ordered semantic
> input batch, not the canonical transport collection.
>
> **semantic input batch**: The ordered payload-only value supplied to the
> game-facing interaction query. Its prototype/API spelling is
> `SemanticInputBatch`; it carries repeated equal payloads and semantic order,
> but no transport identity or delivery metadata.
>
> **interaction definition**: Developer-authored pure logic that interprets an
> selected read-only **GameState** and semantic input batch at a selected
> **Tau**.
> Its
> prototype/API spelling is `InteractionDefinition`.
>
> **transformation**: Closed data returned by an interaction definition. It
> describes a requested Stage or game change without being a timestamped
> journal entry. Its prototype/API spelling is `Transformation`.
>
> **input ingress**: The conceptual boundary where platform interrupts become
> abstract input packets available to the Orchestrator. Concrete transport
> implementations remain target-local presentation-host plumbing.

## Two Concepts

The input boundary contains two distinct concepts:

1. **input packet** values are abstract observations supplied by the host.
2. An **interaction definition** is developer-authored **Stage** logic describing how
   to reason over a set of those packets.

The input ingress supplies data. The developer supplies interpretation. Neither
concept is an authoritative journal fact.

The Orchestrator drains the input ingress, converts packets to the semantic
packet set, and constructs the query input. Stage does not receive
operating-system events, device objects, host-clock values, or
backend-specific coordinates as part of this boundary.

```text
InputIngress
  abstract input packets
    -> Orchestrator
    -> SemanticInputBatch

Stage
  selected GameState at LogicalTime
  InteractionDefinition
  SemanticInputBatch
    -> pure interaction query
    -> Transformation
```

An `InputPacket` is data for one abstract input observation. A
`SemanticInputBatch` is the ordered payload-only value supplied to one
interaction query. The packet vocabulary and its platform-neutral coordinate
model remain open.

An `InteractionDefinition` is the place where the developer writes interaction
logic. It is a Stage-owned dependency composed statically with the concrete
game Stage. It is not a packet, a host callback, a renderer, or an automatic
journal mutation.

## Semantic Interaction Query

The interaction operation is a pure query over an ordered semantic batch and a
selected sample:

```text
InteractionDefinition
  x read-only GameState
  x SemanticInputBatch
  x Tau
  -> Transformation
```

The canonical query boundary is:

```text
interaction_query(definition, state, packets, tau)
  -> Transformation
```

This is the first application seam and remains intentionally small. The
transport layer derives the payload-only batch from identity-bearing transport
observations; the rules for that normalization and for the compatibility
membership view are defined in [transport-and-journal.md](transport-and-journal.md).

The definition, selected read-only `GameState`, and semantic input batch are
present for every call. The Orchestrator queries the selected immutable
`Worldline` at its selected `LogicalTime` and passes that result to the
interaction definition.
The exact logical time is carried by `GameState`; `Tau` selects only
presentation sampling and never selects logical state. The definition remains part of the
Stage's static composition.

The query does not inspect a host clock, mutate a worldline, append a journal
entry, or depend on a previous query. It never inspects `Frame`, rendered
output, `Renderer`, `DrawCommand`, or backend state. A sample in the past, at
the present, or in the future follows exactly the same path. A pointer pick or
raycast is an ordinary query against the selected `GameState`, not a special
past/future API.

The prototype's set semantics remain available as a derived compatibility view
for interactions that only need membership. They do not provide packet ordering
or repeated equal payloads and must not replace `SemanticInputBatch` as the
game-facing canonical value. An interaction that needs press/release order,
repeated actions, or deterministic event replay receives the semantic batch.

## Membership View Construction

The current application boundary is transparent to how a semantic input batch
was constructed. Stage may receive packets directly for one call, or it may
retain packets across calls and combine them with newly supplied packets. These
are orchestration strategies for constructing the semantic batch, not
different interaction APIs. Reusable source identity, ordering, duplicate
handling, and normalization belong to [transport-and-journal.md](transport-and-journal.md).

The labels **unbuffered input** and **buffered input** may describe those two
Stage behaviors internally:

- unbuffered construction does not retain packets for a later call;
- buffered construction retains packets as part of Stage game logic and
  temporal orchestration.

Neither label is visible to `InteractionDefinition` or
`interaction_query`. The query receives the selected read-only `GameState`, the
resulting semantic batch, and `Tau`. If two construction strategies produce
equal semantic batches for the same selected state and `Tau`, they must
produce equal interaction results. If a retained packet remains in the semantic
batch, its presence may affect gameplay; that is a property of the batch
contents, not a mode flag.

The host may queue platform events as plumbing behind the input ingress, but it
does not decide packet meaning or author journal facts. The Orchestrator's
`InputBuffer` captures a semantic batch in an immutable `InputWindow`; its
retain, consume, and discard behavior is defined in
[transport-and-journal.md](transport-and-journal.md). Buffering does not make
input authoritative game state.

## Transformations Without a Magic Interactable Object

There is no required universal interactable object. A `Transformation` is
closed data describing a requested Stage or game change. It can be an ordinary
Stage- or game-specific value, collection, enum, or set of target/action values
appropriate to that definition.

If a future convenience type makes common interaction results easier to write,
it is syntactic sugar over those ordinary values, not a new canonical layer.
A dropdown, focus state, raycast hit, or other UI result is not authoritative
`GameState` merely because it can eventually produce a user command.

Render output and interaction results are sibling Stage-facing values. They may
be derived from the same selected game state and presentation sample, but input
logic must not be hidden inside a renderer or a device backend.

## Authoring Separation

An interaction query produces a closed `Transformation`, not a timestamped
journal entry or an automatic journal mutation. The Stage's `Orchestrator` may
apply the transformation immediately, discard it, or use it for a Stage-local
operation. If it becomes authoritative, the Orchestrator publishes a new
immutable journal/worldline value through the engine APIs and may choose among:

- a Stage-local view operation;
- a journal entry authored through `JournalWriter`;
- a counterfactual or corrected branch operation; or
- rejection or no operation.

The journal remains the only authoritative ingress for game-state change.
`InteractionDefinition` cannot construct timestamped journal entries directly;
the Orchestrator owns admission and the journal/branch machinery owns immutable
publication and timestamp legality.

## Time Boundary

This boundary introduces no input clock. Host event timing is outside the
contract and is not modeled by Stage or the interaction query. It does not
define `HostTime` or `InputTime`.

The relevant selected values are the existing presentation and game values:

```text
Tau          presentation sample
LogicalTime  authoritative game-state sample
GameState    authoritative result at LogicalTime
Frame        presentation result at Tau
```

The canonical interaction query takes the selected `GameState` and `Tau`
explicitly. It does not take an SDK `Frame`; host time is not authoritative
game time.

## Composition

The Stage owns the logical composition of the input query with its selected
worldline and explicit time selections:

```text
Stage
    Worldline
    LogicalTime
    Tau
    Orchestrator
    Renderer
```

The Orchestrator invokes the engine/reference `state(worldline, LogicalTime)`
operation; it does not interpret input packets through the state evaluator or
author input facts there. It passes the read-only result to
`InteractionDefinition`. A failed state projection is an explicit Orchestrator
error and does not invoke interaction logic.
`InteractionDefinition` is a current seam inside the Orchestrator, not a second
source of authoritative state.

The presentation host remains plumbing behind the input ingress and other
narrow host ports. The Orchestrator drains the ingress, converts input to
abstract `InputPacket` values, and owns the resulting logical outputs.

## Non-Goals

This proposal does not:

- define raw OS, mouse, keyboard, controller, touch, or device events;
- define host or input timestamps;
- define source-specific transport envelopes, batch identity, or merge rules;
- define a concrete raycast, coordinate, widget, or menu model;
- assign camera or HUD ownership;
- define a universal interactable-description object;
- define concrete transformation-admission or branch-command types; or
- change `spec/initial.md`, `Worldline`, `LogicalTime`, `Tau`, `GameState`, or
  `Frame`.

## Open Questions

1. What is the smallest `InputPacket` and `Transformation` vocabulary for the
  current Caravan interaction?
2. Which interactions need an ordered batch directly, and which need only a
  membership view?
3. Which transformations are Stage-local view operations, journal facts, or
  branch operations?
