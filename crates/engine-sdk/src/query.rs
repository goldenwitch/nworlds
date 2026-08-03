/// The typed outcome of a value-producing query.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum QueryResult<T, O> {
    /// The requested value exists.
    Value(T),
    /// The requested value is outside the definition's domain.
    OutOfDomain(O),
}

impl<T, O> QueryResult<T, O> {
    /// Reports whether this result contains a value.
    pub fn is_value(&self) -> bool {
        matches!(self, Self::Value(_))
    }

    /// Reports whether this result is outside the definition's domain.
    pub fn is_out_of_domain(&self) -> bool {
        matches!(self, Self::OutOfDomain(_))
    }

    /// Converts a value result into an option, discarding the out-of-domain reason.
    pub fn value(self) -> Option<T> {
        match self {
            Self::Value(value) => Some(value),
            Self::OutOfDomain(_) => None,
        }
    }

    /// Converts an out-of-domain result into an option.
    pub fn out_of_domain(self) -> Option<O> {
        match self {
            Self::Value(_) => None,
            Self::OutOfDomain(reason) => Some(reason),
        }
    }

    /// Maps a value without changing its out-of-domain reason.
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> QueryResult<U, O> {
        match self {
            Self::Value(value) => QueryResult::Value(map(value)),
            Self::OutOfDomain(reason) => QueryResult::OutOfDomain(reason),
        }
    }
}
