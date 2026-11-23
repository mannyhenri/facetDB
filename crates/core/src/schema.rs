//! Table schemas and row validation.

use std::fmt;

use crate::row::Row;
use crate::value::DataType;

/// A column, identified by its position in the schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColId(pub u32);

impl ColId {
    /// The column's position as an index into a row or schema.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// One schema column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

impl Column {
    pub fn new(name: impl Into<String>, data_type: DataType, nullable: bool) -> Self {
        Column {
            name: name.into(),
            data_type,
            nullable,
        }
    }
}

/// Ordered columns plus the single primary-key column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schema {
    columns: Vec<Column>,
    primary_key: ColId,
}

/// Why a schema could not be built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemaError {
    /// The primary key is not a column position.
    PrimaryKeyOutOfRange(ColId),
    /// The primary key is nullable or not Int, Str or Bytes.
    InvalidPrimaryKey(ColId),
    /// Two columns share a name.
    DuplicateName(String),
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaError::PrimaryKeyOutOfRange(c) => {
                write!(f, "primary key column {} does not exist", c.0)
            }
            SchemaError::InvalidPrimaryKey(c) => write!(
                f,
                "primary key column {} must be a non-nullable Int, Str or Bytes column",
                c.0
            ),
            SchemaError::DuplicateName(n) => write!(f, "duplicate column name {n:?}"),
        }
    }
}

impl std::error::Error for SchemaError {}

/// The first way a row does not match a schema.
#[derive(Clone, Debug, PartialEq)]
pub enum RowError {
    /// The row has a different number of values than the schema has columns.
    Arity { expected: usize, actual: usize },
    /// The value in `column` does not fit its type (or is `Null` in a non-nullable column).
    Mismatch {
        column: ColId,
        expected: DataType,
        nullable: bool,
        actual: Option<DataType>,
    },
}

impl fmt::Display for RowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RowError::Arity { expected, actual } => {
                write!(f, "row has {actual} values, schema has {expected} columns")
            }
            RowError::Mismatch {
                column,
                expected,
                nullable,
                actual,
            } => {
                let null = if *nullable { " (nullable)" } else { "" };
                let got = actual.map_or("Null".to_string(), |t| format!("{t:?}"));
                write!(
                    f,
                    "column {}: expected {expected:?}{null}, got {got}",
                    column.0
                )
            }
        }
    }
}

impl std::error::Error for RowError {}

impl Schema {
    /// Builds a schema. The primary key must be a non-nullable Int, Str or Bytes column,
    /// and column names must be unique.
    pub fn new(columns: Vec<Column>, primary_key: ColId) -> Result<Schema, SchemaError> {
        let pk = columns
            .get(primary_key.index())
            .ok_or(SchemaError::PrimaryKeyOutOfRange(primary_key))?;
        let key_type_ok = matches!(
            pk.data_type,
            DataType::Int | DataType::Str | DataType::Bytes
        );
        if pk.nullable || !key_type_ok {
            return Err(SchemaError::InvalidPrimaryKey(primary_key));
        }
        for (i, c) in columns.iter().enumerate() {
            if columns[..i].iter().any(|d| d.name == c.name) {
                return Err(SchemaError::DuplicateName(c.name.clone()));
            }
        }
        Ok(Schema {
            columns,
            primary_key,
        })
    }

    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    pub fn column(&self, id: ColId) -> Option<&Column> {
        self.columns.get(id.index())
    }

    pub fn primary_key(&self) -> ColId {
        self.primary_key
    }

    /// The position of the column called `name`.
    pub fn col_id(&self, name: &str) -> Option<ColId> {
        self.columns
            .iter()
            .position(|c| c.name == name)
            .map(|i| ColId(i as u32))
    }

    /// Checks that `row` has one fitting value per column, reporting the first mismatch.
    pub fn validate(&self, row: &Row) -> Result<(), RowError> {
        let values = row.values();
        if values.len() != self.columns.len() {
            return Err(RowError::Arity {
                expected: self.columns.len(),
                actual: values.len(),
            });
        }
        for (i, (c, v)) in self.columns.iter().zip(values).enumerate() {
            if !v.fits(c.data_type, c.nullable) {
                return Err(RowError::Mismatch {
                    column: ColId(i as u32),
                    expected: c.data_type,
                    nullable: c.nullable,
                    actual: v.data_type(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

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
        .unwrap()
    }

    fn r1() -> Row {
        Row::new(vec![
            Value::Int(1),
            Value::Str("a".into()),
            Value::Float(9.5),
            Value::Bytes(vec![1, 2]),
            Value::Vector(vec![0.1; 128]),
        ])
    }

    #[test]
    fn lookup_by_name() {
        let s = schema();
        assert_eq!(s.col_id("id"), Some(ColId(0)));
        assert_eq!(s.col_id("emb"), Some(ColId(4)));
        assert_eq!(s.col_id("missing"), None);
        assert_eq!(s.primary_key(), ColId(0));
        assert_eq!(s.column(ColId(2)).unwrap().name, "price");
    }

    #[test]
    fn validate_accepts_every_kind_and_null_in_nullable() {
        let s = schema();
        assert_eq!(s.validate(&r1()), Ok(()));
        let mut v = r1().into_values();
        v[2] = Value::Null;
        assert_eq!(s.validate(&Row::new(v)), Ok(()));
    }

    #[test]
    fn validate_reports_first_mismatch() {
        let s = schema();
        let mut v = r1().into_values();
        v[0] = Value::Str("1".into());
        v[3] = Value::Null;
        assert_eq!(
            s.validate(&Row::new(v)),
            Err(RowError::Mismatch {
                column: ColId(0),
                expected: DataType::Int,
                nullable: false,
                actual: Some(DataType::Str),
            })
        );
    }

    #[test]
    fn validate_rejects_null_in_non_nullable_wrong_dim_and_arity() {
        let s = schema();
        let mut v = r1().into_values();
        v[1] = Value::Null;
        assert!(matches!(
            s.validate(&Row::new(v)),
            Err(RowError::Mismatch {
                column: ColId(1),
                actual: None,
                ..
            })
        ));
        let mut v = r1().into_values();
        v[4] = Value::Vector(vec![0.1; 127]);
        assert!(matches!(
            s.validate(&Row::new(v)),
            Err(RowError::Mismatch {
                column: ColId(4),
                ..
            })
        ));
        let mut v = r1().into_values();
        v.pop();
        assert_eq!(
            s.validate(&Row::new(v)),
            Err(RowError::Arity {
                expected: 5,
                actual: 4
            })
        );
    }

    #[test]
    fn new_rejects_bad_primary_keys_and_duplicate_names() {
        let cols = || {
            vec![
                Column::new("id", DataType::Int, false),
                Column::new("price", DataType::Float, false),
                Column::new("note", DataType::Str, true),
            ]
        };
        assert_eq!(
            Schema::new(cols(), ColId(3)),
            Err(SchemaError::PrimaryKeyOutOfRange(ColId(3)))
        );
        assert_eq!(
            Schema::new(cols(), ColId(1)),
            Err(SchemaError::InvalidPrimaryKey(ColId(1)))
        );
        assert_eq!(
            Schema::new(cols(), ColId(2)),
            Err(SchemaError::InvalidPrimaryKey(ColId(2)))
        );
        let mut dup = cols();
        dup.push(Column::new("id", DataType::Bytes, false));
        assert_eq!(
            Schema::new(dup, ColId(0)),
            Err(SchemaError::DuplicateName("id".into()))
        );
    }
}
