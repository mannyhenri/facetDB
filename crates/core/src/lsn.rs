//! Log sequence numbers.

/// Position of a change in the ordered log. A newtype so it cannot be mixed up with row counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Lsn(pub u64);

impl Lsn {
    /// The LSN before any change has been written.
    pub const ZERO: Lsn = Lsn(0);

    /// The LSN directly after this one. Panics on overflow, which a real log never reaches.
    pub fn next(self) -> Lsn {
        Lsn(self.0.checked_add(1).expect("LSN overflow"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_and_next() {
        assert_eq!(Lsn::ZERO, Lsn(0));
        assert_eq!(Lsn::ZERO.next(), Lsn(1));
        assert_eq!(Lsn(41).next(), Lsn(42));
    }

    #[test]
    fn ordered_by_number() {
        assert!(Lsn(3) < Lsn(7));
        let mut v = vec![Lsn(9), Lsn(1), Lsn(5)];
        v.sort();
        assert_eq!(v, vec![Lsn(1), Lsn(5), Lsn(9)]);
    }
}
