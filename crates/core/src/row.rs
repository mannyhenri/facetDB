//! Rows: one value per schema column, in schema order.

use crate::schema::Schema;
use crate::value::Value;

/// Values in schema column order.
#[derive(Clone, Debug, PartialEq)]
pub struct Row(pub Vec<Value>);

impl Row {
    pub fn new(values: Vec<Value>) -> Self {
        Row(values)
    }

    pub fn values(&self) -> &[Value] {
        &self.0
    }

    pub fn into_values(self) -> Vec<Value> {
        self.0
    }

    /// The primary-key value, or `None` if the row is shorter than the schema
    /// (rows that passed `Schema::validate` never are).
    pub fn key(&self, schema: &Schema) -> Option<&Value> {
        self.0.get(schema.primary_key().index())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ColId, Column};
    use crate::value::DataType;

    #[test]
    fn key_is_the_primary_key_column() {
        let schema = Schema::new(
            vec![
                Column::new("name", DataType::Str, false),
                Column::new("id", DataType::Bytes, false),
            ],
            ColId(1),
        )
        .unwrap();
        let row = Row::new(vec![Value::Str("a".into()), Value::Bytes(vec![7])]);
        assert_eq!(row.key(&schema), Some(&Value::Bytes(vec![7])));
        assert_eq!(Row::new(vec![Value::Str("a".into())]).key(&schema), None);
    }
}
