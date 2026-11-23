//! Checkers for the five engine invariants (spec section 2), written before the code they guard.
//!
//! Each checker takes plain observations, not engine objects, so it can be proven now on
//! synthetic traces and called later from a real test: record what the log, applier, layout or
//! segment store did, then pass the record here. Every checker returns `Ok(())` or the first
//! [`Violation`] it finds (`check_same_rows` reports every differing key at once).
//!
//! **Rule:** the feature that owns an invariant writes its invariant test first, by calling the
//! checker below, before it writes the code the test guards.
//!
//! | Invariant | Checker | Feature that writes the real test first | Phase |
//! |---|---|---|---|
//! | A commit is acknowledged only after its log record is durable | [`check_ack_after_durable`] | Group commit / Log API | 1 |
//! | A watermark never moves backwards and never passes the log's last durable LSN | [`check_watermarks`] | Background applier | 3 |
//! | A query at snapshot S returns the same rows from every layout that can answer it | [`check_same_rows`] | Merged base-plus-delta read (Phase 3), then column and vector layouts (Phases 4–5) | 3–5 |
//! | Base segments are immutable: changes create new segments, never edit old ones | [`check_segments_unchanged`] with [`segment_digest`] | Row layout with sorted runs (Phase 3), and Column segments (Phase 4) | 3–4 |
//! | Calls across the layout contract carry at least 1,024 rows when that many are available | [`check_batch_sizes`] | Layout contract | 3 |

use std::cmp::Ordering;
use std::fmt;

use crate::{Lsn, Row, Schema, Value, BATCH_ROWS};

/// One step of an acknowledgement trace, in the order it happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AckEvent {
    /// The log's last durable LSN advanced to this value.
    Durable(Lsn),
    /// A commit with this LSN was acknowledged to its client.
    Acked(Lsn),
}

/// The first broken invariant a checker found, with what is needed to find the problem.
#[derive(Clone, Debug, PartialEq)]
pub enum Violation {
    /// `trace[index]` acknowledged `lsn` while the highest durable LSN so far was `durable`.
    AckBeforeDurable {
        index: usize,
        lsn: Lsn,
        durable: Lsn,
    },
    /// `observations[index]` has watermark `to`, lower than the previous watermark `from`.
    WatermarkRegressed { index: usize, from: Lsn, to: Lsn },
    /// `observations[index]` has a watermark past the log's last durable LSN at that moment.
    WatermarkPastDurable {
        index: usize,
        watermark: Lsn,
        durable: Lsn,
    },
    /// Primary keys whose rows are only in `a`, only in `b`, or in both but not equal
    /// (or duplicated on either side). Each list is sorted by key.
    RowsDiffer {
        only_in_a: Vec<Value>,
        only_in_b: Vec<Value>,
        different: Vec<Value>,
    },
    /// Segment `id` exists before and after, with a different digest.
    SegmentModified { id: u64, before: u64, after: u64 },
    /// `sizes[index]` is a batch with no rows.
    EmptyBatch { index: usize },
    /// `sizes[index]` has fewer than `BATCH_ROWS` rows although `remaining >= BATCH_ROWS`
    /// rows were left before it.
    BatchTooSmall {
        index: usize,
        size: usize,
        remaining: usize,
    },
    /// Fewer than `BATCH_ROWS` rows were left before `sizes[index]`, and it did not take all
    /// `remaining` of them.
    ShortNotFinal {
        index: usize,
        size: usize,
        remaining: usize,
    },
    /// The batch sizes sum to `actual` rows, but `expected` rows were available.
    TotalMismatch { expected: usize, actual: usize },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Violation::AckBeforeDurable {
                index,
                lsn,
                durable,
            } => write!(
                f,
                "event {index}: LSN {} acknowledged while durable LSN is {}",
                lsn.0, durable.0
            ),
            Violation::WatermarkRegressed { index, from, to } => write!(
                f,
                "observation {index}: watermark moved back from {} to {}",
                from.0, to.0
            ),
            Violation::WatermarkPastDurable {
                index,
                watermark,
                durable,
            } => write!(
                f,
                "observation {index}: watermark {} is past durable LSN {}",
                watermark.0, durable.0
            ),
            Violation::RowsDiffer {
                only_in_a,
                only_in_b,
                different,
            } => write!(
                f,
                "rows differ: only in a {only_in_a:?}, only in b {only_in_b:?}, different {different:?}"
            ),
            Violation::SegmentModified { id, before, after } => write!(
                f,
                "segment {id} was modified: digest {before:#018x} became {after:#018x}"
            ),
            Violation::EmptyBatch { index } => write!(f, "batch {index} is empty"),
            Violation::BatchTooSmall {
                index,
                size,
                remaining,
            } => write!(
                f,
                "batch {index} has {size} rows, fewer than {BATCH_ROWS}, while {remaining} rows remained"
            ),
            Violation::ShortNotFinal {
                index,
                size,
                remaining,
            } => write!(
                f,
                "batch {index} has {size} rows but did not take all {remaining} remaining rows"
            ),
            Violation::TotalMismatch { expected, actual } => write!(
                f,
                "batches carry {actual} rows in total, but {expected} were available"
            ),
        }
    }
}

impl std::error::Error for Violation {}

/// Checks that every acknowledged LSN was durable when it was acknowledged: an `Acked(n)` needs
/// a `Durable(d)` with `d >= n` earlier in the trace. Durability starts at `Lsn::ZERO`, and a
/// `Durable` lower than an earlier one does not lower it.
pub fn check_ack_after_durable(trace: &[AckEvent]) -> Result<(), Violation> {
    let mut durable = Lsn::ZERO;
    for (index, event) in trace.iter().enumerate() {
        match *event {
            AckEvent::Durable(lsn) => durable = durable.max(lsn),
            AckEvent::Acked(lsn) if lsn > durable => {
                return Err(Violation::AckBeforeDurable {
                    index,
                    lsn,
                    durable,
                })
            }
            AckEvent::Acked(_) => {}
        }
    }
    Ok(())
}

/// Checks a sequence of `(layout watermark, log last_durable)` pairs taken at the same moments:
/// the watermark never goes below the previous one and never exceeds `last_durable`.
/// A regression is reported before a pass of durable at the same index.
pub fn check_watermarks(observations: &[(Lsn, Lsn)]) -> Result<(), Violation> {
    let mut previous: Option<Lsn> = None;
    for (index, &(watermark, durable)) in observations.iter().enumerate() {
        if let Some(from) = previous {
            if watermark < from {
                return Err(Violation::WatermarkRegressed {
                    index,
                    from,
                    to: watermark,
                });
            }
        }
        if watermark > durable {
            return Err(Violation::WatermarkPastDurable {
                index,
                watermark,
                durable,
            });
        }
        previous = Some(watermark);
    }
    Ok(())
}

/// Checks that `a` and `b` hold the same rows, in any order, matched by primary key.
///
/// Values compare by bits for floats (`f64::total_cmp` / `f32::total_cmp` equality), so NaN
/// matches NaN and `0.0` does not match `-0.0`. A key that appears more than once on either
/// side counts as different. A row too short to have a key is keyed by `Value::Null`.
pub fn check_same_rows(schema: &Schema, a: &[Row], b: &[Row]) -> Result<(), Violation> {
    let a = sorted_by_key(schema, a);
    let b = sorted_by_key(schema, b);
    let (mut only_in_a, mut only_in_b, mut different) = (Vec::new(), Vec::new(), Vec::new());
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        let order = match (a.get(i), b.get(j)) {
            (Some(ka), Some(kb)) => cmp_values(ka.0, kb.0),
            (Some(_), None) => Ordering::Less,
            _ => Ordering::Greater,
        };
        match order {
            Ordering::Less => {
                only_in_a.push(a[i].0.clone());
                i = run_end(&a, i);
            }
            Ordering::Greater => {
                only_in_b.push(b[j].0.clone());
                j = run_end(&b, j);
            }
            Ordering::Equal => {
                let (end_a, end_b) = (run_end(&a, i), run_end(&b, j));
                let single = end_a - i == 1 && end_b - j == 1;
                if !single || !rows_equal(a[i].1, b[j].1) {
                    different.push(a[i].0.clone());
                }
                (i, j) = (end_a, end_b);
            }
        }
    }
    if only_in_a.is_empty() && only_in_b.is_empty() && different.is_empty() {
        Ok(())
    } else {
        Err(Violation::RowsDiffer {
            only_in_a,
            only_in_b,
            different,
        })
    }
}

/// Checks that no segment present both `before` and `after` an apply changed its digest.
/// Pairs are `(segment id, digest)`; new and removed ids are allowed. If an id is listed more
/// than once, every after-digest must equal every before-digest for it.
pub fn check_segments_unchanged(
    before: &[(u64, u64)],
    after: &[(u64, u64)],
) -> Result<(), Violation> {
    let mut after = after.to_vec();
    after.sort_by_key(|&(id, _)| id);
    for &(id, digest_before) in before {
        let start = after.partition_point(|&(i, _)| i < id);
        for &(_, digest_after) in after[start..].iter().take_while(|&&(i, _)| i == id) {
            if digest_after != digest_before {
                return Err(Violation::SegmentModified {
                    id,
                    before: digest_before,
                    after: digest_after,
                });
            }
        }
    }
    Ok(())
}

/// FNV-1a 64-bit hash of a segment's bytes, so a test records a digest instead of a copy.
/// Not cryptographic: it detects accidental edits, not deliberate collisions.
pub fn segment_digest(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes.iter().fold(OFFSET_BASIS, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(PRIME)
    })
}

/// Checks the batch sizes of one scan over `available` rows: no batch is empty; while at least
/// `BATCH_ROWS` rows remain, each batch carries at least `BATCH_ROWS`; once fewer remain, the
/// next batch takes all of them; and the sizes sum to `available`. Per-batch rules are checked
/// in order before the total.
pub fn check_batch_sizes(sizes: &[usize], available: usize) -> Result<(), Violation> {
    let mut remaining = available;
    for (index, &size) in sizes.iter().enumerate() {
        if size == 0 {
            return Err(Violation::EmptyBatch { index });
        }
        if remaining >= BATCH_ROWS && size < BATCH_ROWS {
            return Err(Violation::BatchTooSmall {
                index,
                size,
                remaining,
            });
        }
        if remaining < BATCH_ROWS && size < remaining {
            return Err(Violation::ShortNotFinal {
                index,
                size,
                remaining,
            });
        }
        // A batch larger than what remains is caught by the total below.
        remaining = remaining.saturating_sub(size);
    }
    let actual = sizes.iter().fold(0usize, |sum, &s| sum.saturating_add(s));
    if actual != available {
        return Err(Violation::TotalMismatch {
            expected: available,
            actual,
        });
    }
    Ok(())
}

static MISSING_KEY: Value = Value::Null;

/// Rows paired with their primary key, sorted by key.
fn sorted_by_key<'r>(schema: &Schema, rows: &'r [Row]) -> Vec<(&'r Value, &'r Row)> {
    let mut keyed: Vec<(&Value, &Row)> = rows
        .iter()
        .map(|row| (row.key(schema).unwrap_or(&MISSING_KEY), row))
        .collect();
    keyed.sort_unstable_by(|x, y| cmp_values(x.0, y.0));
    keyed
}

/// The index just past the run of rows that share the key at `start`.
fn run_end(sorted: &[(&Value, &Row)], start: usize) -> usize {
    let key = sorted[start].0;
    start
        + sorted[start..]
            .iter()
            .take_while(|(k, _)| cmp_values(k, key).is_eq())
            .count()
}

fn rows_equal(a: &Row, b: &Row) -> bool {
    a.0.len() == b.0.len() && a.0.iter().zip(&b.0).all(|(x, y)| cmp_values(x, y).is_eq())
}

/// A total order on values: by kind, then by value, floats by `total_cmp`.
fn cmp_values(a: &Value, b: &Value) -> Ordering {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.cmp(y),
        (Value::Float(x), Value::Float(y)) => x.total_cmp(y),
        (Value::Str(x), Value::Str(y)) => x.cmp(y),
        (Value::Bytes(x), Value::Bytes(y)) => x.cmp(y),
        (Value::Vector(x), Value::Vector(y)) => x
            .iter()
            .zip(y)
            .map(|(p, q)| p.total_cmp(q))
            .find(|o| o.is_ne())
            .unwrap_or_else(|| x.len().cmp(&y.len())),
        (Value::Null, Value::Null) => Ordering::Equal,
        _ => kind_rank(a).cmp(&kind_rank(b)),
    }
}

fn kind_rank(v: &Value) -> u8 {
    match v {
        Value::Int(_) => 0,
        Value::Float(_) => 1,
        Value::Str(_) => 2,
        Value::Bytes(_) => 3,
        Value::Vector(_) => 4,
        Value::Null => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColId, Column, DataType};
    use proptest::prelude::*;
    use AckEvent::{Acked, Durable};

    fn lsns(pairs: &[(u64, u64)]) -> Vec<(Lsn, Lsn)> {
        pairs.iter().map(|&(w, d)| (Lsn(w), Lsn(d))).collect()
    }

    // --- ack after durable ---

    #[test]
    fn ack_passes_when_every_ack_follows_durability() {
        assert_eq!(check_ack_after_durable(&[]), Ok(()));
        assert_eq!(
            check_ack_after_durable(&[Durable(Lsn(5)), Acked(Lsn(3)), Acked(Lsn(5))]),
            Ok(())
        );
        // Acking LSN 0 needs nothing durable; a lower Durable does not undo a higher one.
        assert_eq!(
            check_ack_after_durable(&[
                Acked(Lsn(0)),
                Durable(Lsn(7)),
                Durable(Lsn(2)),
                Acked(Lsn(7))
            ]),
            Ok(())
        );
    }

    #[test]
    fn ack_fails_before_its_lsn_is_durable() {
        assert_eq!(
            check_ack_after_durable(&[Durable(Lsn(4)), Acked(Lsn(5))]),
            Err(Violation::AckBeforeDurable {
                index: 1,
                lsn: Lsn(5),
                durable: Lsn(4)
            })
        );
        assert_eq!(
            check_ack_after_durable(&[Acked(Lsn(1))]),
            Err(Violation::AckBeforeDurable {
                index: 0,
                lsn: Lsn(1),
                durable: Lsn(0)
            })
        );
        // Durable after the ack is too late.
        assert_eq!(
            check_ack_after_durable(&[Acked(Lsn(3)), Durable(Lsn(3))]),
            Err(Violation::AckBeforeDurable {
                index: 0,
                lsn: Lsn(3),
                durable: Lsn(0)
            })
        );
    }

    // --- watermarks ---

    #[test]
    fn watermarks_pass_when_monotonic_and_within_durable() {
        assert_eq!(check_watermarks(&[]), Ok(()));
        assert_eq!(
            check_watermarks(&lsns(&[(0, 10), (5, 10), (5, 12), (12, 12)])),
            Ok(())
        );
    }

    #[test]
    fn watermark_fails_when_it_regresses() {
        assert_eq!(
            check_watermarks(&lsns(&[(6, 10), (4, 10)])),
            Err(Violation::WatermarkRegressed {
                index: 1,
                from: Lsn(6),
                to: Lsn(4)
            })
        );
    }

    #[test]
    fn watermark_fails_when_past_durable() {
        assert_eq!(
            check_watermarks(&lsns(&[(3, 10), (11, 10)])),
            Err(Violation::WatermarkPastDurable {
                index: 1,
                watermark: Lsn(11),
                durable: Lsn(10)
            })
        );
        assert_eq!(
            check_watermarks(&lsns(&[(1, 0)])),
            Err(Violation::WatermarkPastDurable {
                index: 0,
                watermark: Lsn(1),
                durable: Lsn(0)
            })
        );
    }

    // --- same rows ---

    fn price_schema() -> Schema {
        Schema::new(
            vec![
                Column::new("id", DataType::Int, false),
                Column::new("price", DataType::Float, true),
            ],
            ColId(0),
        )
        .unwrap()
    }

    fn row(id: i64, price: f64) -> Row {
        Row::new(vec![Value::Int(id), Value::Float(price)])
    }

    #[test]
    fn same_rows_pass_in_any_order() {
        let s = price_schema();
        assert_eq!(check_same_rows(&s, &[], &[]), Ok(()));
        let a = [row(1, 9.5), row(2, f64::NAN), row(3, -1.0)];
        let b = [row(3, -1.0), row(2, f64::NAN), row(1, 9.5)];
        assert_eq!(check_same_rows(&s, &a, &b), Ok(()));
        let nulls = [Row::new(vec![Value::Int(4), Value::Null])];
        assert_eq!(check_same_rows(&s, &nulls, &nulls), Ok(()));
    }

    #[test]
    fn same_rows_work_with_a_str_key_not_in_first_position() {
        let s = Schema::new(
            vec![
                Column::new("v", DataType::Vector { dim: 2 }, false),
                Column::new("name", DataType::Str, false),
            ],
            ColId(1),
        )
        .unwrap();
        let r =
            |v: [f32; 2], n: &str| Row::new(vec![Value::Vector(v.to_vec()), Value::Str(n.into())]);
        let a = [r([1.0, f32::NAN], "x"), r([0.0, 0.0], "y")];
        let b = [r([0.0, 0.0], "y"), r([1.0, f32::NAN], "x")];
        assert_eq!(check_same_rows(&s, &a, &b), Ok(()));
        assert_eq!(
            check_same_rows(&s, &a, &[r([1.0, 2.0], "x"), r([0.0, 0.0], "y")]),
            Err(Violation::RowsDiffer {
                only_in_a: vec![],
                only_in_b: vec![],
                different: vec![Value::Str("x".into())],
            })
        );
    }

    #[test]
    fn rows_differ_reports_every_key_sorted() {
        let s = price_schema();
        let a = [row(5, 1.0), row(1, 9.5), row(2, 1.0), row(4, 0.0)];
        let b = [
            row(1, 9.6),
            row(3, 1.0),
            row(5, 1.0),
            row(6, 1.0),
            row(4, -0.0),
        ];
        assert_eq!(
            check_same_rows(&s, &a, &b),
            Err(Violation::RowsDiffer {
                only_in_a: vec![Value::Int(2)],
                only_in_b: vec![Value::Int(3), Value::Int(6)],
                // 0.0 and -0.0 differ under total_cmp.
                different: vec![Value::Int(1), Value::Int(4)],
            })
        );
    }

    #[test]
    fn rows_differ_on_null_versus_value_and_duplicate_keys() {
        let s = price_schema();
        let null_price = Row::new(vec![Value::Int(1), Value::Null]);
        assert_eq!(
            check_same_rows(&s, &[null_price], &[row(1, 0.0)]),
            Err(Violation::RowsDiffer {
                only_in_a: vec![],
                only_in_b: vec![],
                different: vec![Value::Int(1)],
            })
        );
        // The same row twice on one side is not the same row set.
        assert_eq!(
            check_same_rows(&s, &[row(7, 1.0), row(7, 1.0)], &[row(7, 1.0)]),
            Err(Violation::RowsDiffer {
                only_in_a: vec![],
                only_in_b: vec![],
                different: vec![Value::Int(7)],
            })
        );
        assert_eq!(
            check_same_rows(&s, &[row(8, 1.0), row(8, 1.0)], &[]),
            Err(Violation::RowsDiffer {
                only_in_a: vec![Value::Int(8)],
                only_in_b: vec![],
                different: vec![],
            })
        );
    }

    #[test]
    fn a_row_without_a_key_is_keyed_by_null() {
        let s = price_schema();
        let short = Row::new(vec![]);
        assert_eq!(
            check_same_rows(&s, &[short], &[]),
            Err(Violation::RowsDiffer {
                only_in_a: vec![Value::Null],
                only_in_b: vec![],
                different: vec![],
            })
        );
    }

    #[test]
    fn cmp_values_orders_kinds_and_is_bitwise_for_floats() {
        assert!(cmp_values(&Value::Float(f64::NAN), &Value::Float(f64::NAN)).is_eq());
        assert!(cmp_values(&Value::Float(0.0), &Value::Float(-0.0)).is_ne());
        assert!(cmp_values(&Value::Vector(vec![1.0]), &Value::Vector(vec![1.0, 0.0])).is_lt());
        assert!(cmp_values(&Value::Int(9), &Value::Str("0".into())).is_lt());
        assert!(cmp_values(&Value::Bytes(vec![1]), &Value::Bytes(vec![2])).is_lt());
        assert!(cmp_values(&Value::Null, &Value::Null).is_eq());
    }

    #[test]
    fn cmp_values_orders_every_pair_of_kinds_by_rank() {
        // One sample per kind, in rank order; kind_rank's exhaustive match breaks the build
        // when a Value variant is added.
        let kinds = [
            Value::Int(i64::MAX),
            Value::Float(f64::MIN),
            Value::Str(String::new()),
            Value::Bytes(vec![]),
            Value::Vector(vec![]),
            Value::Null,
        ];
        for (i, x) in kinds.iter().enumerate() {
            for (j, y) in kinds.iter().enumerate() {
                assert_eq!(cmp_values(x, y), i.cmp(&j), "{x:?} vs {y:?}");
            }
        }
    }

    // --- segments ---

    #[test]
    fn segment_digest_is_fnv1a_64() {
        assert_eq!(segment_digest(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(segment_digest(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(segment_digest(b"foobar"), 0x8594_4171_f739_67e8);
        assert_eq!(segment_digest(b"abc"), segment_digest(b"abc"));
        assert_ne!(segment_digest(b"abc"), segment_digest(b"abd"));
    }

    #[test]
    fn segments_pass_when_added_or_removed() {
        let (d1, d2, d3) = (
            segment_digest(b"1"),
            segment_digest(b"2"),
            segment_digest(b"3"),
        );
        assert_eq!(check_segments_unchanged(&[], &[]), Ok(()));
        assert_eq!(
            check_segments_unchanged(&[(1, d1), (2, d2)], &[(3, d3), (2, d2), (1, d1)]),
            Ok(())
        );
        assert_eq!(
            check_segments_unchanged(&[(1, d1), (2, d2)], &[(2, d2)]),
            Ok(())
        );
        assert_eq!(check_segments_unchanged(&[(1, d1)], &[]), Ok(()));
    }

    #[test]
    fn segment_fails_when_edited() {
        let (d1, dx) = (segment_digest(b"seg"), segment_digest(b"seg!"));
        assert_eq!(
            check_segments_unchanged(&[(1, d1)], &[(1, dx)]),
            Err(Violation::SegmentModified {
                id: 1,
                before: d1,
                after: dx
            })
        );
        // A duplicate id with another digest after the apply is an edit too.
        assert_eq!(
            check_segments_unchanged(&[(9, 1), (4, 2)], &[(4, 2), (9, 1), (4, 3)]),
            Err(Violation::SegmentModified {
                id: 4,
                before: 2,
                after: 3
            })
        );
    }

    // --- batch sizes ---

    #[test]
    fn batch_sizes_pass_for_full_batches_then_the_remainder() {
        assert_eq!(check_batch_sizes(&[], 0), Ok(()));
        assert_eq!(check_batch_sizes(&[4096, 4096, 1808], 10_000), Ok(()));
        let mut exact = vec![1024; 9];
        exact.push(784);
        assert_eq!(check_batch_sizes(&exact, 10_000), Ok(()));
        assert_eq!(check_batch_sizes(&[500], 500), Ok(()));
        assert_eq!(check_batch_sizes(&[10_000], 10_000), Ok(()));
        assert_eq!(check_batch_sizes(&[1024, 1023], 2047), Ok(()));
    }

    #[test]
    fn empty_batch_fails() {
        assert_eq!(
            check_batch_sizes(&[5000, 0, 5000], 10_000),
            Err(Violation::EmptyBatch { index: 1 })
        );
        assert_eq!(
            check_batch_sizes(&[0], 0),
            Err(Violation::EmptyBatch { index: 0 })
        );
    }

    #[test]
    fn small_batch_fails_while_a_full_one_was_possible() {
        assert_eq!(
            check_batch_sizes(&[1000, 9000], 10_000),
            Err(Violation::BatchTooSmall {
                index: 0,
                size: 1000,
                remaining: 10_000
            })
        );
        assert_eq!(
            check_batch_sizes(&[1023, 1], 1024),
            Err(Violation::BatchTooSmall {
                index: 0,
                size: 1023,
                remaining: 1024
            })
        );
    }

    #[test]
    fn short_batch_fails_when_it_leaves_rows_behind() {
        assert_eq!(
            check_batch_sizes(&[9000, 600, 400], 10_000),
            Err(Violation::ShortNotFinal {
                index: 1,
                size: 600,
                remaining: 1000
            })
        );
    }

    #[test]
    fn totals_must_match_available() {
        assert_eq!(
            check_batch_sizes(&[5000, 4000], 10_000),
            Err(Violation::TotalMismatch {
                expected: 10_000,
                actual: 9000
            })
        );
        assert_eq!(
            check_batch_sizes(&[], 10),
            Err(Violation::TotalMismatch {
                expected: 10,
                actual: 0
            })
        );
        // Overshooting what remains, or a batch after the end, is a total mismatch.
        assert_eq!(
            check_batch_sizes(&[600], 500),
            Err(Violation::TotalMismatch {
                expected: 500,
                actual: 600
            })
        );
        assert_eq!(
            check_batch_sizes(&[500, 10], 500),
            Err(Violation::TotalMismatch {
                expected: 500,
                actual: 510
            })
        );
        assert_eq!(
            check_batch_sizes(&[usize::MAX, 2000], 2000),
            Err(Violation::TotalMismatch {
                expected: 2000,
                actual: usize::MAX
            })
        );
    }

    // --- Display, set-based over every variant ---

    // Fails to compile when a Violation variant is added, so `every_violation` must be updated.
    #[allow(dead_code)]
    fn every_violation_is_exhaustive(v: &Violation) {
        match v {
            Violation::AckBeforeDurable { .. }
            | Violation::WatermarkRegressed { .. }
            | Violation::WatermarkPastDurable { .. }
            | Violation::RowsDiffer { .. }
            | Violation::SegmentModified { .. }
            | Violation::EmptyBatch { .. }
            | Violation::BatchTooSmall { .. }
            | Violation::ShortNotFinal { .. }
            | Violation::TotalMismatch { .. } => {}
        }
    }

    fn every_violation() -> Vec<(Violation, &'static str)> {
        vec![
            (
                Violation::AckBeforeDurable {
                    index: 1,
                    lsn: Lsn(5),
                    durable: Lsn(4),
                },
                "event 1: LSN 5 acknowledged while durable LSN is 4",
            ),
            (
                Violation::WatermarkRegressed {
                    index: 1,
                    from: Lsn(6),
                    to: Lsn(4),
                },
                "observation 1: watermark moved back from 6 to 4",
            ),
            (
                Violation::WatermarkPastDurable {
                    index: 1,
                    watermark: Lsn(11),
                    durable: Lsn(10),
                },
                "observation 1: watermark 11 is past durable LSN 10",
            ),
            (
                Violation::RowsDiffer {
                    only_in_a: vec![Value::Int(2)],
                    only_in_b: vec![Value::Int(3)],
                    different: vec![Value::Int(1)],
                },
                "rows differ: only in a [Int(2)], only in b [Int(3)], different [Int(1)]",
            ),
            (
                Violation::SegmentModified {
                    id: 1,
                    before: 0xab,
                    after: 0xcd,
                },
                "segment 1 was modified: digest 0x00000000000000ab became 0x00000000000000cd",
            ),
            (Violation::EmptyBatch { index: 1 }, "batch 1 is empty"),
            (
                Violation::BatchTooSmall {
                    index: 0,
                    size: 1000,
                    remaining: 10_000,
                },
                "batch 0 has 1000 rows, fewer than 1024, while 10000 rows remained",
            ),
            (
                Violation::ShortNotFinal {
                    index: 1,
                    size: 600,
                    remaining: 1000,
                },
                "batch 1 has 600 rows but did not take all 1000 remaining rows",
            ),
            (
                Violation::TotalMismatch {
                    expected: 10_000,
                    actual: 9000,
                },
                "batches carry 9000 rows in total, but 10000 were available",
            ),
        ]
    }

    #[test]
    fn every_violation_displays_where_to_look() {
        let all = every_violation();
        assert_eq!(all.len(), 9);
        for (v, text) in all {
            assert_eq!(v.to_string(), text);
            assert_eq!(v.clone(), v);
        }
    }

    // --- proptests: generated valid traces must pass ---

    fn valid_ack_trace() -> impl Strategy<Value = Vec<AckEvent>> {
        proptest::collection::vec((any::<bool>(), 0u64..50), 0..100).prop_map(|steps| {
            let mut durable = 0u64;
            steps
                .into_iter()
                .map(|(advance, n)| {
                    if advance {
                        durable += n;
                        Durable(Lsn(durable))
                    } else {
                        Acked(Lsn(n % (durable + 1)))
                    }
                })
                .collect()
        })
    }

    fn valid_watermarks() -> impl Strategy<Value = Vec<(Lsn, Lsn)>> {
        proptest::collection::vec((0u64..5, 0u64..5), 0..100).prop_map(|steps| {
            let (mut watermark, mut durable) = (0u64, 0u64);
            steps
                .into_iter()
                .map(|(dw, dd)| {
                    durable += dd;
                    watermark = (watermark + dw).min(durable);
                    (Lsn(watermark), Lsn(durable))
                })
                .collect()
        })
    }

    // Floats include NaN, infinities and -0.0: the checker compares bits, so the same row
    // must always match itself.
    fn rows_and_shuffled_copy() -> impl Strategy<Value = (Vec<Row>, Vec<Row>)> {
        proptest::collection::btree_set(any::<i64>(), 0..50)
            .prop_flat_map(|keys| {
                let n = keys.len();
                (
                    Just(keys),
                    proptest::collection::vec(proptest::option::of(any::<f64>()), n),
                )
            })
            .prop_map(|(keys, prices)| {
                keys.into_iter()
                    .zip(prices)
                    .map(|(k, p)| {
                        Row::new(vec![Value::Int(k), p.map_or(Value::Null, Value::Float)])
                    })
                    .collect::<Vec<_>>()
            })
            .prop_flat_map(|rows| (Just(rows.clone()), Just(rows).prop_shuffle()))
    }

    fn segment_bytes() -> impl Strategy<Value = Vec<Vec<u8>>> {
        proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..32), 0..20)
    }

    // Segments 0..n before; after keeps a random subset and adds new ids n.., in reverse order.
    type Digests = Vec<(u64, u64)>;

    fn valid_segment_apply() -> impl Strategy<Value = (Digests, Digests)> {
        (
            segment_bytes(),
            proptest::collection::vec(any::<bool>(), 20),
            segment_bytes(),
        )
            .prop_map(|(old, keep, new)| {
                let before: Digests = old
                    .iter()
                    .enumerate()
                    .map(|(i, b)| (i as u64, segment_digest(b)))
                    .collect();
                let mut after: Digests = before
                    .iter()
                    .zip(&keep)
                    .filter(|(_, &k)| k)
                    .map(|(&p, _)| p)
                    .collect();
                let n = old.len() as u64;
                after.extend(
                    new.iter()
                        .enumerate()
                        .map(|(i, b)| (n + i as u64, segment_digest(b))),
                );
                after.reverse();
                (before, after)
            })
    }

    // Full batches of a random size while BATCH_ROWS remain, then the whole remainder.
    fn valid_batches() -> impl Strategy<Value = (Vec<usize>, usize)> {
        (
            0usize..20_000,
            proptest::collection::vec(any::<u32>(), 0..30),
        )
            .prop_map(|(available, seeds)| {
                let mut seeds = seeds.into_iter();
                let mut remaining = available;
                let mut sizes = Vec::new();
                while remaining > 0 {
                    let size = match seeds.next() {
                        Some(s) if remaining >= BATCH_ROWS => {
                            BATCH_ROWS + s as usize % (remaining - BATCH_ROWS + 1)
                        }
                        _ => remaining,
                    };
                    sizes.push(size);
                    remaining -= size;
                }
                (sizes, available)
            })
    }

    proptest! {
        #[test]
        fn valid_ack_traces_pass(trace in valid_ack_trace()) {
            prop_assert_eq!(check_ack_after_durable(&trace), Ok(()));
        }

        #[test]
        fn valid_watermarks_pass(observations in valid_watermarks()) {
            prop_assert_eq!(check_watermarks(&observations), Ok(()));
        }

        #[test]
        fn same_rows_in_any_order_pass((a, b) in rows_and_shuffled_copy()) {
            prop_assert_eq!(check_same_rows(&price_schema(), &a, &b), Ok(()));
        }

        #[test]
        fn added_and_removed_segments_pass((before, after) in valid_segment_apply()) {
            prop_assert_eq!(check_segments_unchanged(&before, &after), Ok(()));
        }

        #[test]
        fn valid_batch_sizes_pass((sizes, available) in valid_batches()) {
            prop_assert_eq!(check_batch_sizes(&sizes, available), Ok(()));
        }
    }
}
