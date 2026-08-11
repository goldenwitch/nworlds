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
| Game-facing timestamp authority | `no_caller_assigned_timestamps` | `crates/purity-tests/tests/ui/` | Compile-fail pass |
| Arbitrary and non-monotonic query order | `query-order` | `checks.rs` | Runtime pass |
| Fixed-journal stability without a crossed boundary | `within-tick` | `checks.rs` | Runtime pass: distinct 4,000/4,500 fixed-point samples share tick 4 and automaton data while retaining distinct logical times |
| Exact journal-time discontinuity | `journal-discontinuity` | `checks.rs` | Runtime pass |
| Independent terrain, actor, and effect layers | `layer-separation` | `checks.rs` | Runtime pass |
| Farmer, wheat, forester, fire, fighter, arborist differences | `farmer`, `wheat`, `forester`, `arsonist-fire`, `fighter`, `arborist` | `checks.rs` | Runtime pass |
| Seeded journal construction and reproducibility | `seeded` | `checks.rs` | Runtime pass; RNG ownership is behavioral evidence |
| Prefix agreement and parent/child branch isolation | `branches` | `checks.rs` | Runtime pass |
| Fixed-journal future query and lookahead | `lookahead` | `checks.rs` | Runtime pass |
| Explicit logical/presentation sampling, reverse scrub, state-first rendering, branch presentation | `presentation` | `checks.rs` | Runtime pass |
| Runnable anchor observables | `demo-trace` | `anchor-trace.txt` and demo snapshot test | Runtime pass |

## Root Workspace Evidence

These cases run through `cargo test --workspace` and are intentionally not
duplicated in the separate conformance runner or stamped into its report.

| Clause | Test | Evidence | Coverage |
| --- | --- | --- | --- |
| Query-specific indexed projection at a long horizon | `projection_samples_a_long_stationary_trajectory_without_tick_replay`, `projection_samples_a_long_moving_trajectory_without_tick_replay`, `projection_crosses_external_boundaries_without_replaying_from_tick_zero` | `crates/caravan-reference/tests/projection.rs` | Runtime pass at one million and one billion game ticks; moving actors and external boundaries use compressed segments |
| Late facts do not alter earlier derived behavior | `late_actor_does_not_change_an_earlier_farmer_destination`, `late_authored_terrain_does_not_change_an_earlier_farmer_destination`, `late_actor_does_not_give_an_arsonist_an_earlier_target` | `crates/caravan-reference/tests/projection.rs` | Runtime pass |
| Reusable index horizon boundary | `reusable_index_rejects_samples_beyond_its_trajectory_horizon` | `crates/caravan-reference/tests/projection.rs` | Runtime pass; public state uses a selected query-specific index |
| Cross-rule derived terrain and Fire ordering | `projection_fire_sees_farmer_derived_wheat_on_the_same_tick` | `crates/caravan-reference/tests/projection.rs` | Runtime pass |
| Inside-tick visibility versus boundary activation | `inside_tick_terrain_visibility_does_not_rewrite_the_prior_tick_actor_sample`, `inside_tick_terrain_visibility_does_not_trigger_earlier_fire` | `crates/caravan-reference/tests/projection.rs` | Runtime pass |
| Negative timestamp branch reconstruction | `branches_rebuild_negative_timestamp_prefixes` | `crates/engine-branches/tests/branches.rs` | Runtime pass |
| Negative timestamp persistence and replay | `negative_timestamp_journals_round_trip_and_replay` | `crates/engine-persistence/tests/persistence.rs` | Runtime pass |
| Bounded projection parity | `legacy_fold_matches_projection_on_shared_fixture`, `frozen_expected_corpus_matches_the_projection` | `crates/caravan-reference/src/legacy_evaluator.rs`; `crates/caravan-reference/tests/parity.rs` | Runtime pass; legacy equivalence is fixture-scoped |
| Low-level timestamp facade boundary | `no_low_level_timestamp_import` | `crates/purity-tests/tests/ui/` | Compile-fail pass; intentional interoperability API remains documented |
| Concrete Caravan rendering projection | `empty_state_projects_to_owned_empty_output`, `saucer_projection_preserves_stable_tile_order`, `projection_preserves_layers_actors_and_resources`, `repeated_equal_state_and_tau_inputs_project_equal_output` | `crates/caravan-demo/src/render.rs` | Runtime pass; owned render objects preserve state layers, resources, logical time, and `Tau` through `Frame` |

| Long-horizon query measurement | `moving-forester@game_tick_1000000` | `evidence/benchmarks/anchor-report.json` | Release benchmark report records the million-game-tick moving trajectory workload |

## Orchestrator Evidence

The root workspace carries focused application-level evidence outside the
separate conformance catalog:

| Clause | Test | Evidence | Coverage |
| --- | --- | --- | --- |
| State-aware interaction and explicit projection failure | `identical_input_and_tau_use_the_selected_logical_state`, `malformed_selected_state_fails_before_interaction` | `crates/caravan-demo/tests/input.rs` | Root workspace runtime pass |
| Ordered transport batch to semantic interaction | `ordered_transport_batch_derives_the_current_membership_interaction_view` | `crates/caravan-demo/tests/input.rs` | Runtime pass; identity/order normalize into the payload-only semantic batch while the membership view remains compatible |
| Orchestrator input-window lifecycle | `input_buffer_snapshots_and_resolves_only_the_captured_window`, `retained_window_survives_an_interaction_resolution`, `empty_input_is_a_noop_and_branch_append_requires_explicit_policy` | `crates/caravan-demo/src/input.rs`; `crates/caravan-demo/src/orchestrator.rs` | Runtime pass; later arrivals remain pending, successful/no-op windows resolve, and rejected publication retains input |
| In-memory host crossing | `host_crossing_delivers_input_publishes_immutable_state_and_collects_frame`, `input_transport_order_is_preserved_before_semantic_batch_conversion`, `host_storage_round_trip_preserves_selected_worldline_and_scrubbing` | `crates/caravan-demo/tests/host.rs` | Runtime pass; independent input, render, storage, publication, and scrubbing crossings remain observable |

## Explicit Gaps

- The authoritative engine boundary has compiler-checked purity evidence in
  `crates/purity-tests`. Rust cannot prove arbitrary `Renderer` implementation
  bodies have no side effects; those remain a trusted presentation extension
  boundary.
