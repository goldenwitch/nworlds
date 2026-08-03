use core::fmt;

/// A presentation-time value represented by signed fixed-point ticks at the
/// same resolution as `LogicalTime`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Tau(i64);

impl Tau {
    /// Constructs presentation time from its signed tick representation.
    pub const fn from_ticks(ticks: i64) -> Self {
        Self(ticks)
    }

    /// Returns the signed tick representation.
    pub const fn ticks(self) -> i64 {
        self.0
    }

    /// Returns presentation time zero.
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Adds two presentation times if the result is representable.
    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match self.0.checked_add(other.0) {
            Some(ticks) => Some(Self(ticks)),
            None => None,
        }
    }

    /// Subtracts two presentation times if the result is representable.
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

impl fmt::Display for Tau {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::Tau;

    #[test]
    fn construction_zero_equality_and_order_are_exact() {
        let zero = Tau::zero();
        let negative = Tau::from_ticks(-4);
        let equal = Tau::from_ticks(-4);
        let positive = Tau::from_ticks(8);

        assert_eq!(zero.ticks(), 0);
        assert_eq!(negative, equal);
        assert!(negative < zero);
        assert!(positive > equal);
    }

    #[test]
    fn checked_arithmetic_handles_negative_values() {
        let value = Tau::from_ticks(-12);

        assert_eq!(
            value.checked_add(Tau::from_ticks(5)),
            Some(Tau::from_ticks(-7))
        );
        assert_eq!(
            value.checked_sub(Tau::from_ticks(5)),
            Some(Tau::from_ticks(-17))
        );
        assert_eq!(value.checked_neg(), Some(Tau::from_ticks(12)));
    }

    #[test]
    fn checked_arithmetic_reports_overflow_and_underflow() {
        let maximum = Tau::from_ticks(i64::MAX);
        let minimum = Tau::from_ticks(i64::MIN);
        let one = Tau::from_ticks(1);

        assert_eq!(maximum.checked_add(one), None);
        assert_eq!(minimum.checked_sub(one), None);
    }

    #[test]
    fn arbitrary_negative_values_are_supported() {
        assert_eq!(Tau::from_ticks(-9_876_543_210).ticks(), -9_876_543_210);
    }

    #[test]
    fn distinct_subsecond_ticks_are_representable() {
        let half_second = Tau::from_ticks(crate::TICKS_PER_LOGICAL_SECOND / 2);

        assert_ne!(Tau::zero(), half_second);
        assert_eq!(half_second.ticks(), 500);
    }
}
