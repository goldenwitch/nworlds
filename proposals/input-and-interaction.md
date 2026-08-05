# Input and Interaction Boundary

This proposal records the smallest Stage-facing input boundary currently
agreed. It keeps input abstract and value-producing; it does not define an OS
event model, a device protocol, or journal authoring commands.

## Two Concepts

The input boundary contains two distinct concepts:

1. `InputPacket` values are abstract observations supplied by the host.
2. An `InteractionDefinition` is developer-authored Stage logic describing how
   to reason over a set of those packets.

The host supplies data. The developer supplies interpretation. Neither concept
is an authoritative journal fact.

`PresentationHost` acquires platform input, converts it to abstract packets,
and presents a packet set to Stage. Stage does not receive operating-system
events, device objects, host-clock values, or backend-specific coordinates as
part of this boundary.

```text
PresentationHost
  platform input
    -> InputPacketSet

Stage
  InteractionDefinition
  InputPacketSet
    -> pure interaction query
    -> Transformation
```

An `InputPacket` is data for one abstract input observation. An
`InputPacketSet` is the finite set of packets supplied to one interaction
query. The packet vocabulary and its platform-neutral coordinate model remain
open.

An `InteractionDefinition` is the place where the developer writes interaction
logic. It is a Stage-owned dependency composed statically with the concrete
game Stage. It is not a packet, a host callback, a renderer, or an automatic
journal mutation.

## Pure Set Query

The interaction operation is a pure set operation over a selected sample:

```text
InteractionDefinition
  x InputPacketSet
  x Tau
  x LogicalTime
  -> Transformation
```

The canonical query boundary is:

```text
interaction_query(definition, packets, tau, logical_time)
  -> Transformation
```

The definition and packet set are both present for every call. `Tau` identifies
the presentation sample and `LogicalTime` identifies the authoritative game
sample. The Stage supplies the logical time through its owned `Playback`
policy; the definition remains part of the Stage's static composition.

The query does not inspect a host clock, mutate a worldline, append a journal
entry, or depend on a previous query. A sample in the past, at the present, or
in the future follows exactly the same path. A pointer pick or raycast is an
ordinary query against the selected sample, not a special past/future API.

The word `set` is intentional: set semantics do not provide packet ordering or
duplicate identical values. If a later interaction requires either property,
the packet vocabulary must carry explicit identity or ordering, or a different
collection contract must be chosen. Iteration order must not become an
accidental part of interaction meaning.

## Packet-Set Construction

The interaction boundary is transparent to how its packet set was constructed.
Stage may receive packets directly for one call, or it may retain packets across
calls and combine them with newly supplied packets. These are implementation
strategies for constructing an `InputPacketSet`, not different interaction
APIs.

The labels **unbuffered input** and **buffered input** may describe those two
Stage behaviors internally:

- unbuffered construction does not retain packets for a later call;
- buffered construction retains packets as part of Stage game logic and
  temporal orchestration.

Neither label is visible to `InteractionDefinition` or
`interaction_query`. The query receives only the resulting packet set, `Tau`,
and `LogicalTime`. If two construction strategies produce equal packet sets for
the same selected sample, they must produce equal interaction results. If a
retained packet remains in the set, its presence may affect gameplay; that is a
property of the packet-set contents, not a mode flag.

The host may queue platform events as plumbing, but it does not decide which
abstract packets remain semantically active for a Stage query. Retention,
flush, consume, and expiry are part of Stage's game logic and orchestration,
not a separate packet-policy dependency. The pure interaction query still
receives an explicit set rather than hidden input history, and buffering does
not make input authoritative game state.

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

This boundary introduces no new input clock. It does not define `HostTime` or
`InputTime`. Host event timing may matter to the outer plumbing, but it is not
part of the Stage interaction-query contract.

The relevant selected values are the existing presentation and game values:

```text
Tau          presentation sample
LogicalTime  authoritative game sample
GameState    authoritative result at LogicalTime
Frame        presentation result at Tau
```

The canonical query takes `Tau` and `LogicalTime` explicitly. It does not take
an SDK `Frame`; host time is not authoritative game time.

## Composition

The Stage owns the logical composition of the input query with its selected
worldline and playback policy:

```text
Stage
    Worldline
    Playback
  Orchestrator
    Renderer
```

`QueryAdapter` remains the state-query dependency. It answers the selected
`Worldline` and `LogicalTime` query; it does not interpret input packets or
author input facts. `InteractionDefinition` is a current seam inside the
Orchestrator, not a second source of authoritative state.

`PresentationHost` remains plumbing. It acquires platform input, converts it
to abstract `InputPacket` values, and executes the Stage's resulting logical
outputs in its environment.

## Non-Goals

This proposal does not:

- define raw OS, mouse, keyboard, controller, touch, or device events;
- define host or input timestamps;
- choose the concrete packet set representation or packet identity rules;
- define a concrete raycast, coordinate, widget, or menu model;
- assign camera or HUD ownership;
- define a universal interactable-description object;
- define concrete transformation-admission or branch-command types; or
- change `spec/initial.md`, `Worldline`, `LogicalTime`, `Tau`, `GameState`, or
  `Frame`.

## Open Questions

1. What is the smallest `InputPacket` and `Transformation` vocabulary?
2. What are the explicit lifetime, flush, consume, and expiry rules for
  buffered packet sets?
3. Which transformations are Stage-local view operations, journal facts, or branch
   operations?
