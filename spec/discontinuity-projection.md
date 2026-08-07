# Discontinuity Projection

This specification defines the discontinuity index and piecewise projection contract
for the direct indexed query. The reference oracle now uses this path; the
historical tick fold is retained only as a test baseline where its semantics
remain comparable. This specification does not add actor rules.
All behavior referred to here is already defined by
[cellular-automata-anchor.md](cellular-automata-anchor.md).

## Contract

For one immutable worldline and its immutable context definitions:

```text
state(worldline, t_) =
      project(discontinuity_index(worldline), t_)
```

A **breakpoint** is a logical-time location at which a selected indexed value
may change. A breakpoint is a value-level fact, not an imperative action. The
index is an immutable derived value. It may be materialized eagerly for the
finite anchor journal or represented lazily behind the same value/query
boundary, but it must not mutate the worldline or depend on query history.

The index preserves three distinct source classes:

- **Journal breakpoints**: the exact timestamp of each authoritative journal
   entry, together with its immutable append ordinal.
- **Tick breakpoints**: the boundaries of the one-second, phase-zero game-tick
   grid used by the anchor.
- **Caravan definition breakpoints**: thresholds already required by the
   anchor's terrain, actor, effect, and resource definitions.

If different source classes have the same timestamp, the index retains the
source identity and metadata. It must not replace an exact journal breakpoint
with a tick breakpoint merely because their timestamps coincide.

## Piece Selection

The ordered index partitions logical time into non-overlapping half-open
pieces:

```text
piece_i = [start_t, end_t)
```

Each piece contains immutable visible-entry inputs and a domain-owned
projection regime. The rule calculation itself remains query-local rather than
being a callable stored inside the generic engine piece. The first and last
pieces may have an unbounded endpoint. For every representable query time `t_`,
exactly one piece satisfies:

```text
start_t <= t_ && t_ < end_t
```

The selected piece owns its left boundary. The right boundary is excluded, so
when `t_ == end_t` selection moves to the piece beginning at that timestamp.
There are no gaps, and a breakpoint does not become an inclusive right edge of
the preceding piece.

Projection evaluates the selected piece against the exact requested time:

```text
piece = select(index, t_)
game_state = project(piece.inputs, piece.regime, t_)
game_state.logical_time = t_
```

The returned SDK `GameState` therefore preserves the requested `LogicalTime`
even when the projected automaton data is constant across an entire piece.

## Indexed Actor Trajectories and Tick Coverage

The selected piece is a value boundary, not a license for `state` to replay
every preceding game tick. Caravan actor trajectories are indexed value
functions built from immutable worldline inputs. Querying a piece samples the
selected trajectory at the requested tick and `t_`; it does not reconstruct an
actor layer by iterating from tick zero to the target.

Every one-second game-tick boundary at which an indexed Caravan value may
change is represented in the relevant discontinuity index. A sparse set that
contains only journal ticks, threshold ticks, and the requested endpoint is
insufficient when intermediate actor, effect, or resource values can differ.
The generic engine still stores only opaque breakpoints and pieces; Caravan
defines the trajectory values and the finite boundary range needed by a query.
The public `state` query builds a query-scoped index through the requested
sample time. A reusable journal-only index is bounded by its materialized
trajectory horizon and must return an explicit horizon error rather than
silently dropping actors when asked for a later sample.

## Journal Visibility and Ordering

For a fixed branch, an entry `e` is visible to a query exactly when:

```text
e.timestamp <= t_
```

The target timestamp is inclusive. Entries after `t_` are invisible, including
entries that were postdated into the immutable journal before the query was
made. The visible sequence is ordered chronologically and, for equal
timestamps, by the journal's append ordinal.

All entries sharing a timestamp are visible at that timestamp. Their append
order is retained as input ordering wherever an existing Caravan definition
needs a deterministic tie-breaker. There is no secondary ordering invented
from actor identifiers, map order, or breakpoint source kind. A tick marker at
the same timestamp is a separate marker, not an extra journal entry and not an
imperative operation to order before or after the journal group.

For terrain events that activate on the same tick and tile, authored
`SetTerrain` entries are ordered by journal append order before derived terrain
events. Derived events retain the anchor's fixed source order and stable
actor/tile ordering. This is a Caravan rule, not an ordering imposed by the
generic breakpoint index.

Cross-rule evaluation uses the same value-level ordering: authored terrain,
Farmer wheat placement, Arborist forest conversion, Fire ignition and
aging/spread, then Fire terrain destruction. Fire ignition sees the terrain
result after the authored and vegetation-derived events at that tick; resource
totals sample the resulting terrain and actor layers.

## Journal and Tick Breakpoints

The anchor uses a one-second game-tick period and zero phase. Let
`P = 1,000 ms`. For every signed tick index `n`, its interval is:

```text
tick_n = [n * P, (n + 1) * P)

```

The division is mathematical floor division over signed logical time, not
integer division that truncates toward zero.

A journal timestamp has two distinct consequences:

1. Its journal breakpoint makes the entry visible at the exact timestamp.
2. If an existing discrete Caravan definition consumes that entry as tick
    input, its activation occurs on the applicable tick boundary.

For an entry at `e_t`, define its activation boundary as the first tick
boundary at or after that timestamp:

```text
activate_at(e_t) = P * ceil(e_t / P)
```

Thus an entry strictly inside `tick_n` is visible from `e_t` onward, but does
not alter the tick-derived result in `[e_t, (n + 1) * P)`. It participates in
the next tick-boundary result beginning at `(n + 1) * P`. The index contains
both the exact journal breakpoint `e_t` and that tick boundary.

An entry whose timestamp is already a tick boundary has
`activate_at(e_t) == e_t`. It is visible at that boundary and participates in
that tick. The piece beginning at the boundary is consequently selected for
the query at the boundary itself.

This rule does not suppress an exact-time change to a journal-visible layer.
For example, a direct entry value may change at an inside-tick journal
breakpoint while the automaton-derived value remains the value for the current
tick until the next tick breakpoint. The existing Caravan definition decides
which layers expose which values; this specification adds no new actor
behavior.

## Negative Logical Time

Negative logical time uses the same signed grid and the same half-open rule.
There is no implicit clamp to zero and no special pre-zero stepping mode. For
example:

```text
tick_index(-1 ms)      == -1
-1 ms                  in [-1,000 ms, 0 ms)
activate_at(-1 ms)     == 0 ms
activate_at(-1,001 ms) == -1,000 ms
activate_at(-1,000 ms) == -1,000 ms
```

Queries before zero select ordinary negative-time pieces. If an existing
Caravan definition has no value at a requested negative time, it returns that
definition's established absent or out-of-domain result; the index does not
invent an initial board or extrapolate a new actor rule. Checked time
arithmetic remains the responsibility of the shared time module.

## Query-Local Calculation

Projection may allocate disposable local values while answering one query:
temporary collections, derived layer values, and other pure intermediate
calculations are allowed. They are calculation, not authoritative state, and
are discarded after the query.

The query boundary rejects temporal continuation inputs. Evaluation must not
receive or consult:

- a prior `GameState` or any other prior query result;
- a mutable current board, actor, or resource counter;
- a journal-writer or evaluator cursor;
- frame, presentation, or query-order history.

A local calculation may inspect immutable context, the immutable branch
journal, the selected piece, exact `t_`, and the piece's definition inputs. It
may not turn a temporary value into a hidden tick-by-tick continuation from a
different requested time, publish it as current state, or expose an actor
stepping API.

## Ownership Boundary

The engine SDK owns the generic machinery:

- `LogicalTime` ordering, checked time operations, and generic boundary
   selection;
- immutable worldline, journal, branch, and query-input envelopes;
- immutable breakpoint/index and half-open piece containers;
- selection of the unique piece for a requested `t_`;
- the SDK `GameState` envelope carrying the exact requested time and opaque
   game payload.

The SDK may provide parameterized time-grid primitives, including the anchor's
one-second grid, but it does not interpret their game meaning.

Caravan owns the domain definitions:

- which existing journal entries produce exact or tick-activation
   breakpoints;
- the anchor's use of the one-second tick grid and its definition-specific
   thresholds;
- the projection values for saucers, tiles, terrain, actors, effects, and
   resources;
- visibility and out-of-domain rules for those game values.

The SDK must not turn saucers, tiles, terrain, actors, fire, or resources into
engine primitives. Caravan must not receive an engine-owned current-state
object or put game meaning into the generic breakpoint selector. Journal and
game payloads remain distinct: the engine carries their immutable envelopes,
while Caravan defines the payload meaning.

## Existing Discontinuity Sources

The index represents the sources already present in the anchor:

- `CreateSaucer` and postdated spawn/terrain entries at their assigned times;
- one-second game-tick boundaries;
- the existing farmer terminal, forester movement, and arborist completion
   thresholds;
- the existing fire ages, spread, and terrain-destruction thresholds;
- the existing fighter/arsonist collision threshold;
- existing resource counting or integration boundaries.

These are indexed locations in returned values, never actions performed on
objects. Each source is traced to an existing anchor definition; this
specification adds or alters no actor behavior.

## Acceptance

- The public `state(worldline, t_)` API is unchanged.
- Every indexed piece is half-open `[start_t, end_t)` and boundary selection is
   unique and gap-free.
- Exact journal visibility is inclusive; later postdated entries are ignored.
- Equal-time journal entries remain in append order.
- Journal and tick breakpoints remain distinct, including when timestamps match.
- Every game-tick boundary that can change an indexed Caravan value is present
   in the relevant index; sparse endpoint-only tick coverage is not sufficient.
- Inside-tick entries are visible immediately but activate discrete tick input
   at the next boundary; boundary entries activate at that boundary.
- Same-tick authored terrain entries precede derived terrain events by the
   explicit Caravan ordering rule.
- Fire ignition and aging/spread consume the authored-plus-vegetation terrain
   result for that tick; destruction remains the final Fire terrain event.
- Collective forester proposals use shared pre-tick occupancy and deterministic
   destination conflict resolution.
- Forester resource totals use the resulting collective actor positions at each
   indexed tick rather than original journal positions.
- Negative logical times use floor-based tick selection and the same activation
   rule without clamping to zero.
- Projection has no prior-state, current-board, cursor, or frame-history input;
   disposable query-local calculation is permitted, but query evaluation does
   not replay preceding ticks to reconstruct the selected actor layer.
- A reusable index queried beyond its materialized trajectory horizon returns a
   typed error; the public `state` path builds a sufficient query-scoped index.
- Query order, repeated samples, and branch choice do not affect results.
- Existing anchor, conformance, demo, persistence, and presentation tests pass
   against the projection path.
- The production query path contains no reference fold. Any historical parity
   comparison is explicitly labeled as test-only evidence.

The representation may be richer than this first implementation requires, but
it must preserve this value boundary and remain an internal representation
choice rather than a new source of state or mutation.
