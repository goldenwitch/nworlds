use core::cmp::Ordering;
use core::fmt;

use engine_time::LogicalTime;

/// Identifies the source of an indexed breakpoint without assigning it game
/// meaning.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BreakpointSource {
    /// An exact journal timestamp and its immutable append ordinal.
    Journal { append_ordinal: usize },
    /// A boundary in the discrete game-tick grid.
    GameTick { tick_index: i64 },
    /// An opaque domain-defined discontinuity.
    Derived,
}

/// An immutable breakpoint with an opaque source payload.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Breakpoint<P = ()> {
    logical_time: LogicalTime,
    source: BreakpointSource,
    payload: P,
}

impl<P> Breakpoint<P> {
    /// Creates a breakpoint from an engine source and opaque payload.
    pub fn new(logical_time: LogicalTime, source: BreakpointSource, payload: P) -> Self {
        Self {
            logical_time,
            source,
            payload,
        }
    }

    /// Creates a journal breakpoint carrying an opaque payload.
    pub fn journal_with_payload(
        logical_time: LogicalTime,
        append_ordinal: usize,
        payload: P,
    ) -> Self {
        Self::new(
            logical_time,
            BreakpointSource::Journal { append_ordinal },
            payload,
        )
    }

    /// Creates a game-tick breakpoint carrying an opaque payload.
    pub fn game_tick_with_payload(logical_time: LogicalTime, tick_index: i64, payload: P) -> Self {
        Self::new(
            logical_time,
            BreakpointSource::GameTick { tick_index },
            payload,
        )
    }

    /// Creates a domain-defined breakpoint without interpreting its payload.
    pub fn derived(logical_time: LogicalTime, payload: P) -> Self {
        Self::new(logical_time, BreakpointSource::Derived, payload)
    }

    /// Returns the exact logical timestamp of the breakpoint.
    pub const fn logical_time(&self) -> LogicalTime {
        self.logical_time
    }

    /// Returns the source identity and source metadata.
    pub const fn source(&self) -> BreakpointSource {
        self.source
    }

    /// Borrows the opaque breakpoint payload.
    pub fn payload(&self) -> &P {
        &self.payload
    }

    /// Consumes the breakpoint and returns its timestamp, source, and payload.
    pub fn into_parts(self) -> (LogicalTime, BreakpointSource, P) {
        (self.logical_time, self.source, self.payload)
    }
}

impl Breakpoint<()> {
    /// Creates an exact journal breakpoint without an additional payload.
    pub fn journal(logical_time: LogicalTime, append_ordinal: usize) -> Self {
        Self::journal_with_payload(logical_time, append_ordinal, ())
    }

    /// Creates a game-tick breakpoint without an additional payload.
    pub fn game_tick(logical_time: LogicalTime, tick_index: i64) -> Self {
        Self::game_tick_with_payload(logical_time, tick_index, ())
    }
}

/// Creates a one-second game-tick breakpoint at a signed tick index.
pub fn game_tick_boundary(tick_index: i64) -> Option<Breakpoint> {
    LogicalTime::from_game_ticks(tick_index)
        .map(|logical_time| Breakpoint::game_tick(logical_time, tick_index))
}

/// An immutable half-open logical-time interval with an opaque payload.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Piece<P> {
    start_t: Option<LogicalTime>,
    end_t: Option<LogicalTime>,
    payload: P,
}

impl<P> Piece<P> {
    /// Creates a non-empty half-open piece.
    ///
    /// `None` represents an unbounded endpoint. When both endpoints are
    /// present, the piece contains `start_t` and excludes `end_t`.
    pub fn new(
        start_t: Option<LogicalTime>,
        end_t: Option<LogicalTime>,
        payload: P,
    ) -> Result<Self, PieceBoundsError> {
        if let (Some(start_t), Some(end_t)) = (start_t, end_t) {
            if start_t >= end_t {
                return Err(PieceBoundsError::StartNotBeforeEnd { start_t, end_t });
            }
        }

        Ok(Self {
            start_t,
            end_t,
            payload,
        })
    }

    /// Returns the inclusive left endpoint, or `None` when unbounded.
    pub const fn start_t(&self) -> Option<LogicalTime> {
        self.start_t
    }

    /// Returns the exclusive right endpoint, or `None` when unbounded.
    pub const fn end_t(&self) -> Option<LogicalTime> {
        self.end_t
    }

    /// Reports whether a logical time belongs to this half-open piece.
    pub fn contains(&self, logical_time: LogicalTime) -> bool {
        self.start_t.is_none_or(|start_t| start_t <= logical_time)
            && self.end_t.is_none_or(|end_t| logical_time < end_t)
    }

    /// Borrows the opaque piece payload.
    pub fn payload(&self) -> &P {
        &self.payload
    }

    /// Consumes the piece and returns its bounds and payload.
    pub fn into_parts(self) -> (Option<LogicalTime>, Option<LogicalTime>, P) {
        (self.start_t, self.end_t, self.payload)
    }

    fn from_sorted_bounds(
        start_t: Option<LogicalTime>,
        end_t: Option<LogicalTime>,
        payload: P,
    ) -> Self {
        debug_assert!(match (start_t, end_t) {
            (Some(start_t), Some(end_t)) => start_t < end_t,
            _ => true,
        });

        Self {
            start_t,
            end_t,
            payload,
        }
    }
}

/// An invalid pair of finite piece endpoints.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PieceBoundsError {
    /// The left endpoint is not strictly before the right endpoint.
    StartNotBeforeEnd {
        start_t: LogicalTime,
        end_t: LogicalTime,
    },
}

impl fmt::Display for PieceBoundsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StartNotBeforeEnd { start_t, end_t } => {
                write!(
                    formatter,
                    "piece start {start_t} must be before end {end_t}"
                )
            }
        }
    }
}

impl std::error::Error for PieceBoundsError {}

/// An immutable ordered breakpoint index and its gap-free half-open pieces.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DiscontinuityIndex<B = (), P = ()> {
    breakpoints: Vec<Breakpoint<B>>,
    boundary_times: Vec<LogicalTime>,
    pieces: Vec<Piece<P>>,
}

impl<B, P> DiscontinuityIndex<B, P> {
    /// Builds an index from breakpoints and one payload for every derived
    /// piece, in chronological piece order.
    ///
    /// Multiple breakpoints may share one timestamp. They remain separate
    /// breakpoint values, while that timestamp contributes one piece
    /// boundary. An empty breakpoint set therefore requires one payload for
    /// the unbounded piece `[-infinity, +infinity)`.
    pub fn from_breakpoints(
        breakpoints: impl IntoIterator<Item = Breakpoint<B>>,
        piece_payloads: impl IntoIterator<Item = P>,
    ) -> Result<Self, DiscontinuityIndexError> {
        let mut breakpoints = breakpoints.into_iter().collect::<Vec<_>>();
        breakpoints.sort_by(compare_breakpoints);

        let mut boundary_times = Vec::new();
        for breakpoint in &breakpoints {
            let logical_time = breakpoint.logical_time();
            if boundary_times.last().copied() != Some(logical_time) {
                boundary_times.push(logical_time);
            }
        }

        let expected_piece_count = boundary_times.len() + 1;
        let mut piece_payloads = piece_payloads.into_iter().collect::<Vec<_>>();
        if piece_payloads.len() != expected_piece_count {
            return Err(DiscontinuityIndexError {
                expected: expected_piece_count,
                actual: piece_payloads.len(),
            });
        }

        let mut pieces = Vec::with_capacity(expected_piece_count);
        let mut payloads = piece_payloads.drain(..);
        for piece_index in 0..expected_piece_count {
            let start_t = piece_index
                .checked_sub(1)
                .and_then(|boundary_index| boundary_times.get(boundary_index).copied());
            let end_t = boundary_times.get(piece_index).copied();
            let payload = payloads
                .next()
                .expect("piece payload count was checked before construction");
            pieces.push(Piece::from_sorted_bounds(start_t, end_t, payload));
        }

        Ok(Self {
            breakpoints,
            boundary_times,
            pieces,
        })
    }

    /// Builds an index using the same arguments as `from_breakpoints`.
    pub fn new(
        breakpoints: impl IntoIterator<Item = Breakpoint<B>>,
        piece_payloads: impl IntoIterator<Item = P>,
    ) -> Result<Self, DiscontinuityIndexError> {
        Self::from_breakpoints(breakpoints, piece_payloads)
    }

    /// Reports whether no breakpoint sources were indexed.
    pub fn is_empty(&self) -> bool {
        self.breakpoints.is_empty()
    }

    /// Returns the number of preserved breakpoint sources.
    pub fn breakpoint_count(&self) -> usize {
        self.breakpoints.len()
    }

    /// Returns the number of gap-free half-open pieces.
    pub fn piece_count(&self) -> usize {
        self.pieces.len()
    }

    /// Borrows all breakpoints in index order.
    pub fn breakpoints(&self) -> &[Breakpoint<B>] {
        &self.breakpoints
    }

    /// Borrows all breakpoint sources at one exact timestamp.
    pub fn breakpoints_at(
        &self,
        logical_time: LogicalTime,
    ) -> impl Iterator<Item = &Breakpoint<B>> {
        self.breakpoints
            .iter()
            .filter(move |breakpoint| breakpoint.logical_time() == logical_time)
    }

    /// Borrows the unique sorted boundary timestamps.
    pub fn boundary_times(&self) -> &[LogicalTime] {
        &self.boundary_times
    }

    /// Borrows all half-open pieces in chronological order.
    pub fn pieces(&self) -> &[Piece<P>] {
        &self.pieces
    }

    /// Returns the zero-based piece index containing the requested time.
    pub fn selected_piece_index(&self, logical_time: LogicalTime) -> usize {
        let mut low = 0;
        let mut high = self.boundary_times.len();

        while low < high {
            let middle = low + (high - low) / 2;
            if self.boundary_times[middle] <= logical_time {
                low = middle + 1;
            } else {
                high = middle;
            }
        }

        low
    }

    /// Returns the unique half-open piece containing the requested time.
    pub fn select(&self, logical_time: LogicalTime) -> &Piece<P> {
        &self.pieces[self.selected_piece_index(logical_time)]
    }
}

/// The number of payloads supplied for derived pieces did not match the
/// number of unique breakpoint boundaries plus one.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DiscontinuityIndexError {
    /// The number of payloads required by the derived partition.
    pub expected: usize,
    /// The number of payloads supplied by the caller.
    pub actual: usize,
}

impl fmt::Display for DiscontinuityIndexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "expected {} piece payloads, received {}",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for DiscontinuityIndexError {}

fn compare_breakpoints<B>(left: &Breakpoint<B>, right: &Breakpoint<B>) -> Ordering {
    left.logical_time()
        .cmp(&right.logical_time())
        .then_with(|| match (left.source(), right.source()) {
            (
                BreakpointSource::Journal {
                    append_ordinal: left_ordinal,
                },
                BreakpointSource::Journal {
                    append_ordinal: right_ordinal,
                },
            ) => left_ordinal.cmp(&right_ordinal),
            _ => Ordering::Equal,
        })
}
