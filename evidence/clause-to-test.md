# Conformance Matrix

Run the executable proof package with:

```text
cargo test --manifest-path tests/conformance/Cargo.toml
cargo run --manifest-path tests/conformance/Cargo.toml -- --report evidence/conformance-report.json
```

The first command runs every catalogued case. The second command reruns the
same cases and writes machine-readable results. The existing demo integration
test additionally checks the complete stdout trace against
`crates/caravan-demo/snapshots/anchor-trace.txt`.

| Clause | Executable case | Evidence | Coverage |
| --- | --- | --- | --- |
| Void, empty journal, exact `t_` | `empty-journal` | `checks.rs` | Runtime pass |
| Journal-owned creation and 91 tiles | `create-saucer` | `checks.rs` | Runtime pass |
| Target inclusion, postdating, equal-time order | `journal-time` | `checks.rs` | Runtime pass |
| Game-facing timestamp authority | `no_caller_assigned_timestamps`, `no_low_level_timestamp_import` | `crates/purity-tests/tests/ui/` | Compile-fail pass; low-level assigned-time SDK imports remain explicitly documented interoperability APIs |
| Arbitrary and non-monotonic query order | `query-order` | `checks.rs` | Runtime pass |
| Fixed-journal stability without a crossed boundary | `within-tick` | `checks.rs` | Runtime pass: distinct 4,000/4,500 fixed-point samples share tick 4 and automaton data while retaining distinct logical times |
| Complete sampled game-tick boundary coverage | `sampled_index_includes_each_game_tick_boundary_through_sample` | `crates/caravan-reference/src/discontinuities.rs` | Supplemental reference test pass |
| Reusable index horizon boundary | `reusable_index_rejects_samples_beyond_its_trajectory_horizon` | `crates/caravan-reference/tests/projection.rs` | Supplemental reference test pass; public state builds query-scoped indices |
| Exact journal-time discontinuity | `journal-discontinuity` | `checks.rs` | Runtime pass |
| Independent terrain, actor, and effect layers | `layer-separation` | `checks.rs` | Runtime pass |
| Farmer, wheat, forester, fire, fighter, arborist differences | `farmer`, `wheat`, `forester`, `arsonist-fire`, `fighter`, `arborist` | `checks.rs` | Runtime pass |
| Cross-rule derived terrain and Fire ordering | `projection_fire_sees_farmer_derived_wheat_on_the_same_tick` | `crates/caravan-reference/tests/projection.rs` | Supplemental reference test pass |
| Seeded journal construction and reproducibility | `seeded` | `checks.rs` | Runtime pass; RNG ownership is behavioral evidence |
| Prefix agreement and parent/child branch isolation | `branches` | `checks.rs` | Runtime pass |
| Fixed-journal future query and lookahead | `lookahead` | `checks.rs` | Runtime pass |
| Explicit logical/presentation sampling, reverse scrub, state-first rendering, branch presentation | `presentation` | `checks.rs` | Runtime pass |
| Runnable anchor observables | `demo-trace` | `anchor-trace.txt` and demo snapshot test | Runtime pass |
| Bounded projection parity | `legacy_fold_matches_projection_on_shared_fixture`, `frozen_expected_corpus_matches_the_projection` | `crates/caravan-reference/src/legacy_evaluator.rs`; `crates/caravan-reference/tests/parity.rs` | Supplemental reference tests pass; legacy equivalence is fixture-scoped |

## Orchestrator Evidence

The root workspace carries focused application-level evidence outside the
separate conformance catalog:

| Clause | Test | Evidence | Coverage |
| --- | --- | --- | --- |
| State-aware interaction and explicit projection failure | `identical_input_and_tau_use_the_selected_logical_state`, `malformed_selected_state_fails_before_interaction` | `crates/caravan-demo/tests/input.rs` | Root workspace runtime pass |

## Explicit Gaps

- The authoritative engine boundary has compiler-checked purity evidence in
  `crates/purity-tests`. Rust cannot prove arbitrary `Renderer` implementation
  bodies have no side effects; those remain a trusted presentation extension
  boundary.
