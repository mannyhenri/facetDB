// AC-4 fixture: compiled as an integration test of a scratch crate that depends on facet-core.
//
// API this contract fixes:
//   facet_core::invariants::{Violation, check_segments_unchanged, segment_digest}
//   segment_digest(&[u8]) -> u64   // FNV-1a 64-bit
//   check_segments_unchanged(before: &[(u64, u64)], after: &[(u64, u64)]) -> Result<(), Violation>
//     over (segment id, digest) pairs
//   Violation::SegmentModified { id: u64, before: u64, after: u64 }
//   Violation: Debug + Clone + PartialEq + Display
use facet_core::invariants::{check_segments_unchanged, segment_digest, Violation};

fn assert_violation_traits<T: std::fmt::Debug + Clone + PartialEq + std::fmt::Display>() {}

#[test]
fn ac_4_violation_has_required_traits() {
    assert_violation_traits::<Violation>();
}

#[test]
fn ac_4_digest_is_deterministic_and_fnv1a_64() {
    let d: u64 = segment_digest(b"abc");
    assert_eq!(d, segment_digest(b"abc"));
    assert_ne!(d, segment_digest(b"abd"));
    // Published FNV-1a 64-bit test vectors.
    assert_eq!(segment_digest(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(segment_digest(b"a"), 0xaf63_dc4c_8601_ec8c);
}

#[test]
fn ac_4_new_segments_are_allowed() {
    let (d1, d2, d3) = (
        segment_digest(b"seg-1"),
        segment_digest(b"seg-2"),
        segment_digest(b"seg-3"),
    );
    assert_eq!(
        check_segments_unchanged(&[(1, d1), (2, d2)], &[(1, d1), (2, d2), (3, d3)]),
        Ok(())
    );
}

#[test]
fn ac_4_removed_segments_are_allowed() {
    let (d1, d2) = (segment_digest(b"seg-1"), segment_digest(b"seg-2"));
    assert_eq!(check_segments_unchanged(&[(1, d1), (2, d2)], &[(2, d2)]), Ok(()));
}

#[test]
fn ac_4_edited_segment_is_rejected() {
    let d1 = segment_digest(b"seg-1");
    let dx = segment_digest(b"seg-1 edited");
    assert_ne!(d1, dx);
    let err = check_segments_unchanged(&[(1, d1)], &[(1, dx)]).unwrap_err();
    assert_eq!(
        err,
        Violation::SegmentModified {
            id: 1,
            before: d1,
            after: dx,
        }
    );
    assert_eq!(err.clone(), err);
    assert!(!err.to_string().trim().is_empty(), "Display of {err:?} is empty");
}
