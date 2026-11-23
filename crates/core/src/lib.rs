//! Shared core types for Facet: LSN, ChangeBatch, values, schema, rows and column batches.
//! (Fragment will join them with the layout contract.)

mod bitmap;
mod change;
mod column;
mod lsn;
mod row;
mod schema;
mod value;

pub use bitmap::Bitmap;
pub use change::ChangeBatch;
pub use column::{BatchError, ColumnBatch, ColumnData, SelectionMut};
pub use lsn::Lsn;
pub use row::Row;
pub use schema::{ColId, Column, RowError, Schema, SchemaError};
pub use value::{DataType, Value};

/// Target number of rows per batch passed across the layout contract.
pub const BATCH_ROWS: usize = 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_rows_is_1024() {
        assert_eq!(BATCH_ROWS, 1024);
    }
}
