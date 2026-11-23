// AC-3 fixture: every shared type is re-exported from the facet-core crate root.
#[allow(unused_imports)]
use facet_core::{
    ChangeBatch, ColId, Column, ColumnBatch, ColumnData, DataType, Lsn, Row, Schema, Value,
    BATCH_ROWS,
};

#[test]
fn ac_3_core_exports_shared_types() {
    assert_eq!(BATCH_ROWS, 1024);
    assert_eq!(Lsn::ZERO, Lsn(0));
    assert_eq!(Lsn::ZERO.next(), Lsn(1));
}
