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

## Library Boundary Evidence

This section is the reusable-library proof package. Caravan rules, the Caravan
reference oracle, the demo, and native desktop behavior are recorded separately
below and are not treated as generic library evidence.

| Library claim | Executable evidence | Command | Coverage |
| --- | --- | --- | --- |
| An external consumer can use the supported generic facade | `external_consumer_composes_generic_query_branch_and_presentation`; `external_consumer_queries_an_immutable_branch_without_query_history` | `cargo test -p purity-tests --test generic_consumer --locked` | Caller-owned context and payloads compose through `engine-api`; direct queries, immutable branches, query-order independence, and `GameState + Tau` presentation are exercised without Caravan imports. |
| Generic SDK payloads remain opaque and distinct across boundaries | `opaque_payloads_remain_generic_across_context_journal_and_worldline`; `typed_query_results_keep_values_and_domain_reasons_distinct`; `public_payload_access_is_shared_only` | `cargo test -p engine-sdk --test envelopes --locked` | Context, journal, worldline, state, frame, and query-result envelopes carry caller-owned values without game or target types. |
| Library and voxel sample dependency direction remains one-way | `library_production_manifests_do_not_depend_on_consumers`; `voxel_sample_does_not_depend_on_caravan_consumers` | `cargo test -p purity-tests --test dependency_boundaries --locked` | Production manifests for the temporal library reject Caravan, sample, host, target, window, and backend dependencies; the voxel sample rejects Caravan and the existing desktop sample. Intentional dev-only reference fixtures are outside the checked sections. |
| The reusable library and host build without the sample or desktop target | Library-only package check in CI | `cargo check -p engine-time -p engine-sdk -p engine-journal -p engine-branches -p engine-index -p engine-presentation -p engine-api -p nworlds-host --locked` | The command names only reusable temporal/host packages; it does not build `caravan-demo` or `nworlds-desktop`. |
| Immutable publication has no mutable authoritative escape | `no_mutable_authoritative_state`; `no_published_mutation`; `no_caller_assigned_timestamps` | `cargo test -p purity-tests --test boundary --locked` | Compile-fail evidence rejects mutable worldline/state access and caller-assigned journal timestamps. |
| Presentation remains state-first | `no_extra_renderer_input`; `render_packets_are_owned_static_send_and_sync_data`; `repeated_equal_state_and_tau_inputs_project_equal_output` | `cargo test -p purity-tests --test boundary --locked` and `cargo test -p engine-presentation --test presentation --locked` | Render production accepts only `GameState + Tau`, returns owned output, and remains deterministic for equal inputs. |
| Host-owned RenderBatch is generic owned draw intent | `render_batch::tests::batch_is_owned_triangle_data`; `render_batch::tests::batch_is_send_sync_static_data`; Caravan and voxel `Frame<RenderBatch>` client builds | `cargo test -p engine-presentation --locked`, `cargo check -p caravan-demo --locked`, and `cargo check -p voxel-sample --locked` | The shared batch contains only owned clip-space vertices/colors; current clients project into it and target sinks no longer consume game-specific render models. |
| Host ports remain target-neutral | `composition_delegates_to_a_target_neutral_package` and the generic `ApplicationHost` unit suite | `cargo test -p nworlds-host --locked` | Input, storage, render, and package composition are tested with non-Caravan test values; host code owns transport only. |

The current retained dependency gap is intentionally empty for production
library dependencies. Caravan appears in selected engine test/dev-dependencies
only as reference data; the architecture guard does not treat those fixtures
as library production contamination.

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
| Negative timestamp persistence and replay | `negative_timestamp_journals_round_trip_and_replay` | `crates/caravan-persistence/tests/persistence.rs` | Runtime pass |
| Bounded projection parity | `legacy_fold_matches_projection_on_shared_fixture`, `frozen_expected_corpus_matches_the_projection` | `crates/caravan-reference/src/legacy_evaluator.rs`; `crates/caravan-reference/tests/parity.rs` | Runtime pass; legacy equivalence is fixture-scoped |
| Low-level timestamp facade boundary | `no_low_level_timestamp_import` | `crates/purity-tests/tests/ui/` | Compile-fail pass; intentional interoperability API remains documented |
| Concrete Caravan rendering projection | `empty_state_projects_to_owned_empty_output`, `saucer_projection_preserves_stable_tile_order`, `projection_preserves_layers_actors_and_resources`, `repeated_equal_state_and_tau_inputs_project_equal_output`, `render_packets_are_owned_static_send_and_sync_data` | `crates/caravan-demo/src/render.rs` | Runtime and compile-time pass; owned render packets preserve state layers, resources, logical time, and `Tau` through `Frame` |
| Render production accepts no extra runtime input | `no_extra_renderer_input` | `crates/purity-tests/tests/ui/` | Compile-fail pass; `Renderer::render` cannot accept a journal, worldline, Orchestrator, or auxiliary view input |

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

## Native Windows Evidence

| Clause | Evidence | Coverage |
| --- | --- | --- |
| Historical desktop `winit`/`wgpu` host launched and presented the owned render output | Historical `cargo run --manifest-path crates/nworlds-desktop/Cargo.toml --bin nworlds-desktop` observation | Manual local acceptance: the window opened and rendered the Caravan scene on `x86_64-pc-windows-msvc`; retained as proof evidence while the generic host is migrated |
| Native resize remains below the game layer | Local window resize observation | Manual local acceptance: resizing stretched the presentation without changing the game-facing path |
| Native input reaches the unchanged semantic interaction path | Space key through `winit` keyboard input | Manual local acceptance: Space changed the center tile color |
| Native shutdown is owned by the target event loop | Close request through the Windows window | Manual local acceptance: the target exited cleanly |

| Generic desktop target has no game dependency | `cargo test -p purity-tests --test dependency_boundaries --locked` | Runtime manifest guard: `nworlds-desktop` production dependencies exclude Caravan and voxel consumers |
| Synthetic package submits an owned desktop `Frame<RenderBatch>` | `cargo test -p nworlds-desktop --locked` | Runtime pass: target-local generic composition collects a synthetic owned batch without a game crate |
| Caravan remaps onto the generic desktop composition | `cargo check -p caravan-demo --examples --locked` | Compile pass: the sample-owned `CaravanInputAdapter` and `CaravanPackage` compose through `nworlds-desktop::DesktopApplication`; native runtime observation remains separate |
| Voxel remaps onto the generic desktop composition | `cargo test -p voxel-sample --locked`; `cargo check -p voxel-sample --locked` | Runtime/compile pass: package-owned click picking, wheel scale publication, and `Frame<RenderBatch>` production compose through the shared target; native launch and persistence remain separate |
| Current Caravan client launches through the generic desktop composition | `cargo run -p caravan-demo --example desktop --locked` | Manual local runtime smoke: the generated/example composition opened without startup errors on Windows; no device-support claim |
| Current voxel client launches through the generic desktop composition | `cargo run -p voxel-sample --locked` | Manual local runtime smoke: the shared host launched the voxel package without startup errors on Windows; click/scale behavior is covered by package tests; no device-support claim |

## Presentation Driver Evidence

| Claim | Primary evidence | Coverage |
| --- | --- | --- |
| Visual Tau varies presentation without replacing the selected complete state | `cargo test -p engine-presentation --locked` | `PresentationDriver` tests cover Tau-only variation, selected-state identity, overflow, and complete-state sample plans |
| Selecting a new complete state resets visual Tau | `cargo test -p engine-presentation --locked`; `cargo test -p voxel-sample --locked` | Generic driver and voxel package tests cover reset after authoritative publication |
| Redraw is presentation demand, not package update | `cargo test -p nworlds-host --locked`; `cargo test -p nworlds-desktop --locked` | Host test separates `update()` from `present()`; target test proves the generic redraw composition remains package-neutral |
| Both samples use the driver-backed presentation path | `cargo test -p caravan-demo --locked`; `cargo test -p voxel-sample --locked` | Caravan `present_state` and voxel `VoxelPackage` presentation paths remain green with immutable worldline tests |

## Target Artifact and Support Evidence

The target-factory proposal owns artifact identity, manifest/checksum
verification, retention, and the separation between compile, runtime, and
device claims. The platform matrix consumes those records for each profile;
this evidence table names the current implementation gap rather than treating
existing compile jobs as a stronger claim.

| Claim | Primary evidence | Current status |
| --- | --- | --- |
| A generated artifact maps to one package source and one resolved profile | Target-factory `TargetArtifact` contract and manifest/checksum design | Contract settled; generic mint/inspection implementation pending |
| CI can mint and inspect artifacts without game-specific target dependencies | Target-neutral generated-composition CI job with manifest/checksum inspection | Gap: current CI has target compile lanes but no factory artifact job |
| Compile, runtime, and device evidence remain distinct | Platform matrix evidence levels and target-specific records | Contract settled; profile-specific runtime/device publication remains pending |
| A support row reaches `complete` only with its required runtime/device evidence | Platform matrix status rule and `support.vine` closure tasks | Current Windows manual/runtime record retained; other committed rows remain explicit gaps |

## Explicit Gaps

- CodeQL remains a separate CI security workflow in
  `.github/workflows/codeql.yml`; it is not an architectural-boundary proof
  and was not executed locally in this closure pass.
- The authoritative engine boundary has compiler-checked purity evidence in
  `crates/purity-tests`. Rust cannot prove arbitrary `Renderer` implementation
  bodies have no side effects; those remain a trusted presentation extension
  boundary.
- Native window, input, resize, and GPU observations are manual local evidence;
  no CI pixel or device test is claimed by this slice.
- Generic target-artifact minting, manifest/checksum inspection, and publication
  are not yet implemented by the target factory; existing host build jobs are
  compile evidence only.
