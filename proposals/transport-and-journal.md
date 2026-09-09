# Transport and Journal Layer

This proposal defines the reusable logical layer between source-specific
transport and the game-facing Stage/Orchestrator composition. It is the
pattern shared by local input, replay input, and future network input.

The layer encapsulates transport logistics and journal operations. Transport
metadata supports delivery and recovery, while the game description remains
owned by Stage and the domain model.

## Position

The layer sits between source adapters and game-facing interaction/publication:

```text
local device / network / replay source
    -> source translation and delivery
    -> transport and journal layer
        -> ordered semantic input batch
        -> journal admission/publication operations
    -> Stage / Orchestrator
```

The same layer supports the authoritative path:

```text
Input observation
    -> ordered semantic batch
    -> InteractionDefinition
    -> Transformation
    -> Orchestrator admission
    -> JournalWriter or branch construction
    -> immutable Journal / Worldline
```

The first implementation may remain application-owned. This proposal fixes the
semantic boundary before a reusable engine crate or universal transport API is
introduced.

## Vocabulary

> **observation payload**: The platform-neutral game-facing value carried by
> one input observation. The current prototype spelling is `InputPacket`.
>
> **transport envelope**: Source and delivery metadata surrounding an
> observation payload. It may carry source identity, stream identity,
> observation identity, sequence information, and delivery status. `LogicalTime`
> and `GameState` remain the authoritative game-time surfaces.
>
> **ordered input batch**: The deterministic semantic collection supplied to one
> interaction step. It preserves observation identity and an explicit order
> relation. Equal payloads with different identities remain distinct.
>
> **membership view**: A derived set view of an ordered input batch for a
> membership-oriented interaction. The current `InputPacketSet` is this
> prototype-shaped specialization.
>
> **input buffer**: Orchestrator-owned pending retention for normalized
> observations. It accepts batches, captures windows, and applies explicit
> resolution while interaction definitions interpret packet meaning.
>
> **input window**: An immutable snapshot of exactly the observations supplied
> to one interaction attempt. Later arrivals are not silently included in that
> window.
>
> **input resolution**: The explicit result of an interaction attempt:
> `Retain`, `Consume`, or `Discard`. Publication failure retains the window;
> accepted transformations consume it; a successful no-op discards it.
>
> **journal operation**: A value-producing operation that admits an accepted
> transformation through `JournalWriter`, counterfactual construction, or
> corrected-branch construction. Each operation returns a new immutable
> journal or branch value.

## Ordered Batch Semantics

The reusable input collection is an ordered semantic batch. Its canonical
properties are:

- each observation has stable identity within its source stream;
- order is explicit and independent of hash-map or arrival iteration;
- duplicate identities are handled by transport policy;
- equal payloads with different identities are not silently deduplicated;
- a source may deliver observations out of order, with normalization before
  interaction reasoning;
- multiple sources use an explicit merge order; and
- the batch is replayable from its value independently of the original device
  or network connection.

An interaction may derive a set view for membership. Interactions that use
press/release order, repeated actions, or deterministic event replay consume
the ordered batch.

The derived compatibility view remains valid as a narrow specialization:

```text
InputPacket
    -> InputPacketSet<HashSet<InputPacket>>
    -> InteractionDefinition
```

The ordered batch remains the canonical reusable collection for network and
replay semantics; the set view intentionally specializes membership.

The Orchestrator-facing lifecycle is separate from transport delivery:

```text
InputIngress
  -> OrderedInputBatch
  -> InputBuffer::ingest
  -> InputBuffer::snapshot
  -> InputWindow
  -> interaction / admission
  -> InputBuffer::resolve(window, resolution)
```

The buffer keeps arrivals that occur after a window is captured for a later
window. `Retain` keeps the captured observations pending, while `Consume` and
`Discard` remove that window's identities.

## Transport Metadata

Transport metadata exists to make delivery deterministic and recoverable. It
may support duplicate suppression, ordering, acknowledgement, replay, and late
arrival handling. `LogicalTime` remains the game clock and journal publication
remains the source of authority.

The game-facing interaction seam receives semantic observations. Socket
handles, operating-system events, host timestamps, acknowledgement state, and
network connection objects remain with source adapters.

When a transport observation is accepted as an authoritative game change, the
Orchestrator chooses the journal authoring time and publishes through the
existing journal/branch APIs. A source sequence number explains identity and
ordering, while `JournalWriter` assigns `LogicalTime`.

## Journal Operations

The layer exposes journal operations as value-producing admission paths:

```text
ordered input batch
    -> InteractionDefinition
    -> closed Transformation
    -> admission decision
    -> JournalWriter record
       or counterfactual branch
       or corrected branch
    -> new immutable worldline value
```

Journal operations return new immutable values. A late or corrected input uses
the existing immutable branch machinery. A rejected transformation preserves
the selected worldline value.

`JournalWriter` remains the timestamp authority. `engine-journal` and
`engine-branches` remain the current publication machinery. The transport and
journal layer coordinates them around immutable values; the game query derives
the current board from those values.

## Source Independence

Local, replay, and network sources all cross the same semantic boundary:

```text
source-specific observation
    -> transport envelope
    -> ordered input batch
    -> state-aware interaction query
```

A local source may use device event identity. A network source may use a peer
and sequence identity. A replay source may use recorded journal position. Those
metadata shapes remain below the semantic batch contract and are not exposed as
three different interaction APIs.

## Ownership

- Source adapters translate native or wire observations and transport their
  envelopes.
- The transport and journal layer normalizes identity/order, derives membership
  views, owns the input-buffer/window lifecycle, and coordinates journal
  admission/publication operations.
- Stage/Orchestrator owns interaction meaning, admission decisions, selected
  `LogicalTime`/`Tau`, and whether an accepted result becomes actual,
  counterfactual, or corrected history.
- `JournalWriter` owns game-facing timestamp assignment.
- Branch construction owns immutable prefix/suffix behavior.
- The engine/domain evaluator owns the meaning of the resulting worldline.
- The host transports packets; Stage and the Orchestrator interpret them and
  publish authoritative journal facts.

## Current Boundary

This proposal currently establishes:

- source and stream identity for normalized observations;
- ordered semantic batches and membership views;
- input-buffer and input-window lifecycle;
- journal and branch admission operations over immutable values; and
- application-owned choices for wire formats, host scheduling, multiplayer
  authority, and Orchestrator composition.

## Acceptance Shape

A future implementation of this layer is complete when focused evidence proves:

- local, replay, and network-shaped sources can produce the same semantic batch;
- observation identity and explicit order survive normalization;
- duplicate identities are handled deterministically;
- repeated equal payloads remain distinct when their identities differ;
- membership views reproduce the current set-based interaction behavior;
- captured windows resolve deterministically as retain, consume, or discard;
- publication and projection failures retain the captured window for retry;
- out-of-order delivery resolves through the explicit semantic order;
- accepted inputs publish immutable journal or branch values;
- late inputs produce new immutable branch values;
- transport metadata explains delivery while `LogicalTime` remains authoritative;
- the existing direct-query, branch, persistence, and purity evidence remains
  green.

## Open Decisions

1. What source and stream identity is the smallest reusable observation identity?
2. What merge rule creates one deterministic order when multiple sources meet?
3. Does interaction receive the ordered batch directly, a membership view, or a
   statically selected input view for each definition?
4. What packet retention, flush, consume, and expiry operations belong to this
   layer versus Stage/Orchestrator policy?
5. Which simple wire/record format is sufficient for the first implementation?
