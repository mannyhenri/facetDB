// AC-5 fixture: compiled as an integration test of a scratch crate that depends on facet-core.
//
// API this contract fixes:
//   facet_core::invariants::{Violation, check_batch_sizes}
//   check_batch_sizes(sizes: &[usize], available: usize) -> Result<(), Violation>  // uses BATCH_ROWS
//   Violation::EmptyBatch { index: usize }
//   Violation::BatchTooSmall { index: usize, size: usize, remaining: usize }
//   Violation::ShortNotFinal { index: usize, size: usize, remaining: usize }
//   Violation::TotalMismatch { expected: usize, actual: usize }
//   Violation: Debug + Clone + PartialEq + Display
use facet_core::invariants::{check_batch_sizes, Violation};
use facet_core::BATCH_ROWS;

fn assert_violation_traits<T: std::fmt::Debug + Clone + PartialEq + std::fmt::Display>() {}

fn assert_reported(v: &Violation) {
    assert_eq!(&v.clone(), v);
    assert!(!v.to_string().trim().is_empty(), "Display of {v:?} is empty");
}

fn rejects(sizes: &[usize], available: usize, expected: Violation) {
    let err = check_batch_sizes(sizes, available).unwrap_err();
    assert_eq!(err, expected, "sizes {sizes:?}, available {available}");
    assert_reported(&err);
}

#[test]
fn ac_5_violation_has_required_traits() {
    assert_violation_traits::<Violation>();
    assert_eq!(BATCH_ROWS, 1024);
}

#[test]
fn ac_5_valid_scans_pass() {
    assert_eq!(check_batch_sizes(&[4096, 4096, 1808], 10_000), Ok(()));
    let mut exact = vec![1024; 9];
    exact.push(784);
    assert_eq!(check_batch_sizes(&exact, 10_000), Ok(()));
    assert_eq!(check_batch_sizes(&[500], 500), Ok(()));
}

#[test]
fn ac_5_small_batches_are_rejected() {
    rejects(
        &[1000, 9000],
        10_000,
        Violation::BatchTooSmall {
            index: 0,
            size: 1000,
            remaining: 10_000,
        },
    );
    rejects(
        &[9000, 600, 400],
        10_000,
        Violation::ShortNotFinal {
            index: 1,
            size: 600,
            remaining: 1000,
        },
    );
}

#[test]
fn ac_5_empty_batch_is_rejected() {
    rejects(&[5000, 0, 5000], 10_000, Violation::EmptyBatch { index: 1 });
}

#[test]
fn ac_5_total_mismatch_is_rejected() {
    rejects(
        &[5000, 4000],
        10_000,
        Violation::TotalMismatch {
            expected: 10_000,
            actual: 9000,
        },
    );
}
