//! Change batches: everything one log record applies, under one LSN.

use crate::lsn::Lsn;
use crate::row::Row;
use crate::value::Value;

/// The changes written at one LSN.
#[derive(Clone, Debug, PartialEq)]
pub struct ChangeBatch {
    pub lsn: Lsn,
    /// New rows.
    pub inserts: Vec<Row>,
    /// Full new versions of existing rows, matched by their primary key.
    pub updates: Vec<Row>,
    /// Primary keys of deleted rows.
    pub deletes: Vec<Value>,
}

impl ChangeBatch {
    /// An empty batch at `lsn`.
    pub fn new(lsn: Lsn) -> Self {
        ChangeBatch {
            lsn,
            inserts: Vec::new(),
            updates: Vec::new(),
            deletes: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ColId, Column, Schema};
    use crate::value::DataType;

    #[test]
    fn keeps_all_changes_under_one_lsn_with_all_six_kinds() {
        let schema = Schema::new(
            vec![
                Column::new("id", DataType::Int, false),
                Column::new("name", DataType::Str, false),
                Column::new("price", DataType::Float, true),
                Column::new("blob", DataType::Bytes, false),
                Column::new("emb", DataType::Vector { dim: 128 }, false),
            ],
            ColId(0),
        )
        .unwrap();
        let r1 = Row::new(vec![
            Value::Int(1),
            Value::Str("a".into()),
            Value::Float(9.5),
            Value::Bytes(vec![1, 2]),
            Value::Vector(vec![0.1; 128]),
        ]);
        let r2 = Row::new(vec![
            Value::Int(3),
            Value::Str("b".into()),
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
        assert_eq!(
            (
                batch.inserts.len(),
                batch.updates.len(),
                batch.deletes.len()
            ),
            (1, 1, 1)
        );
        assert!(schema.validate(&r1).is_ok());
        assert!(schema.validate(&r2).is_ok());
        assert_eq!(batch.updates[0].key(&schema), Some(&Value::Int(3)));
    }

    #[test]
    fn new_is_empty() {
        let b = ChangeBatch::new(Lsn(1));
        assert_eq!(b.lsn, Lsn(1));
        assert!(b.inserts.is_empty() && b.updates.is_empty() && b.deletes.is_empty());
    }
}
