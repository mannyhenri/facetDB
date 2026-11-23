// AC-1 fixture: compiled as an integration test of a scratch crate that depends on facet-core.
//
// API this contract fixes (beyond the names given in the scope):
//   Column::new(name: &str, DataType, nullable: bool)
//   Schema::new(Vec<Column>, ColId)  -> Schema or Result<Schema, _>
//   ColId(pub u32), Row::new(Vec<Value>), schema.validate(&Row) -> Result<(), _>
//   ChangeBatch { lsn, inserts, updates, deletes } with public fields.
use facet_core::{ChangeBatch, ColId, Column, DataType, Lsn, Row, Schema, Value};

// Accepts Schema::new returning either Schema or Result<Schema, E>.
trait IntoSchema {
    fn into_schema(self) -> Schema;
}
impl IntoSchema for Schema {
    fn into_schema(self) -> Schema {
        self
    }
}
impl<E: std::fmt::Debug> IntoSchema for Result<Schema, E> {
    fn into_schema(self) -> Schema {
        self.expect("Schema::new rejected a valid schema")
    }
}

fn schema() -> Schema {
    Schema::new(
        vec![
            Column::new("id", DataType::Int, false),
            Column::new("name", DataType::Str, false),
            Column::new("price", DataType::Float, true),
            Column::new("blob", DataType::Bytes, false),
            Column::new("emb", DataType::Vector { dim: 128 }, false),
        ],
        ColId(0),
    )
    .into_schema()
}

#[test]
fn ac_1_change_batch_keeps_all_changes_under_one_lsn_and_accepts_six_value_kinds() {
    let schema = schema();

    let r1 = Row::new(vec![
        Value::Int(1),
        Value::Str("a".to_string()),
        Value::Float(9.5),
        Value::Bytes(vec![1, 2]),
        Value::Vector(vec![0.1; 128]),
    ]);
    let r2 = Row::new(vec![
        Value::Int(3),
        Value::Str("b".to_string()),
        Value::Null,
        Value::Bytes(vec![3]),
        Value::Vector(vec![0.2; 128]),
    ]);

    let batch = ChangeBatch {
        lsn: Lsn(7),
        inserts: vec![r1.clone()],
        updates: vec![r2.clone()],
        deletes: vec![Value::Int(2)],
    };

    assert_eq!(batch.lsn, Lsn(7));
    assert_eq!(batch.inserts.len(), 1);
    assert_eq!(batch.updates.len(), 1);
    assert_eq!(batch.deletes.len(), 1);
    assert_eq!(batch.inserts[0], r1);
    assert_eq!(batch.updates[0], r2);
    assert_eq!(batch.deletes, vec![Value::Int(2)]);

    assert!(schema.validate(&r1).is_ok(), "r1 (all five typed kinds) must validate");
    assert!(schema.validate(&r2).is_ok(), "r2 (Null in nullable price) must validate");

    let bad = Row::new(vec![
        Value::Str("1".to_string()),
        Value::Str("a".to_string()),
        Value::Float(9.5),
        Value::Bytes(vec![1, 2]),
        Value::Vector(vec![0.1; 128]),
    ]);
    assert!(schema.validate(&bad).is_err(), "Str in the Int id column must fail validation");
}
