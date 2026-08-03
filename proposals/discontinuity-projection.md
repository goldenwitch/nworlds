# Discontinuity Projection

This proposal is the next implementation target after the indexed anchor
prototype. The public query already has the right shape, but the reference
oracle still computes it by folding a private `WorkingState` through ticks.
This work replaces that hidden stepping model with explicit discontinuities and
piecewise projection.

## Target

```text
state(worldline, t_) =
    project(worldline, discontinuity_index(worldline), t_)
```

A **discontinuity** is a logical-time location where an indexed result may
change because of an authoritative journal entry, a game-tick boundary, or a
definition-specific threshold such as fire aging or an actor's terminal action.
Between relevant discontinuities, a projection function determines the result
without consuming or mutating a result from another time.

The projection may use disposable local data while answering one query. That is
ordinary calculation, not authoritative mutation. No query receives a prior
`GameState`, a current board, a mutable actor, a cursor, or frame history.

## Boundaries

Engine responsibilities:

- represent logical-time discontinuities and ordered piecewise domains;
- expose immutable worldline/journal inputs;
- select the applicable piece for a requested `t_`;
- return an SDK `GameState` carrying exact `t_`.

Caravan responsibilities:

- declare which journal entries and actor definitions create discontinuities;
- provide projection values for terrain, actors, effects, and resources;
- define thresholds and visibility rules for the anchor actors.

The engine must not turn saucers, tiles, actors, fire, or resources into SDK
primitives. The game must not receive an engine-owned current-state object.

## Required Shape

The discontinuity index is a value derived from an immutable worldline. It may
be built eagerly for the finite anchor journal or queried lazily later; that is
an implementation choice behind the same interface.

Each indexed piece needs:

```text
[start_t, end_t)
projection inputs
projection function/result definition
```

Journal entries at an exact timestamp are visible at that timestamp. A journal
entry inside a game tick affects the next tick-boundary behavior; an entry on a
tick boundary participates in that tick. The index must preserve exact journal
discontinuities separately from game-tick discontinuities.

## Initial Discontinuity Classes

The anchor needs at least:

- `CreateSaucer` at `t_=0`;
- postdated spawn and terrain entries at their assigned times;
- one-second game-tick boundaries;
- farmer terminal action;
- forester movement decisions;
- arborist completion at three game ticks;
- fire ages 0, 1, 2, and 3;
- fire spread and terrain destruction;
- fighter collision with an arsonist;
- resource-total integration/counting boundaries.

These are locations in the indexed result, not actions performed on objects.

## Work Sequence

1. Freeze the discontinuity and piecewise-function contract.
2. Extract an ordered immutable discontinuity index from the worldline and game
   definitions.
3. Project each game layer from the selected piece and exact `t_`.
4. Compare projection results with the current reference oracle over generated
   traces and arbitrary query order.
5. Delete the private `WorkingState`/tick-fold path after parity is established.

## Acceptance

- The public `state(worldline, t_)` API is unchanged.
- The projection path has no prior-state or current-board input.
- Query order, repeated samples, and branch choice do not affect results.
- Exact journal timestamps and one-second tick boundaries remain distinct.
- The existing anchor, conformance, demo, persistence, and presentation tests
  pass against the projection path.
- The old reference fold is removed only after parity evidence is recorded.

Complex analysis is not required for the first implementation. If a richer
mathematical representation becomes useful, it must preserve the same value
boundary and remain an internal representation choice rather than a new source
of state or mutation.
