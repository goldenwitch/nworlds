# Transport and Journal Layer

This proposal defines the reusable logical layer between source-specific
transport and the game-facing Stage/Orchestrator composition. It is the
pattern shared by local input, replay input, and future network input.

The layer encapsulates transport logistics and journal operations without
making transport metadata authoritative game state and without making source
platforms or network protocols part of the game description.

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

The same layer must support the inverse authoritative path:

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
> **transport envelope**: Source and delivery metadata surrounding an observation
> payload. It may carry source identity, stream identity, observation identity,
> sequence information, and delivery status. It is not authoritative game time
> and is not part of `GameState`.
>
> **ordered input batch**: The deterministic semantic collection supplied to one
> interaction step. It preserves observation identity and an explicit order
> relation. Equal payloads with different identities remain distinct.
>
> **membership view**: A derived set view of an ordered input batch for an
> interaction that only needs packet presence. The current `InputPacketSet` is
> this prototype-shaped specialization.
>
> **journal operation**: A value-producing operation that admits an accepted
> transformation through `JournalWriter`, counterfactual construction, or
> corrected-branch construction. It does not mutate an existing journal.

## Ordered Batch Semantics

The reusable input collection is not an unordered set. Its canonical properties
are:

- each observation has stable identity within its source stream;
- order is explicit rather than inherited from hash-map or arrival iteration;
- duplicate identities are handled by transport policy;
- equal payloads with different identities are not silently deduplicated;
- a source may deliver observations out of order, but the semantic batch is
  normalized before interaction reasoning;
- multiple sources require an explicit merge order rather than accidental host
  arrival order; and
- the batch is replayable without the original device or network connection.

An interaction that only needs membership may derive a set view. An interaction
that needs press/release order, repeated actions, or deterministic event
replay consumes the ordered batch instead.

The current prototype remains valid as a narrow specialization:

```text
InputPacket
    -> InputPacketSet<HashSet<InputPacket>>
    -> InteractionDefinition
```

It must not become the canonical reusable collection for network or replay
semantics because it intentionally discards order and repeated equal payloads.

## Transport Metadata

Transport metadata exists to make delivery deterministic and recoverable. It
may support duplicate suppression, ordering, acknowledgement, replay, and late
arrival handling. It must not become a hidden game clock or an alternate source
of authority.

The game-facing interaction seam receives semantic observations, not socket
handles, operating-system events, host timestamps, acknowledgement state, or
network connection objects.

When a transport observation is accepted as an authoritative game change, the
Orchestrator chooses the journal authoring time and publishes through the
existing journal/branch APIs. A source sequence number can explain identity and
ordering; it does not assign `LogicalTime`.

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

The actual journal is never rewritten in place. A late or corrected input uses
the existing immutable branch machinery. A rejected transformation leaves the
selected worldline unchanged.

`JournalWriter` remains the timestamp authority. `engine-journal` and
`engine-branches` remain the current publication machinery. The transport and
journal layer coordinates them; it does not replace their immutable values or
introduce a mutable current board.

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
  views, and coordinates journal admission/publication operations.
- Stage/Orchestrator owns interaction meaning, admission decisions, selected
  `LogicalTime`/`Tau`, and whether an accepted result becomes actual,
  counterfactual, or corrected history.
- `JournalWriter` owns game-facing timestamp assignment.
- Branch construction owns immutable prefix/suffix behavior.
- The engine/domain evaluator owns the meaning of the resulting worldline.
- The host does not interpret packets or write authoritative journal facts.

## Non-Goals

This proposal does not define:

- an operating-system event model;
- a socket, replication, acknowledgement, or wire protocol;
- host time or input timestamps;
- a universal game command or interactable-object model;
- a generic engine Orchestrator trait or loop API;
- branch merging or multiplayer authority semantics; or
- a required serialization format for transport or persistence.

## Acceptance Shape

A future implementation of this layer is complete when focused evidence proves:

- local, replay, and network-shaped sources can produce the same semantic batch;
- observation identity and explicit order survive normalization;
- duplicate identities are handled deterministically;
- repeated equal payloads remain distinct when their identities differ;
- membership views reproduce the current set-based interaction behavior;
- out-of-order delivery does not become accidental semantic order;
- accepted inputs publish immutable journal or branch values;
- late inputs do not mutate the published parent;
- transport metadata never supplies authoritative `LogicalTime`; and
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
