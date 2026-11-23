// AC-2 fixture: compiled as an integration test of a scratch crate that depends on facet-core.
//
// API this contract fixes (beyond the names given in the scope):
//   Column::new(name: &str, DataType, nullable: bool), Schema::new(Vec<Column>, ColId), ColId(pub u32)
//   Row::new(Vec<Value>); Row: Clone + PartialEq + Debug
//   ColumnBatch::from_rows(&Schema, &[Row]) -> ColumnBatch or Result<ColumnBatch, _>
//   ColumnBatch::to_rows(&self) -> Vec<Row> or Result<Vec<Row>, _>   (selected rows only)
//   ColumnBatch::columns(&self) -> &[ColumnData] (or &Vec<ColumnData>), one per schema column
//   ColumnData::{Int(Vec<i64>), Float(Vec<f64>), Str(Vec<String>), Bytes(Vec<Vec<u8>>),
//                Vector { dim: usize, values: Vec<f32> }}
//   ColumnBatch::selection(&self) / selection_mut(&mut self) -> bitmap with
//     len(), count_ones(), get(usize) -> bool, set(usize, bool)
use facet_core::{ColId, Column, ColumnBatch, ColumnData, DataType, Row, Schema, Value};

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

trait IntoBatch {
    fn into_batch(self) -> ColumnBatch;
}
impl IntoBatch for ColumnBatch {
    fn into_batch(self) -> ColumnBatch {
        self
    }
}
impl<E: std::fmt::Debug> IntoBatch for Result<ColumnBatch, E> {
    fn into_batch(self) -> ColumnBatch {
        self.expect("ColumnBatch::from_rows rejected valid rows")
    }
}

trait IntoRows {
    fn into_rows(self) -> Vec<Row>;
}
impl IntoRows for Vec<Row> {
    fn into_rows(self) -> Vec<Row> {
        self
    }
}
impl<E: std::fmt::Debug> IntoRows for Result<Vec<Row>, E> {
    fn into_rows(self) -> Vec<Row> {
        self.expect("ColumnBatch::to_rows failed")
    }
}

const N: usize = 1024;
const DIM: usize = 128;

fn schema() -> Schema {
    Schema::new(
        vec![
            Column::new("id", DataType::Int, false),
            Column::new("name", DataType::Str, false),
            Column::new("price", DataType::Float, true),
            Column::new("blob", DataType::Bytes, false),
            Column::new("emb", DataType::Vector { dim: DIM }, false),
        ],
        ColId(0),
    )
    .into_schema()
}

fn rows() -> Vec<Row> {
    (0..N)
        .map(|i| {
            let price = if i % 5 == 0 {
                Value::Null
            } else {
                Value::Float(i as f64 * 0.5)
            };
            let emb = (0..DIM).map(|j| (i * DIM + j) as f32 / 7.0).collect();
            Row::new(vec![
                Value::Int(i as i64),
                Value::Str(format!("name-{i}")),
                price,
                Value::Bytes(vec![i as u8, (i >> 8) as u8, 0xAB]),
                Value::Vector(emb),
            ])
        })
        .collect()
}

#[test]
fn ac_2_column_batch_holds_typed_arrays_and_selection_and_round_trips() {
    let schema = schema();
    let input = rows();
    let mut batch = ColumnBatch::from_rows(&schema, &input).into_batch();

    let columns = batch.columns();
    assert_eq!(columns.len(), 5, "one typed array per schema column");

    match &columns[0] {
        ColumnData::Int(v) => {
            assert_eq!(v.len(), N);
            assert_eq!(*v, (0..N as i64).collect::<Vec<i64>>());
        }
        other => panic!("id column must be ColumnData::Int, got {other:?}"),
    }
    match &columns[1] {
        ColumnData::Str(v) => {
            assert_eq!(v.len(), N);
            assert_eq!(v[1023], "name-1023");
        }
        other => panic!("name column must be ColumnData::Str, got {other:?}"),
    }
    match &columns[2] {
        ColumnData::Float(v) => {
            assert_eq!(v.len(), N);
            assert_eq!(v[1], 0.5);
        }
        other => panic!("price column must be ColumnData::Float, got {other:?}"),
    }
    match &columns[3] {
        ColumnData::Bytes(v) => {
            assert_eq!(v.len(), N);
            assert_eq!(v[1023], vec![0xFF, 0x03, 0xAB]);
        }
        other => panic!("blob column must be ColumnData::Bytes, got {other:?}"),
    }
    match &columns[4] {
        ColumnData::Vector { dim, values } => {
            assert_eq!(*dim, DIM);
            assert_eq!(values.len(), 131_072, "emb is stored flat: 1,024 x 128 f32 values");
            assert_eq!(values[DIM * 3 + 5], (3 * DIM + 5) as f32 / 7.0);
        }
        other => panic!("emb column must be ColumnData::Vector, got {other:?}"),
    }

    assert_eq!(batch.selection().len(), 1024);
    assert_eq!(batch.selection().count_ones(), 1024);

    let back = batch.to_rows().into_rows();
    assert_eq!(back.len(), N);
    assert!(back == input, "to_rows() must equal the input rows (incl. Null prices)");

    batch.selection_mut().set(3, false);
    assert!(!batch.selection().get(3));
    assert_eq!(batch.selection().len(), 1024);
    assert_eq!(batch.selection().count_ones(), 1023);

    let selected = batch.to_rows().into_rows();
    let mut expected = input.clone();
    expected.remove(3);
    assert_eq!(selected.len(), 1023);
    assert!(selected == expected, "to_rows() must return every row except row 3, in order");
    assert!(!selected.contains(&input[3]));
}
