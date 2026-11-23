//! Column batches: the same rows stored as one typed array per column.

use std::fmt;

use crate::bitmap::Bitmap;
use crate::row::Row;
use crate::schema::{RowError, Schema};
use crate::value::{DataType, Value};

/// One column's values for every row of a batch. A null slot holds a placeholder
/// (0, empty, or `dim` zeros) and is marked invalid in the batch's validity bitmap.
#[derive(Clone, Debug, PartialEq)]
pub enum ColumnData {
    Int(Vec<i64>),
    Float(Vec<f64>),
    Str(Vec<String>),
    Bytes(Vec<Vec<u8>>),
    /// Row `i` is `values[i * dim..(i + 1) * dim]`.
    Vector {
        dim: usize,
        values: Vec<f32>,
    },
}

impl ColumnData {
    fn with_capacity(data_type: DataType, rows: usize) -> Self {
        match data_type {
            DataType::Int => ColumnData::Int(Vec::with_capacity(rows)),
            DataType::Float => ColumnData::Float(Vec::with_capacity(rows)),
            DataType::Str => ColumnData::Str(Vec::with_capacity(rows)),
            DataType::Bytes => ColumnData::Bytes(Vec::with_capacity(rows)),
            DataType::Vector { dim } => ColumnData::Vector {
                dim,
                values: Vec::with_capacity(rows * dim),
            },
        }
    }

    /// Appends `value`, which must already fit this column; `Null` appends a placeholder.
    fn push(&mut self, value: &Value) {
        match (self, value) {
            (column, Value::Null) => column.push_null(),
            (ColumnData::Int(a), Value::Int(x)) => a.push(*x),
            (ColumnData::Float(a), Value::Float(x)) => a.push(*x),
            (ColumnData::Str(a), Value::Str(x)) => a.push(x.clone()),
            (ColumnData::Bytes(a), Value::Bytes(x)) => a.push(x.clone()),
            (ColumnData::Vector { values, .. }, Value::Vector(x)) => values.extend_from_slice(x),
            (column, value) => unreachable!("{value:?} pushed into {column:?} after validation"),
        }
    }

    fn push_null(&mut self) {
        match self {
            ColumnData::Int(a) => a.push(0),
            ColumnData::Float(a) => a.push(0.0),
            ColumnData::Str(a) => a.push(String::new()),
            ColumnData::Bytes(a) => a.push(Vec::new()),
            ColumnData::Vector { dim, values } => values.resize(values.len() + *dim, 0.0),
        }
    }

    fn get(&self, i: usize) -> Value {
        match self {
            ColumnData::Int(a) => Value::Int(a[i]),
            ColumnData::Float(a) => Value::Float(a[i]),
            ColumnData::Str(a) => Value::Str(a[i].clone()),
            ColumnData::Bytes(a) => Value::Bytes(a[i].clone()),
            ColumnData::Vector { dim, values } => {
                Value::Vector(values[i * dim..(i + 1) * dim].to_vec())
            }
        }
    }
}

/// Rows in columnar form: one typed array and one validity bitmap per schema column,
/// plus a selection bitmap with one bit per row. There is no row cap; later crates aim
/// for `BATCH_ROWS` rows per batch.
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnBatch {
    columns: Vec<ColumnData>,
    validity: Vec<Bitmap>,
    selection: Bitmap,
}

/// A row given to `ColumnBatch::from_rows` did not match the schema.
#[derive(Clone, Debug, PartialEq)]
pub struct BatchError {
    /// Index of the first bad row.
    pub row: usize,
    pub error: RowError,
}

impl fmt::Display for BatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "row {}: {}", self.row, self.error)
    }
}

impl std::error::Error for BatchError {}

impl ColumnBatch {
    /// Builds a batch from rows, all selected. Every row is validated against `schema`.
    pub fn from_rows(schema: &Schema, rows: &[Row]) -> Result<ColumnBatch, BatchError> {
        for (i, row) in rows.iter().enumerate() {
            schema
                .validate(row)
                .map_err(|error| BatchError { row: i, error })?;
        }
        let mut columns = Vec::with_capacity(schema.columns().len());
        let mut validity = Vec::with_capacity(schema.columns().len());
        for (c, column) in schema.columns().iter().enumerate() {
            let mut data = ColumnData::with_capacity(column.data_type, rows.len());
            let mut valid = Bitmap::all_set(rows.len());
            for (r, row) in rows.iter().enumerate() {
                let value = &row.values()[c];
                if matches!(value, Value::Null) {
                    valid.set(r, false);
                }
                data.push(value);
            }
            columns.push(data);
            validity.push(valid);
        }
        Ok(ColumnBatch {
            columns,
            validity,
            selection: Bitmap::all_set(rows.len()),
        })
    }

    /// The selected rows, in order.
    pub fn to_rows(&self) -> Vec<Row> {
        self.selection
            .iter_ones()
            .map(|r| Row::new((0..self.columns.len()).map(|c| self.value(c, r)).collect()))
            .collect()
    }

    /// Number of rows, selected or not.
    pub fn len(&self) -> usize {
        self.selection.len()
    }

    pub fn is_empty(&self) -> bool {
        self.selection.is_empty()
    }

    /// One typed array per schema column, in schema order.
    pub fn columns(&self) -> &[ColumnData] {
        &self.columns
    }

    /// One bitmap per column; a cleared bit marks a null.
    pub fn validity(&self) -> &[Bitmap] {
        &self.validity
    }

    pub fn selection(&self) -> &Bitmap {
        &self.selection
    }

    pub fn selection_mut(&mut self) -> &mut Bitmap {
        &mut self.selection
    }

    fn value(&self, column: usize, row: usize) -> Value {
        if self.validity[column].get(row) {
            self.columns[column].get(row)
        } else {
            Value::Null
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ColId, Column};
    use proptest::prelude::*;

    const DIM: usize = 128;

    fn schema(dim: usize) -> Schema {
        Schema::new(
            vec![
                Column::new("id", DataType::Int, false),
                Column::new("name", DataType::Str, false),
                Column::new("price", DataType::Float, true),
                Column::new("blob", DataType::Bytes, false),
                Column::new("emb", DataType::Vector { dim }, false),
            ],
            ColId(0),
        )
        .unwrap()
    }

    fn rows(n: usize) -> Vec<Row> {
        (0..n)
            .map(|i| {
                let price = if i % 5 == 0 {
                    Value::Null
                } else {
                    Value::Float(i as f64 * 0.5)
                };
                Row::new(vec![
                    Value::Int(i as i64),
                    Value::Str(format!("name-{i}")),
                    price,
                    Value::Bytes(vec![i as u8]),
                    Value::Vector((0..DIM).map(|j| (i * DIM + j) as f32).collect()),
                ])
            })
            .collect()
    }

    #[test]
    fn typed_columns_validity_and_selection_for_a_full_batch() {
        let input = rows(crate::BATCH_ROWS);
        let batch = ColumnBatch::from_rows(&schema(DIM), &input).unwrap();
        assert_eq!(batch.len(), 1024);
        assert_eq!(batch.columns().len(), 5);
        assert!(matches!(&batch.columns()[0], ColumnData::Int(v) if v.len() == 1024));
        assert!(matches!(&batch.columns()[1], ColumnData::Str(v) if v.len() == 1024));
        assert!(matches!(&batch.columns()[2], ColumnData::Float(v) if v.len() == 1024));
        assert!(matches!(&batch.columns()[3], ColumnData::Bytes(v) if v.len() == 1024));
        assert!(matches!(
            &batch.columns()[4],
            ColumnData::Vector { dim: DIM, values } if values.len() == 131_072
        ));
        assert_eq!(batch.validity()[2].count_ones(), 1024 - 205);
        assert_eq!(batch.validity()[0].count_ones(), 1024);
        assert_eq!(batch.selection().len(), 1024);
        assert_eq!(batch.selection().count_ones(), 1024);
        assert_eq!(batch.to_rows(), input);
    }

    #[test]
    fn to_rows_returns_selected_rows_only() {
        let input = rows(10);
        let mut batch = ColumnBatch::from_rows(&schema(DIM), &input).unwrap();
        batch.selection_mut().set(3, false);
        let mut expected = input;
        expected.remove(3);
        assert_eq!(batch.to_rows(), expected);
    }

    #[test]
    fn no_row_cap_above_batch_rows() {
        let batch = ColumnBatch::from_rows(&schema(DIM), &rows(crate::BATCH_ROWS + 1)).unwrap();
        assert_eq!(batch.len(), 1025);
    }

    #[test]
    fn empty_batch() {
        let batch = ColumnBatch::from_rows(&schema(DIM), &[]).unwrap();
        assert!(batch.is_empty());
        assert_eq!(batch.to_rows(), Vec::<Row>::new());
    }

    #[test]
    fn from_rows_rejects_an_invalid_row() {
        let mut input = rows(3);
        input[2].0[0] = Value::Str("x".into());
        let err = ColumnBatch::from_rows(&schema(DIM), &input).unwrap_err();
        assert_eq!(err.row, 2);
        assert!(matches!(
            err.error,
            RowError::Mismatch {
                column: ColId(0),
                ..
            }
        ));
    }

    const PDIM: usize = 3;

    fn nullable_schema() -> Schema {
        Schema::new(
            vec![
                Column::new("id", DataType::Int, false),
                Column::new("name", DataType::Str, true),
                Column::new("price", DataType::Float, true),
                Column::new("blob", DataType::Bytes, true),
                Column::new("emb", DataType::Vector { dim: PDIM }, true),
            ],
            ColId(0),
        )
        .unwrap()
    }

    fn row_strategy() -> impl Strategy<Value = Row> {
        (
            any::<i64>(),
            proptest::option::of(".{0,8}"),
            proptest::option::of(-1e12f64..1e12),
            proptest::option::of(proptest::collection::vec(any::<u8>(), 0..8)),
            proptest::option::of(proptest::collection::vec(-1e6f32..1e6, PDIM)),
        )
            .prop_map(|(id, name, price, blob, emb)| {
                Row::new(vec![
                    Value::Int(id),
                    name.map_or(Value::Null, Value::Str),
                    price.map_or(Value::Null, Value::Float),
                    blob.map_or(Value::Null, Value::Bytes),
                    emb.map_or(Value::Null, Value::Vector),
                ])
            })
    }

    proptest! {
        #[test]
        fn from_rows_to_rows_round_trips(
            rows in proptest::collection::vec((row_strategy(), any::<bool>()), 0..200)
        ) {
            let input: Vec<Row> = rows.iter().map(|(r, _)| r.clone()).collect();
            let mut batch = ColumnBatch::from_rows(&nullable_schema(), &input).unwrap();
            prop_assert_eq!(batch.to_rows(), input.clone());

            for (i, (_, selected)) in rows.iter().enumerate() {
                batch.selection_mut().set(i, *selected);
            }
            let expected: Vec<Row> = rows
                .iter()
                .filter(|(_, selected)| *selected)
                .map(|(r, _)| r.clone())
                .collect();
            prop_assert_eq!(batch.to_rows(), expected);
        }
    }
}
