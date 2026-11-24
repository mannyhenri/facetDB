//! Unit tests for the benchmark data generator, written before the code (TDD).
//!
//! API these tests fix:
//!   facet_bench::datagen::{Dataset, rows, write_files, VECTOR_DIM, VECTOR_CATEGORIES}
//!   enum Dataset { Events, Orders, Vectors }  (Clone, Copy, Debug, PartialEq, Eq)
//!     Dataset::ALL, name(), from_name(&str) -> Option<Dataset>, schema() -> Schema,
//!     default_rows() -> u64, file_names() -> &'static [&'static str]
//!   rows(Dataset, seed: u64, ids: Range<u64>) -> impl Iterator<Item = Row>
//!     row i depends only on (dataset, seed, i), never on the range or the total row count
//!   write_files(Dataset, seed: u64, rows: u64, out: &Path, threads: usize) -> io::Result<()>
//!   facet_bench::cli::{parse_args, Args}: parse_args(&[String]) -> Result<Args, String>
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use facet_bench::cli::{parse_args, Args};
use facet_bench::datagen::{rows, write_files, Dataset, VECTOR_CATEGORIES, VECTOR_DIM};
use facet_core::{DataType, Row, Schema, Value};

const SEED: u64 = 42;

fn all_rows(ds: Dataset, seed: u64, n: u64) -> Vec<Row> {
    rows(ds, seed, 0..n).collect()
}

fn col(schema: &Schema, name: &str) -> usize {
    schema
        .col_id(name)
        .unwrap_or_else(|| panic!("no column {name}"))
        .index()
}

fn int(v: &Value) -> i64 {
    match v {
        Value::Int(i) => *i,
        other => panic!("expected Int, got {other:?}"),
    }
}

fn float(v: &Value) -> f64 {
    match v {
        Value::Float(f) => *f,
        other => panic!("expected Float, got {other:?}"),
    }
}

fn string(v: &Value) -> &str {
    match v {
        Value::Str(s) => s,
        other => panic!("expected Str, got {other:?}"),
    }
}

/// A fresh, empty scratch directory under the target dir's temp area.
fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("facet-datagen-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn read_files(ds: Dataset, dir: &Path) -> Vec<Vec<u8>> {
    ds.file_names()
        .iter()
        .map(|f| fs::read(dir.join(f)).unwrap_or_else(|e| panic!("{f}: {e}")))
        .collect()
}

/// Parses one CSV field back into a `Value` of the column's type (empty + nullable = NULL).
fn parse_field(field: &str, data_type: DataType, nullable: bool) -> Value {
    if field.is_empty() && nullable {
        return Value::Null;
    }
    match data_type {
        DataType::Int => Value::Int(
            field
                .parse()
                .unwrap_or_else(|_| panic!("bad Int {field:?}")),
        ),
        DataType::Float => Value::Float(
            field
                .parse()
                .unwrap_or_else(|_| panic!("bad Float {field:?}")),
        ),
        DataType::Str => Value::Str(field.to_string()),
        other => panic!("unexpected CSV column type {other:?}"),
    }
}

// ---------------------------------------------------------------- schemas

#[test]
fn dataset_names_round_trip_and_unknown_is_none() {
    assert_eq!(
        Dataset::ALL,
        [Dataset::Events, Dataset::Orders, Dataset::Vectors]
    );
    for ds in Dataset::ALL {
        assert_eq!(Dataset::from_name(ds.name()), Some(ds));
    }
    assert_eq!(Dataset::Events.name(), "events");
    assert_eq!(Dataset::Orders.name(), "orders");
    assert_eq!(Dataset::Vectors.name(), "vectors");
    assert_eq!(Dataset::from_name("hits"), None);
    assert_eq!(Dataset::from_name(""), None);
    assert_eq!(Dataset::from_name("Events"), None);
}

#[test]
fn default_row_counts_and_file_names() {
    assert_eq!(Dataset::Events.default_rows(), 10_000_000);
    assert_eq!(Dataset::Orders.default_rows(), 1_000_000);
    assert_eq!(Dataset::Vectors.default_rows(), 1_000_000);
    assert_eq!(Dataset::Events.file_names(), ["events.csv"]);
    assert_eq!(Dataset::Orders.file_names(), ["orders.csv"]);
    assert_eq!(
        Dataset::Vectors.file_names(),
        ["vectors.f32", "vectors_meta.csv"]
    );
    assert_eq!(VECTOR_DIM, 128);
    assert_eq!(VECTOR_CATEGORIES, 100);
}

fn shape(schema: &Schema) -> Vec<(String, DataType, bool)> {
    schema
        .columns()
        .iter()
        .map(|c| (c.name.clone(), c.data_type, c.nullable))
        .collect()
}

fn expected(cols: &[(&str, DataType, bool)]) -> Vec<(String, DataType, bool)> {
    cols.iter()
        .map(|(n, t, b)| (n.to_string(), *t, *b))
        .collect()
}

#[test]
fn events_schema_is_the_18_column_mix() {
    use DataType::*;
    let s = Dataset::Events.schema();
    assert_eq!(
        shape(&s),
        expected(&[
            ("event_id", Int, false),
            ("event_time", Int, false),
            ("user_id", Int, false),
            ("session_id", Int, false),
            ("url", Str, false),
            ("referer", Str, false),
            ("title", Str, false),
            ("search_phrase", Str, false),
            ("country", Str, false),
            ("region_id", Int, false),
            ("browser", Str, false),
            ("os", Str, false),
            ("device_type", Str, false),
            ("is_mobile", Int, false),
            ("resolution_width", Int, false),
            ("resolution_height", Int, false),
            ("duration_ms", Int, false),
            ("revenue", Float, true),
        ])
    );
    assert_eq!(s.primary_key().index(), 0);
}

#[test]
fn orders_schema_is_narrow_and_keyed() {
    use DataType::*;
    let s = Dataset::Orders.schema();
    assert_eq!(
        shape(&s),
        expected(&[
            ("order_id", Int, false),
            ("customer_id", Int, false),
            ("product_id", Int, false),
            ("quantity", Int, false),
            ("price", Float, false),
            ("status", Str, false),
        ])
    );
    assert_eq!(s.primary_key().index(), 0);
}

#[test]
fn vectors_schema_has_128_dim_vector_category_and_price() {
    use DataType::*;
    let s = Dataset::Vectors.schema();
    assert_eq!(
        shape(&s),
        expected(&[
            ("id", Int, false),
            ("vector", Vector { dim: 128 }, false),
            ("category", Str, false),
            ("price", Float, false),
        ])
    );
    assert_eq!(s.primary_key().index(), 0);
}

// ---------------------------------------------------------------- rows

#[test]
fn every_row_fits_its_schema() {
    for ds in Dataset::ALL {
        let schema = ds.schema();
        for (i, row) in all_rows(ds, SEED, 3_000).iter().enumerate() {
            if let Err(e) = schema.validate(row) {
                panic!("{} row {i} does not fit its schema: {e:?}", ds.name());
            }
        }
    }
}

#[test]
fn row_counts_are_exact_including_empty_and_single() {
    for ds in Dataset::ALL {
        for n in [0u64, 1, 2, 999, 4_097] {
            assert_eq!(
                rows(ds, SEED, 0..n).count() as u64,
                n,
                "{} n={n}",
                ds.name()
            );
        }
        assert_eq!(
            rows(ds, SEED, 10..10).count(),
            0,
            "{} empty range",
            ds.name()
        );
    }
}

#[test]
fn primary_keys_are_unique_and_equal_the_row_index() {
    for ds in Dataset::ALL {
        let rows = all_rows(ds, SEED, 5_000);
        let keys: Vec<i64> = rows.iter().map(|r| int(&r.0[0])).collect();
        let distinct: HashSet<i64> = keys.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            keys.len(),
            "{} has duplicate keys",
            ds.name()
        );
        assert_eq!(keys, (0..5_000).collect::<Vec<i64>>(), "{} keys", ds.name());
    }
}

#[test]
fn a_sub_range_equals_the_same_slice_of_a_full_run() {
    for ds in Dataset::ALL {
        let full = all_rows(ds, SEED, 5_000);
        let part: Vec<Row> = rows(ds, SEED, 1_234..3_210).collect();
        assert_eq!(part, full[1_234..3_210].to_vec(), "{}", ds.name());
        let last: Vec<Row> = rows(ds, SEED, 4_999..5_000).collect();
        assert_eq!(last, full[4_999..].to_vec(), "{} last row", ds.name());
    }
}

#[test]
fn same_seed_same_rows_different_seed_different_rows() {
    for ds in Dataset::ALL {
        let a = all_rows(ds, SEED, 500);
        let b = all_rows(ds, SEED, 500);
        let c = all_rows(ds, SEED + 1, 500);
        let d = all_rows(ds, u64::MAX, 500);
        assert_eq!(a, b, "{} not deterministic", ds.name());
        assert_ne!(a, c, "{} ignores the seed", ds.name());
        assert_ne!(a, d, "{} ignores the seed (u64::MAX)", ds.name());
        // Keys stay 0..n whatever the seed.
        assert_eq!(int(&c[499].0[0]), 499);
    }
}

#[test]
fn strings_never_need_csv_escaping() {
    for ds in Dataset::ALL {
        for row in all_rows(ds, SEED, 3_000) {
            for v in &row.0 {
                if let Value::Str(s) = v {
                    assert!(
                        !s.contains([',', '"', '\n', '\r']),
                        "{} string needs escaping: {s:?}",
                        ds.name()
                    );
                }
            }
        }
    }
}

#[test]
fn floats_are_finite() {
    for ds in Dataset::ALL {
        for row in all_rows(ds, SEED, 3_000) {
            for v in &row.0 {
                match v {
                    Value::Float(f) => assert!(f.is_finite(), "{} float {f}", ds.name()),
                    Value::Vector(xs) => {
                        assert!(xs.iter().all(|x| x.is_finite()), "{} vector", ds.name())
                    }
                    _ => {}
                }
            }
        }
    }
}

#[test]
fn events_values_are_in_their_domains_and_skewed() {
    let s = Dataset::Events.schema();
    let rows = all_rows(Dataset::Events, SEED, 20_000);
    let (country, mobile, phrase, revenue, browser) = (
        col(&s, "country"),
        col(&s, "is_mobile"),
        col(&s, "search_phrase"),
        col(&s, "revenue"),
        col(&s, "browser"),
    );
    let mut browsers: HashMap<String, usize> = HashMap::new();
    let (mut empty_phrase, mut null_revenue) = (0, 0);
    for r in &rows {
        let c = string(&r.0[country]);
        assert!(
            c.len() == 2 && c.bytes().all(|b| b.is_ascii_uppercase()),
            "country {c:?}"
        );
        assert!(matches!(int(&r.0[mobile]), 0 | 1));
        for name in [
            "duration_ms",
            "resolution_width",
            "resolution_height",
            "event_time",
        ] {
            assert!(int(&r.0[col(&s, name)]) >= 0, "{name} negative");
        }
        if string(&r.0[phrase]).is_empty() {
            empty_phrase += 1;
        }
        match &r.0[revenue] {
            Value::Null => null_revenue += 1,
            v => assert!(float(v) >= 0.0),
        }
        *browsers
            .entry(string(&r.0[browser]).to_string())
            .or_default() += 1;
    }
    // "Empty for most rows" / "NULL for most rows", but not all.
    assert!(empty_phrase > rows.len() / 2 && empty_phrase < rows.len());
    assert!(null_revenue > rows.len() / 2 && null_revenue < rows.len());
    // Skewed: the most common browser takes well over a uniform share.
    assert!(browsers.len() >= 3, "only {} browsers", browsers.len());
    let top = *browsers.values().max().unwrap();
    assert!(
        top * browsers.len() > 2 * rows.len(),
        "browser distribution looks uniform: {browsers:?}"
    );
}

#[test]
fn orders_values_are_in_their_domains() {
    let s = Dataset::Orders.schema();
    let (q, p, st) = (col(&s, "quantity"), col(&s, "price"), col(&s, "status"));
    let mut statuses = HashSet::new();
    for r in all_rows(Dataset::Orders, SEED, 20_000) {
        assert!((1..=10).contains(&int(&r.0[q])), "quantity {:?}", r.0[q]);
        assert!(float(&r.0[p]) > 0.0, "price {:?}", r.0[p]);
        assert!(int(&r.0[col(&s, "customer_id")]) >= 0);
        assert!(int(&r.0[col(&s, "product_id")]) >= 0);
        statuses.insert(string(&r.0[st]).to_string());
    }
    assert!(
        (2..=8).contains(&statuses.len()),
        "status set should be small: {statuses:?}"
    );
}

#[test]
fn vectors_have_100_categories_positive_prices_and_cluster_by_category() {
    let rows = all_rows(Dataset::Vectors, SEED, 20_000);
    let mut by_cat: HashMap<String, Vec<&Vec<f32>>> = HashMap::new();
    for r in &rows {
        let Value::Vector(v) = &r.0[1] else {
            panic!("expected Vector, got {:?}", r.0[1])
        };
        assert_eq!(v.len(), VECTOR_DIM);
        assert!(float(&r.0[3]) > 0.0);
        let cat = string(&r.0[2]);
        assert!(!cat.is_empty());
        by_cat.entry(cat.to_string()).or_default().push(v);
    }
    assert_eq!(
        by_cat.len(),
        VECTOR_CATEGORIES,
        "distinct categories in 20,000 rows"
    );

    // Clustered: a vector is closer to its own category's mean than to another category's mean.
    let mean = |vs: &[&Vec<f32>]| -> Vec<f32> {
        let mut m = vec![0f32; VECTOR_DIM];
        for v in vs {
            for (a, b) in m.iter_mut().zip(v.iter()) {
                *a += b / vs.len() as f32;
            }
        }
        m
    };
    let dist =
        |a: &[f32], b: &[f32]| -> f32 { a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum() };
    let mut cats: Vec<&String> = by_cat.keys().collect();
    cats.sort();
    let means: Vec<Vec<f32>> = cats.iter().map(|c| mean(&by_cat[*c])).collect();
    let (mut closer, mut total) = (0, 0);
    for (ci, c) in cats.iter().enumerate() {
        let other = &means[(ci + 1) % means.len()];
        for v in &by_cat[*c] {
            total += 1;
            if dist(v, &means[ci]) < dist(v, other) {
                closer += 1;
            }
        }
    }
    assert!(
        closer * 10 >= total * 9,
        "only {closer}/{total} vectors nearer their own centroid"
    );
}

// ---------------------------------------------------------------- files

#[test]
fn files_are_byte_identical_for_any_thread_count() {
    // Spans several chunks with a ragged tail, so chunk boundaries and ordering are exercised.
    for (ds, n) in [
        (Dataset::Events, 150_017u64),
        (Dataset::Orders, 150_017),
        (Dataset::Vectors, 40_003),
    ] {
        let one = scratch(&format!("{}-t1", ds.name()));
        let many = scratch(&format!("{}-t7", ds.name()));
        write_files(ds, SEED, n, &one, 1).unwrap();
        write_files(ds, SEED, n, &many, 7).unwrap();
        assert!(
            read_files(ds, &one) == read_files(ds, &many),
            "{} output depends on the thread count",
            ds.name()
        );
        let other = scratch(&format!("{}-seed", ds.name()));
        write_files(ds, SEED + 1, n, &other, 7).unwrap();
        assert!(
            read_files(ds, &one) != read_files(ds, &other),
            "{} files ignore the seed",
            ds.name()
        );
        for d in [one, many, other] {
            fs::remove_dir_all(d).unwrap();
        }
    }
}

#[test]
fn csv_files_hold_exactly_the_generated_rows() {
    for ds in [Dataset::Events, Dataset::Orders] {
        let n = 2_500;
        let dir = scratch(&format!("{}-csv", ds.name()));
        write_files(ds, SEED, n, &dir, 3).unwrap();
        let text = fs::read_to_string(dir.join(ds.file_names()[0])).unwrap();
        assert!(
            text.ends_with('\n'),
            "{} file must end with a newline",
            ds.name()
        );
        let mut lines = text.lines();
        let schema = ds.schema();
        let header: Vec<&str> = lines.next().unwrap().split(',').collect();
        let names: Vec<&str> = schema.columns().iter().map(|c| c.name.as_str()).collect();
        assert_eq!(header, names, "{} header", ds.name());
        let parsed: Vec<Row> = lines
            .map(|l| {
                let fields: Vec<&str> = l.split(',').collect();
                assert_eq!(fields.len(), names.len(), "{} line {l:?}", ds.name());
                Row(fields
                    .iter()
                    .zip(schema.columns())
                    .map(|(f, c)| parse_field(f, c.data_type, c.nullable))
                    .collect())
            })
            .collect();
        assert_eq!(
            parsed,
            all_rows(ds, SEED, n),
            "{} CSV differs from rows()",
            ds.name()
        );
        fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn vector_files_hold_exactly_the_generated_rows() {
    let n = 1_500u64;
    let dir = scratch("vectors-files");
    write_files(Dataset::Vectors, SEED, n, &dir, 4).unwrap();
    let raw = fs::read(dir.join("vectors.f32")).unwrap();
    assert_eq!(raw.len() as u64, n * VECTOR_DIM as u64 * 4);
    let meta = fs::read_to_string(dir.join("vectors_meta.csv")).unwrap();
    let mut lines = meta.lines();
    assert_eq!(lines.next(), Some("id,category,price"));
    let expected = all_rows(Dataset::Vectors, SEED, n);
    let mut count = 0;
    for (i, (line, row)) in lines.zip(&expected).enumerate() {
        let vec: Vec<f32> = raw[i * VECTOR_DIM * 4..(i + 1) * VECTOR_DIM * 4]
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        let f: Vec<&str> = line.split(',').collect();
        let got = Row(vec![
            Value::Int(f[0].parse().unwrap()),
            Value::Vector(vec),
            Value::Str(f[1].to_string()),
            Value::Float(f[2].parse().unwrap()),
        ]);
        assert_eq!(&got, row, "vector row {i}");
        count += 1;
    }
    assert_eq!(count, n);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn zero_rows_writes_header_only_files() {
    let dir = scratch("zero");
    write_files(Dataset::Orders, SEED, 0, &dir, 2).unwrap();
    assert_eq!(
        fs::read_to_string(dir.join("orders.csv")).unwrap(),
        "order_id,customer_id,product_id,quantity,price,status\n"
    );
    write_files(Dataset::Vectors, SEED, 0, &dir, 2).unwrap();
    assert_eq!(fs::read(dir.join("vectors.f32")).unwrap().len(), 0);
    assert_eq!(
        fs::read_to_string(dir.join("vectors_meta.csv")).unwrap(),
        "id,category,price\n"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn write_files_creates_a_missing_output_dir() {
    let base = scratch("mkdir");
    let nested = base.join("a").join("b");
    write_files(Dataset::Orders, SEED, 10, &nested, 1).unwrap();
    assert!(nested.join("orders.csv").is_file());
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn write_files_errors_when_out_is_a_file_or_threads_is_zero() {
    let base = scratch("errors");
    let file = base.join("not-a-dir");
    fs::write(&file, b"x").unwrap();
    assert!(write_files(Dataset::Orders, SEED, 10, &file, 1).is_err());
    let err = write_files(Dataset::Orders, SEED, 10, &base, 0).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    fs::remove_dir_all(base).unwrap();
}

// ---------------------------------------------------------------- cli

fn args(xs: &[&str]) -> Result<Args, String> {
    parse_args(&xs.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn cli_applies_defaults() {
    let a = args(&["events", "--out", "/tmp/x"]).unwrap();
    assert_eq!(a.dataset, Dataset::Events);
    assert_eq!(a.rows, 10_000_000);
    assert_eq!(a.seed, 42);
    assert_eq!(a.out, PathBuf::from("/tmp/x"));
    assert_eq!(args(&["orders", "--out", "o"]).unwrap().rows, 1_000_000);
    assert_eq!(args(&["vectors", "--out", "o"]).unwrap().rows, 1_000_000);
}

#[test]
fn cli_reads_every_flag_in_any_order() {
    let a = args(&["orders", "--seed", "7", "--out", "d", "--rows", "123"]).unwrap();
    assert_eq!(
        (a.dataset, a.rows, a.seed, a.out),
        (Dataset::Orders, 123, 7, PathBuf::from("d"))
    );
    let b = args(&[
        "vectors",
        "--rows",
        "0",
        "--out",
        "d",
        "--seed",
        "18446744073709551615",
    ])
    .unwrap();
    assert_eq!((b.rows, b.seed), (0, u64::MAX));
}

#[test]
fn cli_rejects_bad_input() {
    for bad in [
        &[][..],
        &["--out", "d"][..],
        &["hits", "--out", "d"][..],
        &["events"][..],
        &["events", "--out"][..],
        &["events", "--out", "d", "--rows"][..],
        &["events", "--out", "d", "--rows", "ten"][..],
        &["events", "--out", "d", "--rows", "-1"][..],
        &["events", "--out", "d", "--seed", "1.5"][..],
        &["events", "--out", "d", "--threads", "4"][..],
        &["events", "--out", "d", "extra"][..],
    ] {
        let r = args(bad);
        assert!(r.is_err(), "{bad:?} should be rejected, got {r:?}");
        assert!(!r.unwrap_err().is_empty(), "{bad:?}: empty error message");
    }
}
