#![forbid(unsafe_code)]

mod checks;
mod fixtures;

#[derive(Clone, Copy)]
pub struct Case {
    pub id: &'static str,
    pub clause: &'static str,
    pub test: &'static str,
    pub artifact: &'static str,
    pub run: fn(),
}

#[derive(Clone, Copy)]
pub struct Gap {
    pub id: &'static str,
    pub status: &'static str,
    pub description: &'static str,
}

pub const GAPS: &[Gap] = &[
    Gap {
        id: "presentation-extension-trust",
        status: "explicit",
        description: "The authoritative engine boundary has compiler-checked purity evidence. Rust cannot prove arbitrary Renderer extension bodies have no side effects; those implementations are trusted extensions receiving immutable values and returning owned values.",
    },
];

pub fn cases() -> &'static [Case] {
    &[
        Case {
            id: "empty-journal",
            clause: "Initial spec: void and empty journal; exact sampled t_",
            test: "empty_journal_uses_ordinary_zero_fact_evaluation",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::empty_journal_uses_ordinary_zero_fact_evaluation,
        },
        Case {
            id: "create-saucer",
            clause: "Anchor: CreateSaucer establishes exactly 91 Void tiles",
            test: "create_saucer_has_91_void_tiles_and_empty_layers",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::create_saucer_has_91_void_tiles_and_empty_layers,
        },
        Case {
            id: "journal-time",
            clause: "Journal-owned timestamps, target inclusion, postdated visibility, append order",
            test: "journal_timestamps_control_visibility_and_append_order",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::journal_timestamps_control_visibility_and_append_order,
        },
        Case {
            id: "query-order",
            clause: "Direct indexed query is independent of forward, backward, and repeated query order",
            test: "query_order_does_not_change_a_fixed_worldline",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::query_order_does_not_change_a_fixed_worldline,
        },
        Case {
            id: "within-tick",
            clause: "Fixed journal data is stable when no journal boundary is crossed",
            test: "fixed_journal_repeated_sample_is_stable",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::fixed_journal_repeated_sample_is_stable,
        },
        Case {
            id: "journal-discontinuity",
            clause: "Journal facts create exact-time discontinuities",
            test: "journal_entry_is_visible_at_its_exact_time",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::journal_entry_is_visible_at_its_exact_time,
        },
        Case {
            id: "layer-separation",
            clause: "Terrain, actor, and effects are separate inspectable layers",
            test: "terrain_actor_and_effect_layers_coexist_independently",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::terrain_actor_and_effect_layers_coexist_independently,
        },
        Case {
            id: "farmer",
            clause: "Anchor actor rule: farmer transformation is an indexed difference",
            test: "farmer_difference_is_repeatable_and_places_wheat",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::farmer_difference_is_repeatable_and_places_wheat,
        },
        Case {
            id: "wheat",
            clause: "Anchor resource rule: wheat totals are indexed, not carried",
            test: "wheat_resource_is_indexed_without_query_carryover",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::wheat_resource_is_indexed_without_query_carryover,
        },
        Case {
            id: "forester",
            clause: "Anchor actor/resource rule: forester movement and wood production",
            test: "forester_difference_and_wood_total_are_repeatable",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::forester_difference_and_wood_total_are_repeatable,
        },
        Case {
            id: "arsonist-fire",
            clause: "Anchor hazard rules: ignition, aging, spread, and terrain destruction",
            test: "arsonist_and_fire_are_indexed_layer_differences",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::arsonist_and_fire_are_indexed_layer_differences,
        },
        Case {
            id: "fighter",
            clause: "Anchor conflict rule: fighter movement and arsonist collision",
            test: "fighter_collision_is_a_repeatable_indexed_difference",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::fighter_collision_is_a_repeatable_indexed_difference,
        },
        Case {
            id: "arborist",
            clause: "Anchor actor rule: arborist converts terrain after three turns",
            test: "arborist_conversion_keeps_actor_and_terrain_separate",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::arborist_conversion_keeps_actor_and_terrain_separate,
        },
        Case {
            id: "seeded",
            clause: "Seeded journal is concrete before evaluation and reproducible",
            test: "same_seed_reproduces_journal_and_states",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::same_seed_reproduces_journal_and_states,
        },
        Case {
            id: "branches",
            clause: "Actual, counterfactual, and corrected branches are isolated immutable values",
            test: "branches_agree_at_prefix_and_diverge_without_parent_mutation",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::branches_agree_at_prefix_and_diverge_without_parent_mutation,
        },
        Case {
            id: "lookahead",
            clause: "Lookahead uses the same query with a fixed journal and no generated future entries",
            test: "lookahead_keeps_the_journal_fixed",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::lookahead_keeps_the_journal_fixed,
        },
        Case {
            id: "presentation",
            clause: "Presentation composes explicit logical/presentation samples and state-first rendering without mutation",
            test: "presentation_supports_scrubbing_branches_and_repeatable_rendering",
            artifact: "tests/conformance/src/checks.rs",
            run: checks::presentation_supports_scrubbing_branches_and_repeatable_rendering,
        },
        Case {
            id: "caravan-trace",
            clause: "Runnable Caravan trace exposes the anchor observables",
            test: "caravan_trace_contains_the_anchor_observables",
            artifact: "crates/caravan-sample/snapshots/anchor-trace.txt",
            run: checks::caravan_trace_contains_the_anchor_observables,
        },
    ]
}
