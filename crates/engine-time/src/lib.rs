#![forbid(unsafe_code)]

mod logical_time;
mod scale;
mod tau;

pub use logical_time::LogicalTime;
pub use tau::Tau;

/// The number of logical ticks in one logical second.
pub const TICKS_PER_LOGICAL_SECOND: i64 = scale::TICKS_PER_LOGICAL_SECOND;

/// The anchor relation for the cellular automata: one game tick is one logical
/// second/unit.
pub const GAME_TICK_PERIOD: LogicalTime = LogicalTime::from_ticks(scale::GAME_TICK_PERIOD_TICKS);
