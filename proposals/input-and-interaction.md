# Input and Interaction Boundary

This proposal records the smallest Stage-facing input boundary currently
agreed. It keeps input abstract and value-producing. Native event translation,
device protocols, and journal authoring each have their own owning layer.

The reusable transport and journal semantics are owned by the
[transport and journal layer proposal](transport-and-journal.md). This
proposal owns the current Stage interaction seam and its small prototype
vocabulary.

Cross-boundary ownership is indexed in
[redundancy-register.md](redundancy-register.md). This proposal owns semantic
interaction meaning; reusable identity/order transport is owned by
`transport-and-journal.md`, and target input translation is owned by the
presentation-host boundary.

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
> describes a requested Stage or game change before journal admission assigns
> timestamped history. Its prototype/API spelling is `Transformation`.
>
> **input ingress**: The conceptual boundary where platform interrupts become
> abstract input packets available to the Orchestrator. Concrete transport
> implementations remain target-local presentation-host plumbing.

## Two Concepts

The input boundary contains two distinct concepts:

1. **input packet** values are abstract observations supplied by the host.
2. An **interaction definition** is developer-authored **Stage** logic describing how
   to reason over a set of those packets.

The input ingress supplies data. The developer supplies interpretation. Journal
facts arise after the Orchestrator accepts an interpreted transformation.

The Orchestrator drains the input ingress, converts packets to the semantic
batch, and constructs the query input. The Stage-facing boundary carries
abstract packets, game-facing coordinates, and selected temporal values; native
event objects and device details remain with the host adapter.

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
game Stage. It receives a selected state and semantic input, then returns a
closed transformation for Orchestrator admission.

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
The exact logical time is carried by `GameState`; `Tau` supplies the
presentation sample while `LogicalTime` supplies the game-state sample. The
definition remains part of the Stage's static composition.

The query uses the selected `GameState`, semantic batch, and `Tau` as its
complete input. A sample in the past, at the present, or in the future follows
exactly the same path. A pointer pick or raycast is an ordinary query against
the selected `GameState`, using the same interaction boundary.

The prototype's set semantics remain available as a derived compatibility view
for membership-oriented interactions. `SemanticInputBatch` carries ordering,
repeated equal payloads, press/release sequences, and deterministic replay.
Each interaction chooses the view that matches its meaning.

## Membership View Construction

The current application boundary is transparent to how a semantic input batch
was constructed. Stage may receive packets directly for one call, or it may
retain packets across calls and combine them with newly supplied packets. These
are orchestration strategies for constructing the semantic batch within one
interaction API. Reusable source identity, ordering, duplicate
handling, and normalization belong to [transport-and-journal.md](transport-and-journal.md).

The labels **unbuffered input** and **buffered input** may describe those two
Stage behaviors internally:

- unbuffered construction presents packets for one call;
- buffered construction retains packets as part of Stage game logic and
  temporal orchestration.

The labels remain orchestration details. The query receives the selected
read-only `GameState`, the resulting semantic batch, and `Tau`. Equal semantic
batches for the same selected state and `Tau` produce equal interaction
results. A retained packet affects gameplay through its presence in the batch.

The host may queue platform events as plumbing behind the input ingress. The
Orchestrator's
`InputBuffer` captures a semantic batch in an immutable `InputWindow`; its
retain, consume, and discard behavior is defined in
[transport-and-journal.md](transport-and-journal.md). Buffering remains
orchestration state; journal publication supplies authoritative game state.

## Transformations

The boundary uses closed data rather than a universal interactable object. A
`Transformation` describes a requested Stage or game change. It can be an ordinary
Stage- or game-specific value, collection, enum, or set of target/action values
appropriate to that definition.

If a future convenience type makes common interaction results easier to write,
it remains syntactic sugar over those ordinary values. Dropdowns, focus state,
raycast hits, and other UI results stay in interaction or presentation values;
accepted transformations enter authoritative history through the Orchestrator.

Render output and interaction results are sibling Stage-facing values. They may
be derived from the same selected game state and presentation sample, with
interaction logic in its definition and rendering logic in its projection.

## Authoring Separation

An interaction query produces a closed `Transformation`. The Stage's
`Orchestrator` may apply the transformation immediately, discard it, or use it
for a Stage-local operation. If it becomes authoritative, the Orchestrator
publishes a new immutable journal/worldline value through the engine APIs and
may choose among:

- a Stage-local view operation;
- a journal entry authored through `JournalWriter`;
- a counterfactual or corrected branch operation; or
- rejection or no operation.

Journal publication is the authoritative ingress for game-state change.
`InteractionDefinition` returns untimestamped data; the Orchestrator owns
admission, while journal and branch machinery own immutable publication and
timestamp legality.

## Time Boundary

This boundary uses the existing `LogicalTime` and `Tau` values. Host event
timing remains a delivery concern for the presentation host and has no role in
the interaction query's selected game sample.

The relevant selected values are the existing presentation and game values:

```text
Tau          presentation sample
LogicalTime  authoritative game-state sample
GameState    authoritative result at LogicalTime
Frame        presentation result at Tau
```

The canonical interaction query takes the selected `GameState` and `Tau`
explicitly. Render frames and host scheduling remain downstream presentation
values.

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
operation, passes the read-only result to `InteractionDefinition`, and handles
projection errors before interaction runs. `InteractionDefinition` is a seam
inside the Orchestrator; journal publication remains the authoritative state
path.

The presentation host remains plumbing behind the input ingress and other
narrow host ports. The Orchestrator drains the ingress, converts input to
abstract `InputPacket` values, and owns the resulting logical outputs.

## Current Boundary

This proposal currently establishes:

- abstract packets and semantic input batches at the Stage boundary;
- developer-authored interaction definitions over selected state and `Tau`;
- closed transformations for Orchestrator admission;
- journal and branch publication as the authoritative state path; and
- the existing `Worldline`, `LogicalTime`, `Tau`, `GameState`, and `Frame`
  contracts from `spec/initial.md`.

## Open Questions

1. What is the smallest `InputPacket` and `Transformation` vocabulary for the
  current Caravan interaction?
2. Which interactions need an ordered batch directly, and which need only a
  membership view?
3. Which transformations are Stage-local view operations, journal facts, or
  branch operations?
