# Initial Specification for the game "Caravan of Seasons"

This draft describes the smallest model that preserves continuous logical time, presentation time, lookahead, and counterfactual histories.

## Vocabulary

All vocabulary terms are bold as is tradition.

> **t_**: Continuous logical time. The value of **t_** is part of authoritative state.
>
> **tau**: Presentation time. It selects when a state is sampled for display and is independent of **t_**.
>
> **context**: The immutable rules, configuration, assets, and definitions of the game.
>
> **void**: The ordinary default absence of populated world content in a
> domain result. It is a value, not an out-of-domain or uninitialized state.
>
> **journal**: An append-only sequence of authoritative facts, each associated with a value of **t_**. The journal populates the void; it does not record mutations to a hidden initial game state.
>
> **worldline**: An immutable **context** together with a journal branch. A branch may be the actual history or an immutable counterfactual fork.
>
> **game state**: The authoritative result of evaluating a **worldline** at a value of **t_**. It contains **t_** itself.
>
> **playback**: A pure function from **tau** to **t_** that selects which logical time is displayed.
>
> **frame**: The result of rendering one **game state** at one value of **tau**.
>
> **animation**: A pure function over **tau**. An optional result means that the visual element is absent at that value.

## Invariants

- A **game state** is a pure function of its **worldline** and **t_**.
- A **game state** is an indexed interpretation of the **context** and the
    journal facts visible at **t_**. It does not require a stored initial game
    state or a mutable state carried forward from an earlier query.
- Distinct values of **t_** select distinct game states, because logical time is authoritative state.
- The logical-time domain is continuous. State fields may nevertheless change discontinuously at journal events.
- A **playback** may select any logical time, including a time beyond the current journal horizon or a time in the past.
- Rendering is pure for fixed inputs:

```text
render : GameState -> Tau -> Frame
```

- Rendering and animation do not modify the **context**, **journal**, **worldline**, or **game state**.
- All procedural presentation is reconstructible from the selected **game state**, immutable definitions, and **tau**. No frame history is required.

## Core functions

```text
state : Worldline -> LogicalTime -> GameState
playback : Tau -> LogicalTime
present : Worldline -> Playback -> Tau -> Frame
```

Their composition is:

```text
present(worldline, playback, tau) =
    render(state(worldline, playback(tau)), tau)
```

`LogicalTime` and `Tau` are distinct static types even if they share a numeric representation.

## Authority and population

The **context** supplies immutable definitions, configuration, and the rules for
interpreting a worldline. Authoritative instance content is introduced through
journal facts. There is no implicit creation phase, mutable current board,
hidden random source, or evaluator side channel that can create authoritative
content outside the journal.

## Journal-populated worlds

A **worldline** is interpreted from its context and journal. `state` is a total
query over every journal value, including the empty journal. An empty journal
is the ordinary zero-fact input: the query evaluates the domain's default
values and returns its ordinary empty result at the requested **t_** through
the same evaluation path used for every visible-entry prefix.

Authoritative journal facts add domain content to that default result. In the
Caravan anchor, `CreateSaucer` is an ordinary fact that adds the saucer's
tiles; later facts add actors, terrain, and other values.

The function `state(worldline, t_)` directly determines the authoritative
result for the requested time. It does not advance a current board, rewind a
previous result, or depend on the order in which times were queried. Derived
consequences of journal facts are part of the indexed result and need not be
recorded as additional mutations.

## Lookahead

Lookahead evaluates the current **worldline** at a future value of **t_** while holding its **journal** fixed. No unrecorded future action is assumed.

```text
future_state = state(actual_worldline, future_t)
```

A later authoritative event produces a new journal or branch; it does not mutate a previously evaluated lookahead. The same journal remains fixed while temporal definitions determine the result at the requested future value of **t_**.

## Counterfactual pasts

A counterfactual is an immutable fork at a logical time `fork_t`:

```text
counterfactual =
    prefix(worldline.journal, fork_t)
    + alternate_events
```

The counterfactual agrees with its parent before `fork_t` and may diverge afterward. The actual journal remains unchanged. Counterfactual initial conditions require a different **context**, not an alteration of the journal.

The same **playback** mechanism can scrub or play either worldline.

## Rendering

The renderer receives the selected **game state** and **tau**. The GPU may evaluate animations entirely from those inputs. A persistent GPU simulation that depends on unrecorded previous frames is outside this model.
