use core::fmt;

/// A logical-time value represented by signed fixed-point ticks.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LogicalTime(i64);

impl LogicalTime {
    /// Constructs a logical time from its signed tick representation.
    pub const fn from_ticks(ticks: i64) -> Self {
        Self(ticks)
    }

    /// Returns the signed tick representation.
    pub const fn ticks(self) -> i64 {
        self.0
    }

    /// Constructs logical time at a game-tick boundary if it is representable.
    pub const fn from_game_ticks(game_ticks: i64) -> Option<Self> {
        match game_ticks.checked_mul(crate::TICKS_PER_LOGICAL_SECOND) {
            Some(ticks) => Some(Self::from_ticks(ticks)),
            None => None,
        }
    }

    /// Returns the floor-indexed automaton game tick containing this sample.
    pub const fn game_tick_index(self) -> i64 {
        self.0.div_euclid(crate::TICKS_PER_LOGICAL_SECOND)
    }

    /// Returns logical time zero.
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Adds two logical times if the result is representable.
    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match self.0.checked_add(other.0) {
            Some(ticks) => Some(Self(ticks)),
            None => None,
        }
    }

    /// Subtracts two logical times if the result is representable.
    pub const fn checked_sub(self, other: Self) -> Option<Self> {
        match self.0.checked_sub(other.0) {
            Some(ticks) => Some(Self(ticks)),
            None => None,
        }
    }

    /// Adds signed ticks if the result is representable.
    pub const fn checked_add_ticks(self, ticks: i64) -> Option<Self> {
        match self.0.checked_add(ticks) {
            Some(ticks) => Some(Self(ticks)),
            None => None,
        }
    }

    /// Subtracts signed ticks if the result is representable.
    pub const fn checked_sub_ticks(self, ticks: i64) -> Option<Self> {
        match self.0.checked_sub(ticks) {
            Some(ticks) => Some(Self(ticks)),
            None => None,
        }
    }

    /// Multiplies by a signed scalar if the result is representable.
    pub const fn checked_mul(self, scalar: i64) -> Option<Self> {
        match self.0.checked_mul(scalar) {
            Some(ticks) => Some(Self(ticks)),
            None => None,
        }
    }

    /// Negates the value if the result is representable.
    pub const fn checked_neg(self) -> Option<Self> {
        match self.0.checked_neg() {
            Some(ticks) => Some(Self(ticks)),
            None => None,
        }
    }
}

impl fmt::Display for LogicalTime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::LogicalTime;
    use crate::{GAME_TICK_PERIOD, TICKS_PER_LOGICAL_SECOND};

    #[test]
    fn construction_and_zero_are_exact() {
        assert_eq!(LogicalTime::from_ticks(42).ticks(), 42);
        assert_eq!(LogicalTime::zero(), LogicalTime::from_ticks(0));
    }

    #[test]
    fn equality_and_order_are_tick_order() {
        let earlier = LogicalTime::from_ticks(-2);
        let equal = LogicalTime::from_ticks(-2);
        let later = LogicalTime::from_ticks(3);

        assert_eq!(earlier, equal);
        assert!(earlier < later);
        assert!(later > equal);
    }

    #[test]
    fn checked_arithmetic_preserves_signed_ticks() {
        let value = LogicalTime::from_ticks(-10);

        assert_eq!(
            value.checked_add(LogicalTime::from_ticks(4)),
            Some(LogicalTime::from_ticks(-6))
        );
        assert_eq!(
            value.checked_sub(LogicalTime::from_ticks(4)),
            Some(LogicalTime::from_ticks(-14))
        );
        assert_eq!(
            value.checked_add_ticks(4),
            Some(LogicalTime::from_ticks(-6))
        );
        assert_eq!(
            value.checked_sub_ticks(4),
            Some(LogicalTime::from_ticks(-14))
        );
        assert_eq!(value.checked_mul(2), Some(LogicalTime::from_ticks(-20)));
        assert_eq!(value.checked_neg(), Some(LogicalTime::from_ticks(10)));
    }

    #[test]
    fn checked_arithmetic_reports_overflow_and_underflow() {
        let maximum = LogicalTime::from_ticks(i64::MAX);
        let minimum = LogicalTime::from_ticks(i64::MIN);
        let one = LogicalTime::from_ticks(1);

        assert_eq!(maximum.checked_add(one), None);
        assert_eq!(minimum.checked_sub(one), None);
        assert_eq!(maximum.checked_mul(2), None);
        assert_eq!(minimum.checked_neg(), None);
    }

    #[test]
    fn arbitrary_negative_values_are_supported() {
        let value = LogicalTime::from_ticks(-9_876_543_210);

        assert_eq!(value.ticks(), -9_876_543_210);
        assert!(value < LogicalTime::zero());
    }

    #[test]
    fn game_tick_anchor_is_one_logical_second() {
        assert_eq!(TICKS_PER_LOGICAL_SECOND, 1_000);
        assert_eq!(
            GAME_TICK_PERIOD,
            LogicalTime::from_game_ticks(1).expect("one game tick is representable")
        );
        assert_eq!(LogicalTime::from_ticks(999).game_tick_index(), 0);
        assert_eq!(LogicalTime::from_ticks(1_000).game_tick_index(), 1);
    }

    #[test]
    fn game_tick_conversion_reports_fixed_point_overflow() {
        assert_eq!(LogicalTime::from_game_ticks(i64::MAX), None);
    }
}
