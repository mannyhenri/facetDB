//! Cell values and their data types.

/// One cell. `Null` is allowed only in nullable columns.
///
/// `PartialEq` only: `Float` holds an `f64`, so `Eq` and `Hash` would be wrong.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bytes(Vec<u8>),
    Vector(Vec<f32>),
    Null,
}

/// The type of a schema column, matching the non-null `Value` variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DataType {
    Int,
    Float,
    Str,
    Bytes,
    Vector { dim: usize },
}

impl Value {
    /// The data type of this value; `None` for `Null`. A vector's `dim` is its length.
    pub fn data_type(&self) -> Option<DataType> {
        match self {
            Value::Int(_) => Some(DataType::Int),
            Value::Float(_) => Some(DataType::Float),
            Value::Str(_) => Some(DataType::Str),
            Value::Bytes(_) => Some(DataType::Bytes),
            Value::Vector(v) => Some(DataType::Vector { dim: v.len() }),
            Value::Null => None,
        }
    }

    /// Whether this value may be stored in a column of type `data_type`.
    /// `Null` fits only when the column is `nullable`; a vector must have exactly `dim` elements.
    pub fn fits(&self, data_type: DataType, nullable: bool) -> bool {
        match self.data_type() {
            None => nullable,
            Some(t) => t == data_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_type_of_each_kind() {
        assert_eq!(Value::Int(1).data_type(), Some(DataType::Int));
        assert_eq!(Value::Float(1.0).data_type(), Some(DataType::Float));
        assert_eq!(Value::Str("a".into()).data_type(), Some(DataType::Str));
        assert_eq!(Value::Bytes(vec![1]).data_type(), Some(DataType::Bytes));
        assert_eq!(
            Value::Vector(vec![0.0; 3]).data_type(),
            Some(DataType::Vector { dim: 3 })
        );
        assert_eq!(Value::Null.data_type(), None);
    }

    #[test]
    fn fits_checks_type_dim_and_nullability() {
        assert!(Value::Int(1).fits(DataType::Int, false));
        assert!(!Value::Int(1).fits(DataType::Float, true));
        assert!(Value::Vector(vec![0.0; 4]).fits(DataType::Vector { dim: 4 }, false));
        assert!(!Value::Vector(vec![0.0; 3]).fits(DataType::Vector { dim: 4 }, false));
        assert!(Value::Null.fits(DataType::Str, true));
        assert!(!Value::Null.fits(DataType::Str, false));
    }

    /// One non-null sample per data type, index-aligned with `ALL_TYPES`.
    fn samples() -> Vec<Value> {
        vec![
            Value::Int(1),
            Value::Float(1.0),
            Value::Str("a".into()),
            Value::Bytes(vec![1]),
            Value::Vector(vec![0.0; 4]),
        ]
    }

    const ALL_TYPES: [DataType; 5] = [
        DataType::Int,
        DataType::Float,
        DataType::Str,
        DataType::Bytes,
        DataType::Vector { dim: 4 },
    ];

    // Fails to compile when a DataType variant is added, so ALL_TYPES must be updated.
    #[allow(dead_code)]
    fn all_types_is_exhaustive(t: DataType) {
        match t {
            DataType::Int | DataType::Float | DataType::Str | DataType::Bytes => {}
            DataType::Vector { .. } => {}
        }
    }

    #[test]
    fn fits_exactly_the_matching_type_for_every_kind() {
        for (i, value) in samples().iter().enumerate() {
            for (j, &t) in ALL_TYPES.iter().enumerate() {
                for nullable in [false, true] {
                    assert_eq!(
                        value.fits(t, nullable),
                        i == j,
                        "{value:?} in {t:?} (nullable: {nullable})"
                    );
                }
            }
        }
        for t in ALL_TYPES {
            assert!(Value::Null.fits(t, true), "Null in nullable {t:?}");
            assert!(!Value::Null.fits(t, false), "Null in non-nullable {t:?}");
        }
    }
}
