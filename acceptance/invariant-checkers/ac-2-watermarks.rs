// AC-2 fixture: compiled as an integration test of a scratch crate that depends on facet-core.
//
// API this contract fixes:
//   facet_core::invariants::{Violation, check_watermarks}
//   check_watermarks(&[(Lsn, Lsn)]) -> Result<(), Violation>   // (layout watermark, last_durable)
//   Violation::WatermarkRegressed { index: usize, from: Lsn, to: Lsn }
//   Violation::WatermarkPastDurable { index: usize, watermark: Lsn, durable: Lsn }
//   Violation: Debug + Clone + PartialEq + Display
use facet_core::invariants::{check_watermarks, Violation};
use facet_core::Lsn;

fn assert_violation_traits<T: std::fmt::Debug + Clone + PartialEq + std::fmt::Display>() {}

fn assert_reported(v: &Violation) {
    assert_eq!(&v.clone(), v);
    assert!(!v.to_string().trim().is_empty(), "Display of {v:?} is empty");
}

fn obs(pairs: &[(u64, u64)]) -> Vec<(Lsn, Lsn)> {
    pairs.iter().map(|&(w, d)| (Lsn(w), Lsn(d))).collect()
}

#[test]
fn ac_2_violation_has_required_traits() {
    assert_violation_traits::<Violation>();
}

#[test]
fn ac_2_monotonic_watermarks_within_durable_pass() {
    assert_eq!(
        check_watermarks(&obs(&[(0, 10), (5, 10), (5, 12), (12, 12)])),
        Ok(())
    );
}

#[test]
fn ac_2_regressing_watermark_is_rejected() {
    let err = check_watermarks(&obs(&[(6, 10), (4, 10)])).unwrap_err();
    assert_eq!(
        err,
        Violation::WatermarkRegressed {
            index: 1,
            from: Lsn(6),
            to: Lsn(4),
        }
    );
    assert_reported(&err);
}

#[test]
fn ac_2_watermark_past_durable_is_rejected() {
    let err = check_watermarks(&obs(&[(3, 10), (11, 10)])).unwrap_err();
    assert_eq!(
        err,
        Violation::WatermarkPastDurable {
            index: 1,
            watermark: Lsn(11),
            durable: Lsn(10),
        }
    );
    assert_reported(&err);
}
