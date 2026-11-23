//! A small fixed-length bitset, used for selection and validity in column batches.

/// A fixed number of bits, packed into `u64` words. Bits past `len` are always zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bitmap {
    words: Vec<u64>,
    len: usize,
}

impl Bitmap {
    /// `len` bits, all set.
    pub fn all_set(len: usize) -> Self {
        let mut words = vec![u64::MAX; len.div_ceil(64)];
        let tail = len % 64;
        if tail > 0 {
            // tail > 0 implies at least one word.
            *words.last_mut().unwrap() = (1u64 << tail) - 1;
        }
        Bitmap { words, len }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Panics if `i >= len`.
    pub fn get(&self, i: usize) -> bool {
        self.check(i);
        self.words[i / 64] & (1 << (i % 64)) != 0
    }

    /// Panics if `i >= len`.
    pub fn set(&mut self, i: usize, value: bool) {
        self.check(i);
        let mask = 1 << (i % 64);
        if value {
            self.words[i / 64] |= mask;
        } else {
            self.words[i / 64] &= !mask;
        }
    }

    /// Number of set bits.
    pub fn count_ones(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// Positions of the set bits, in increasing order.
    pub fn iter_ones(&self) -> impl Iterator<Item = usize> + '_ {
        self.words.iter().enumerate().flat_map(|(w, &word)| {
            let mut bits = word;
            std::iter::from_fn(move || {
                if bits == 0 {
                    return None;
                }
                let b = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                Some(w * 64 + b)
            })
        })
    }

    fn check(&self, i: usize) {
        assert!(
            i < self.len,
            "bit {i} out of range for bitmap of {}",
            self.len
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_set_counts_exactly_len() {
        for len in [0, 1, 63, 64, 65, 1024, 1025] {
            let b = Bitmap::all_set(len);
            assert_eq!(b.len(), len);
            assert_eq!(b.count_ones(), len);
            assert_eq!(b.is_empty(), len == 0);
        }
    }

    #[test]
    fn set_and_get() {
        let mut b = Bitmap::all_set(130);
        b.set(3, false);
        b.set(64, false);
        b.set(129, false);
        assert!(!b.get(3) && !b.get(64) && !b.get(129));
        assert!(b.get(0) && b.get(65) && b.get(128));
        assert_eq!(b.count_ones(), 127);
        b.set(3, true);
        assert!(b.get(3));
        assert_eq!(b.count_ones(), 128);
        assert_eq!(
            b.iter_ones().filter(|&i| i >= 128).collect::<Vec<_>>(),
            vec![128]
        );
    }

    #[test]
    fn iter_ones_matches_get_across_words() {
        let mut b = Bitmap::all_set(200);
        for i in (0..200).filter(|i| i % 3 != 0 || (64..128).contains(i)) {
            b.set(i, false);
        }
        let expected: Vec<usize> = (0..200).filter(|&i| b.get(i)).collect();
        assert_eq!(b.iter_ones().collect::<Vec<_>>(), expected);
        assert_eq!(expected.len(), b.count_ones());
        assert_eq!(Bitmap::all_set(0).iter_ones().count(), 0);
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn get_past_len_panics() {
        Bitmap::all_set(64).get(64);
    }
}
