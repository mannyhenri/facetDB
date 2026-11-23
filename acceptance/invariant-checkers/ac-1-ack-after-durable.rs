// AC-1 fixture: compiled as an integration test of a scratch crate that depends on facet-core.
//
// API this contract fixes:
//   facet_core::invariants::{AckEvent, Violation, check_ack_after_durable}
//   enum AckEvent { Durable(Lsn), Acked(Lsn) }
//   check_ack_after_durable(&[AckEvent]) -> Result<(), Violation>
//   Violation::AckBeforeDurable { index: usize, lsn: Lsn, durable: Lsn }
//   Violation: Debug + Clone + PartialEq + Display
use facet_core::invariants::{check_ack_after_durable, AckEvent, Violation};
use facet_core::Lsn;

fn assert_violation_traits<T: std::fmt::Debug + Clone + PartialEq + std::fmt::Display>() {}

fn assert_reported(v: &Violation) {
    assert_eq!(&v.clone(), v);
    assert!(!v.to_string().trim().is_empty(), "Display of {v:?} is empty");
}

#[test]
fn ac_1_violation_has_required_traits() {
    assert_violation_traits::<Violation>();
}

#[test]
fn ac_1_acks_after_durability_pass() {
    use AckEvent::*;
    assert_eq!(
        check_ack_after_durable(&[Durable(Lsn(5)), Acked(Lsn(3)), Acked(Lsn(5))]),
        Ok(())
    );
}

#[test]
fn ac_1_ack_past_durable_is_rejected() {
    use AckEvent::*;
    let err = check_ack_after_durable(&[Durable(Lsn(4)), Acked(Lsn(5))]).unwrap_err();
    assert_eq!(
        err,
        Violation::AckBeforeDurable {
            index: 1,
            lsn: Lsn(5),
            durable: Lsn(4),
        }
    );
    assert_reported(&err);
}

#[test]
fn ac_1_ack_with_nothing_durable_is_rejected() {
    use AckEvent::*;
    let err = check_ack_after_durable(&[Acked(Lsn(1))]).unwrap_err();
    assert_eq!(
        err,
        Violation::AckBeforeDurable {
            index: 0,
            lsn: Lsn(1),
            durable: Lsn(0),
        }
    );
    assert_reported(&err);
}
