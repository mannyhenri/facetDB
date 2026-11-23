// AC-3 fixture: compiled as an integration test of a scratch crate that depends on facet-core.
//
// API this contract fixes:
//   facet_core::invariants::{Violation, check_same_rows}
//   check_same_rows(&Schema, a: &[Row], b: &[Row]) -> Result<(), Violation>
//   Violation::RowsDiffer { only_in_a: Vec<Value>, only_in_b: Vec<Value>, different: Vec<Value> }
//     (primary-key Values; Float compares by f64::total_cmp equality, so NaN matches NaN)
//   Violation: Debug + Clone + PartialEq + Display
use facet_core::invariants::{check_same_rows, Violation};
use facet_core::{ColId, Column, DataType, Row, Schema, Value};

fn assert_violation_traits<T: std::fmt::Debug + Clone + PartialEq + std::fmt::Display>() {}

fn schema() -> Schema {
    Schema::new(
        vec![
            Column::new("id", DataType::Int, false),
            Column::new("price", DataType::Float, true),
        ],
        ColId(0),
    )
    .expect("valid schema")
}

fn row(id: i64, price: f64) -> Row {
    Row::new(vec![Value::Int(id), Value::Float(price)])
}

#[test]
fn ac_3_violation_has_required_traits() {
    assert_violation_traits::<Violation>();
}

#[test]
fn ac_3_same_rows_in_any_order_pass_and_nan_matches_nan() {
    let schema = schema();
    let a = vec![row(1, 9.5), row(2, f64::NAN)];
    let b = vec![row(2, f64::NAN), row(1, 9.5)];
    assert_eq!(check_same_rows(&schema, &a, &b), Ok(()));
}

#[test]
fn ac_3_every_differing_key_is_reported() {
    let schema = schema();
    let a = vec![row(1, 9.5), row(2, 1.0)];
    let b = vec![row(1, 9.6), row(3, 1.0)];
    let err = check_same_rows(&schema, &a, &b).unwrap_err();
    assert_eq!(
        err,
        Violation::RowsDiffer {
            only_in_a: vec![Value::Int(2)],
            only_in_b: vec![Value::Int(3)],
            different: vec![Value::Int(1)],
        }
    );
    assert_eq!(err.clone(), err);
    assert!(!err.to_string().trim().is_empty(), "Display of {err:?} is empty");
}

#[test]
fn ac_3_identical_inputs_pass() {
    let schema = schema();
    let a = vec![row(1, 9.5), row(2, 1.0)];
    assert_eq!(check_same_rows(&schema, &a, &a.clone()), Ok(()));
}
