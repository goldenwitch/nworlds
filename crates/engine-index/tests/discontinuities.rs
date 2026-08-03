use engine_index::{
    game_tick_boundary, Breakpoint, BreakpointSource, DiscontinuityIndex, DiscontinuityIndexError,
    Piece, PieceBoundsError,
};
use engine_time::LogicalTime;

fn time(ticks: i64) -> LogicalTime {
    LogicalTime::from_ticks(ticks)
}

#[test]
fn an_empty_index_is_one_unbounded_piece() {
    let index = DiscontinuityIndex::from_breakpoints(
        Vec::<Breakpoint<()>>::new(),
        vec!["all logical time"],
    )
    .expect("the empty partition has one piece");

    assert!(index.is_empty());
    assert_eq!(index.breakpoint_count(), 0);
    assert_eq!(index.piece_count(), 1);
    assert_eq!(index.boundary_times(), &[]);
    assert_eq!(index.selected_piece_index(time(i64::MIN)), 0);
    assert_eq!(index.selected_piece_index(time(i64::MAX)), 0);
    assert_eq!(index.select(time(0)).payload(), &"all logical time");
    assert_eq!(index.select(time(0)).start_t(), None);
    assert_eq!(index.select(time(0)).end_t(), None);
}

#[test]
fn journal_and_tick_sources_at_one_time_remain_distinct() {
    let journal = Breakpoint::journal(time(0), 7);
    let tick = Breakpoint::game_tick(time(0), 0);
    let index = DiscontinuityIndex::from_breakpoints(
        vec![journal.clone(), tick.clone()],
        vec!["before", "after"],
    )
    .expect("two sources at one boundary create two pieces");

    assert_eq!(index.breakpoint_count(), 2);
    assert_eq!(index.boundary_times(), &[time(0)]);
    assert_eq!(
        index.breakpoints_at(time(0)).collect::<Vec<_>>(),
        vec![&journal, &tick]
    );
    assert_eq!(
        journal.source(),
        BreakpointSource::Journal { append_ordinal: 7 }
    );
    assert_eq!(tick.source(), BreakpointSource::GameTick { tick_index: 0 });
    assert_eq!(index.select(time(-1)).payload(), &"before");
    assert_eq!(index.select(time(0)).payload(), &"after");
}

#[test]
fn journal_order_is_preserved_by_append_ordinal_at_equal_time() {
    let later_ordinal = Breakpoint::journal(time(5), 9);
    let earlier_ordinal = Breakpoint::journal(time(5), 3);
    let index = DiscontinuityIndex::from_breakpoints(
        vec![later_ordinal.clone(), earlier_ordinal.clone()],
        vec!["before", "after"],
    )
    .expect("the journal sources share one boundary");

    assert_eq!(
        index.breakpoints_at(time(5)).collect::<Vec<_>>(),
        vec![&earlier_ordinal, &later_ordinal]
    );
}

#[test]
fn pieces_are_half_open_and_cover_negative_time_without_clamping() {
    let index = DiscontinuityIndex::from_breakpoints(
        vec![
            Breakpoint::game_tick(time(-1_000), -1),
            Breakpoint::game_tick(time(0), 0),
            Breakpoint::game_tick(time(1_000), 1),
        ],
        vec!["before", "negative tick", "zero tick", "positive tick"],
    )
    .expect("the signed grid has four pieces");

    assert_eq!(index.select(time(-1_001)).payload(), &"before");
    assert_eq!(index.select(time(-1_000)).payload(), &"negative tick");
    assert_eq!(index.select(time(-1)).payload(), &"negative tick");
    assert_eq!(index.select(time(0)).payload(), &"zero tick");
    assert_eq!(index.select(time(999)).payload(), &"zero tick");
    assert_eq!(index.select(time(1_000)).payload(), &"positive tick");

    for (piece, (expected_start, expected_end)) in index.pieces().iter().zip([
        (None, Some(time(-1_000))),
        (Some(time(-1_000)), Some(time(0))),
        (Some(time(0)), Some(time(1_000))),
        (Some(time(1_000)), None),
    ]) {
        assert_eq!(piece.start_t(), expected_start);
        assert_eq!(piece.end_t(), expected_end);
    }
}

#[test]
fn game_tick_boundaries_use_floor_grid_for_negative_indices() {
    let negative_boundary =
        game_tick_boundary(-1).expect("negative tick boundary is representable");
    let zero_boundary = game_tick_boundary(0).expect("zero tick boundary is representable");

    assert_eq!(negative_boundary.logical_time(), time(-1_000));
    assert_eq!(
        negative_boundary.source(),
        BreakpointSource::GameTick { tick_index: -1 }
    );
    assert_eq!(zero_boundary.logical_time(), time(0));
    assert!(game_tick_boundary(i64::MAX).is_none());
}

#[test]
fn derived_breakpoint_payloads_are_opaque_to_selection() {
    let threshold = Breakpoint::derived(time(2), String::from("domain threshold"));
    let index =
        DiscontinuityIndex::from_breakpoints(vec![threshold.clone()], vec!["before", "after"])
            .expect("one derived boundary has two pieces");

    assert_eq!(
        index.breakpoints()[0].payload(),
        &String::from("domain threshold")
    );
    assert_eq!(index.select(time(1)).payload(), &"before");
    assert_eq!(index.select(time(2)).payload(), &"after");
}

#[test]
fn index_results_are_independent_of_query_order_and_source_storage() {
    let source_breakpoints = vec![Breakpoint::journal(time(4), 0)];
    let index =
        DiscontinuityIndex::from_breakpoints(source_breakpoints.clone(), vec!["before", "after"])
            .expect("one journal boundary has two pieces");

    let after = index.select(time(4)).payload();
    let before = index.select(time(3)).payload();
    let after_again = index.select(time(4)).payload();

    assert_eq!(after, &"after");
    assert_eq!(before, &"before");
    assert_eq!(after_again, after);
    assert_eq!(source_breakpoints[0].logical_time(), time(4));
    assert_eq!(index.breakpoints()[0].logical_time(), time(4));
}

#[test]
fn invalid_piece_bounds_and_payload_counts_are_explicit() {
    assert_eq!(
        Piece::new(Some(time(4)), Some(time(4)), "payload"),
        Err(PieceBoundsError::StartNotBeforeEnd {
            start_t: time(4),
            end_t: time(4),
        })
    );

    assert_eq!(
        DiscontinuityIndex::from_breakpoints(
            vec![Breakpoint::journal(time(0), 0)],
            Vec::<&str>::new()
        ),
        Err(DiscontinuityIndexError {
            expected: 2,
            actual: 0,
        })
    );
}
