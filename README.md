# nworlds

nworlds is a library-first Rust workspace for deterministic, directly indexed
temporal game worlds. The reusable temporal library and target-neutral host are
the product boundary. **Caravan of Seasons** is the contained reference game
and sample consumer used to exercise immutable worldlines, indexed state
queries, semantic input, presentation, and persistence.

This repository is an active proof-of-life and research implementation. It is
not a production game, a released engine SDK, or a release distribution.

## Core Model

Authoritative state is queried from an immutable worldline:

```text
state(worldline, logical_time) -> game_state
```

Presentation is downstream:

```text
GameState + Tau -> Frame<RenderBatch>
```

The game-facing path is independent of transport and device choices. Native
input is normalized into ordered observations and then into a payload-only
`SemanticInputBatch`. Rendering consumes owned output; it does not become an
authoritative interaction surface.

## Developer Path

The intended target-neutral game-development path is:

```text
nworlds test
nworlds run
nworlds package
```

The command surface is specified in the [target-factory proposal](proposals/target-factory.md)
and is not yet a shipped CLI in this research workspace. The current executable
checks and sample commands are listed in [index.md](index.md).

The independent voxel sample is a separate consumer of the generic engine:

```text
cargo run --manifest-path crates/voxel-sample/Cargo.toml
```

It owns a journal-authored cottage made from distinct block kinds. Click a
voxel to publish its removal; use the mouse wheel to adjust the voxel scale
continuously. The recommended engine integration example is
[`engine_integration.rs`](crates/voxel-sample/src/engine_integration.rs); the
game-specific voxel model lives separately in `world.rs`.

## Repository Guide

Game developers should start with the [Game Developer Guide](game-developer-guide.md),
which walks through the engine features and the target-neutral composition path.

The structured workspace map, specifications, proposals, evidence, and common
commands live in [index.md](index.md). Current work and deferred decisions are
tracked in the [roadmap](roadmap.md) and the active VINE graph.

Contributor workflow and required checks are in
[CONTRIBUTING.md](CONTRIBUTING.md). Target and device observations remain
separate from library and game-package evidence.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the branch, review, testing, and
design-record practices. Security reports belong in [SECURITY.md](SECURITY.md),
not in public issues.

## License

The current source and documentation are released under the [MIT License](LICENSE).
