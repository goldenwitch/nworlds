use std::collections::BTreeSet;

use caravan_domain::{ActorId, ActorKind, GameJournalEntry};
use engine_index::{
    Breakpoint, BreakpointSource, DiscontinuityIndex as EngineDiscontinuityIndex, Piece,
};
use engine_journal::{Journal, JournalEntry};
use engine_time::{game_tick_index, LogicalTime};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ActorThreshold {
    FarmerTerminal,
    ForesterMovement,
    ArsonistIgnition,
    FighterCollision,
    ArboristConversion,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuleThreshold {
    FireAge { age_in_game_ticks: u32 },
    WheatResource,
    WoodResource,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CaravanBreakpointSource {
    CreateSaucer {
        append_ordinal: usize,
    },
    JournalEntry {
        append_ordinal: usize,
    },
    GameTick {
        tick_index: i64,
    },
    ActorThreshold {
        actor_id: ActorId,
        kind: ActorKind,
        threshold: ActorThreshold,
    },
    RuleThreshold {
        threshold: RuleThreshold,
    },
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PieceInput {
    visible_entry_count: usize,
    tick_index_at_start: Option<i64>,
    projection: PieceProjection,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum PieceProjection {
    JournalVisible,
    TickIndexed,
}

impl PieceInput {
    pub const fn visible_entry_count(self) -> usize {
        self.visible_entry_count
    }

    pub const fn tick_index_at_start(self) -> Option<i64> {
        self.tick_index_at_start
    }

    pub(crate) const fn is_tick_indexed(self) -> bool {
        matches!(self.projection, PieceProjection::TickIndexed)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DiscontinuityIndex {
    entries: Vec<JournalEntry>,
    index: EngineDiscontinuityIndex<CaravanBreakpointSource, PieceInput>,
}

impl DiscontinuityIndex {
    pub fn new(journal: &Journal) -> Self {
        Self::build(journal, None)
    }

    pub fn breakpoint_count(&self) -> usize {
        self.index.breakpoint_count()
    }

    pub fn breakpoints(&self) -> &[Breakpoint<CaravanBreakpointSource>] {
        self.index.breakpoints()
    }

    pub fn breakpoints_at(
        &self,
        logical_time: LogicalTime,
    ) -> impl Iterator<Item = &Breakpoint<CaravanBreakpointSource>> {
        self.index.breakpoints_at(logical_time)
    }

    pub fn boundary_times(&self) -> &[LogicalTime] {
        self.index.boundary_times()
    }

    pub fn pieces(&self) -> &[Piece<PieceInput>] {
        self.index.pieces()
    }

    pub fn selected_piece_index(&self, logical_time: LogicalTime) -> usize {
        self.index.selected_piece_index(logical_time)
    }

    pub fn select(&self, logical_time: LogicalTime) -> &Piece<PieceInput> {
        self.index.select(logical_time)
    }

    pub(crate) fn for_sample(journal: &Journal, logical_time: LogicalTime) -> Self {
        Self::build(journal, Some(logical_time))
    }

    pub(crate) fn entries_for(&self, piece: &Piece<PieceInput>) -> &[JournalEntry] {
        let count = piece.payload().visible_entry_count();
        &self.entries[..count]
    }

    fn build(journal: &Journal, sample_time: Option<LogicalTime>) -> Self {
        let entries = journal.iter().cloned().collect::<Vec<_>>();
        let mut breakpoints = Vec::new();
        let mut tick_indices = BTreeSet::from([0]);

        for (append_ordinal, entry) in entries.iter().enumerate() {
            let logical_time = entry.logical_time();
            breakpoints.push(Breakpoint::new(
                logical_time,
                BreakpointSource::Journal { append_ordinal },
                CaravanBreakpointSource::JournalEntry { append_ordinal },
            ));

            if matches!(entry.payload(), GameJournalEntry::CreateSaucer { .. }) {
                breakpoints.push(Breakpoint::derived(
                    logical_time,
                    CaravanBreakpointSource::CreateSaucer { append_ordinal },
                ));
            }

            let entry_tick = game_tick_index(logical_time);
            tick_indices.insert(entry_tick);
            if let Some(next_tick) = entry_tick.checked_add(1) {
                tick_indices.insert(next_tick);
            }

            if let GameJournalEntry::SpawnActor { id, kind, .. } = *entry.payload() {
                if let Some(first_transition_tick) = entry_tick.checked_add(1) {
                    if let Some(first_transition_time) =
                        LogicalTime::from_game_ticks(first_transition_tick)
                    {
                        tick_indices.insert(first_transition_tick);
                        if let Some(threshold) = actor_threshold(kind) {
                            breakpoints.push(Breakpoint::derived(
                                first_transition_time,
                                CaravanBreakpointSource::ActorThreshold {
                                    actor_id: id,
                                    kind,
                                    threshold,
                                },
                            ));
                        }
                    }
                }

                if kind == ActorKind::Arborist {
                    if let Some(conversion_tick) = entry_tick.checked_add(3) {
                        if let Some(conversion_time) = LogicalTime::from_game_ticks(conversion_tick)
                        {
                            tick_indices.insert(conversion_time.game_tick_index());
                            breakpoints.push(Breakpoint::derived(
                                conversion_time,
                                CaravanBreakpointSource::ActorThreshold {
                                    actor_id: id,
                                    kind,
                                    threshold: ActorThreshold::ArboristConversion,
                                },
                            ));
                        }
                    }
                }
            }
        }

        if let Some(logical_time) = sample_time {
            tick_indices.insert(game_tick_index(logical_time));
        }

        for tick_index in tick_indices {
            let Some(logical_time) = LogicalTime::from_game_ticks(tick_index) else {
                continue;
            };

            breakpoints.push(Breakpoint::new(
                logical_time,
                BreakpointSource::GameTick { tick_index },
                CaravanBreakpointSource::GameTick { tick_index },
            ));
            breakpoints.extend((0..=3).map(|age_in_game_ticks| {
                Breakpoint::derived(
                    logical_time,
                    CaravanBreakpointSource::RuleThreshold {
                        threshold: RuleThreshold::FireAge { age_in_game_ticks },
                    },
                )
            }));
            breakpoints.push(Breakpoint::derived(
                logical_time,
                CaravanBreakpointSource::RuleThreshold {
                    threshold: RuleThreshold::WheatResource,
                },
            ));
            breakpoints.push(Breakpoint::derived(
                logical_time,
                CaravanBreakpointSource::RuleThreshold {
                    threshold: RuleThreshold::WoodResource,
                },
            ));
        }

        let boundary_times = breakpoints
            .iter()
            .map(|breakpoint| breakpoint.logical_time())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        let piece_inputs = (0..=boundary_times.len())
            .map(|piece_index| {
                let start_t = piece_index
                    .checked_sub(1)
                    .and_then(|boundary_index| boundary_times.get(boundary_index).copied());
                let visible_entry_count = start_t.map_or(0, |start| {
                    entries
                        .iter()
                        .take_while(|entry| entry.logical_time() <= start)
                        .count()
                });

                PieceInput {
                    visible_entry_count,
                    tick_index_at_start: start_t.map(game_tick_index),
                    projection: match start_t.map(game_tick_index) {
                        Some(tick_index) if tick_index >= 0 => PieceProjection::TickIndexed,
                        _ => PieceProjection::JournalVisible,
                    },
                }
            })
            .collect::<Vec<_>>();

        let index = EngineDiscontinuityIndex::from_breakpoints(breakpoints, piece_inputs)
            .expect("Caravan breakpoint construction supplies one input per piece");

        Self { entries, index }
    }
}

pub fn discontinuity_index(journal: &Journal) -> DiscontinuityIndex {
    DiscontinuityIndex::new(journal)
}

fn actor_threshold(kind: ActorKind) -> Option<ActorThreshold> {
    match kind {
        ActorKind::Farmer => Some(ActorThreshold::FarmerTerminal),
        ActorKind::Forester => Some(ActorThreshold::ForesterMovement),
        ActorKind::Arsonist => Some(ActorThreshold::ArsonistIgnition),
        ActorKind::Fighter => Some(ActorThreshold::FighterCollision),
        ActorKind::Arborist => None,
    }
}
