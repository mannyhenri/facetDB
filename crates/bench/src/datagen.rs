//! Deterministic benchmark datasets: wide web events, narrow keyed orders and clustered vectors.
//!
//! Row `i` of a dataset depends only on `(dataset, seed, i)`: each row draws from its own
//! PRNG seeded from those three values. Output is therefore identical for any thread count or
//! range split, and `rows` and `write_files` agree because `rows` parses the very CSV line that
//! `write_files` writes.

use std::fs::{self, File};
use std::io::{self, Write};
use std::ops::Range;
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use facet_core::{ColId, Column, DataType, Row, Schema, Value};

/// Dimensions of every vector in the vectors dataset.
pub const VECTOR_DIM: usize = 128;
/// Number of distinct categories in the vectors dataset.
pub const VECTOR_CATEGORIES: usize = 100;

/// Rows generated per unit of parallel work.
const CHUNK_ROWS: u64 = 1 << 16;
/// 2024-01-01T00:00:00Z in epoch milliseconds; events are ~250 ms apart from here.
const EVENTS_START_MS: u64 = 1_704_067_200_000;

/// One of the three benchmark datasets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dataset {
    Events,
    Orders,
    Vectors,
}

impl Dataset {
    pub const ALL: [Dataset; 3] = [Dataset::Events, Dataset::Orders, Dataset::Vectors];

    /// The name used on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Dataset::Events => "events",
            Dataset::Orders => "orders",
            Dataset::Vectors => "vectors",
        }
    }

    pub fn from_name(name: &str) -> Option<Dataset> {
        Dataset::ALL.into_iter().find(|d| d.name() == name)
    }

    pub fn default_rows(self) -> u64 {
        match self {
            Dataset::Events => 10_000_000,
            Dataset::Orders | Dataset::Vectors => 1_000_000,
        }
    }

    /// Files `write_files` creates in the output directory.
    pub fn file_names(self) -> &'static [&'static str] {
        match self {
            Dataset::Events => &["events.csv"],
            Dataset::Orders => &["orders.csv"],
            Dataset::Vectors => &["vectors.f32", "vectors_meta.csv"],
        }
    }

    pub fn schema(self) -> Schema {
        use DataType::*;
        let cols: &[(&str, DataType, bool)] = match self {
            Dataset::Events => &[
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
            ],
            Dataset::Orders => &[
                ("order_id", Int, false),
                ("customer_id", Int, false),
                ("product_id", Int, false),
                ("quantity", Int, false),
                ("price", Float, false),
                ("status", Str, false),
            ],
            Dataset::Vectors => &[
                ("id", Int, false),
                ("vector", Vector { dim: VECTOR_DIM }, false),
                ("category", Str, false),
                ("price", Float, false),
            ],
        };
        let columns = cols
            .iter()
            .map(|&(name, t, nullable)| Column::new(name, t, nullable))
            .collect();
        Schema::new(columns, ColId(0)).expect("built-in dataset schema is valid")
    }

    /// The CSV file: every column except the vector, which goes to `vectors.f32`.
    fn csv_file(self) -> &'static str {
        self.file_names()[self.file_names().len() - 1]
    }

    fn salt(self) -> u64 {
        match self {
            Dataset::Events => 0x6576_656e_7473,
            Dataset::Orders => 0x6f72_6465_7273,
            Dataset::Vectors => 0x7665_6374_6f72,
        }
    }
}

/// Rows `ids` of `dataset` for `seed`, as Facet rows in schema order.
pub fn rows(dataset: Dataset, seed: u64, ids: Range<u64>) -> impl Iterator<Item = Row> {
    let gen = Generator::new(dataset, seed);
    ids.map(move |i| gen.row(i))
}

/// Writes rows `0..rows` of `dataset` into `out` (created if missing) as the files named by
/// `Dataset::file_names`, generating chunks on `threads` threads. The bytes do not depend on
/// `threads`.
pub fn write_files(
    dataset: Dataset,
    seed: u64,
    rows: u64,
    out: &Path,
    threads: usize,
) -> io::Result<()> {
    if threads == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "threads must be at least 1",
        ));
    }
    fs::create_dir_all(out)?;
    let gen = Generator::new(dataset, seed);
    let mut csv = File::create(out.join(dataset.csv_file()))?;
    let mut vectors = match dataset {
        Dataset::Vectors => Some(File::create(out.join("vectors.f32"))?),
        _ => None,
    };
    csv.write_all(gen.header().as_bytes())?;

    // Worker w generates chunks w, w + threads, ...; reading the workers round-robin writes
    // the chunks in order while at most two per worker wait in memory.
    let chunks = rows.div_ceil(CHUNK_ROWS);
    let threads = threads.min(chunks.max(1) as usize);
    thread::scope(|s| {
        let workers: Vec<_> = (0..threads)
            .map(|w| {
                let (tx, rx) = mpsc::sync_channel(2);
                let gen = &gen;
                s.spawn(move || {
                    for c in (w as u64..chunks).step_by(threads) {
                        let ids = c * CHUNK_ROWS..((c + 1) * CHUNK_ROWS).min(rows);
                        let (mut text, mut floats) = (Vec::new(), Vec::new());
                        for i in ids {
                            gen.write_row(i, &mut text, &mut floats);
                        }
                        if tx.send((text, floats)).is_err() {
                            return; // the writer stopped on an I/O error
                        }
                    }
                });
                rx
            })
            .collect();
        for c in 0..chunks {
            let (text, floats) = workers[(c % threads as u64) as usize]
                .recv()
                .expect("generator thread panicked");
            csv.write_all(&text)?;
            if let Some(f) = vectors.as_mut() {
                f.write_all(&floats)?;
            }
        }
        Ok(())
    })
}

/// Generates single rows of one dataset for one seed.
struct Generator {
    dataset: Dataset,
    schema: Schema,
    key: u64,
    /// `VECTOR_CATEGORIES` x `VECTOR_DIM` centroids (vectors dataset only).
    centroids: Vec<f32>,
}

impl Generator {
    fn new(dataset: Dataset, seed: u64) -> Generator {
        let key = mix64(seed ^ dataset.salt());
        let mut centroids = Vec::new();
        if dataset == Dataset::Vectors {
            let mut r = Rng(mix64(key ^ 0x63_656e_7472_6f69));
            centroids = (0..VECTOR_CATEGORIES * VECTOR_DIM)
                .map(|_| (r.unit() * 2.0 - 1.0) as f32)
                .collect();
        }
        Generator {
            dataset,
            schema: dataset.schema(),
            key,
            centroids,
        }
    }

    /// The CSV header line: the names of the non-vector columns.
    fn header(&self) -> String {
        let names: Vec<&str> = self
            .schema
            .columns()
            .iter()
            .filter(|c| !matches!(c.data_type, DataType::Vector { .. }))
            .map(|c| c.name.as_str())
            .collect();
        names.join(",") + "\n"
    }

    /// Row `i` as a `Row`: its CSV line parsed back, plus its vector.
    fn row(&self, i: u64) -> Row {
        let (mut text, mut floats) = (Vec::new(), Vec::new());
        self.write_row(i, &mut text, &mut floats);
        let line = std::str::from_utf8(&text).expect("generated CSV is ASCII");
        let mut fields = line.trim_end_matches('\n').split(',');
        let values = self
            .schema
            .columns()
            .iter()
            .map(|c| match c.data_type {
                DataType::Vector { .. } => Value::Vector(
                    floats
                        .chunks_exact(4)
                        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                        .collect(),
                ),
                t => parse_field(
                    fields.next().expect("CSV line has every field"),
                    t,
                    c.nullable,
                ),
            })
            .collect();
        Row(values)
    }

    /// Appends row `i`'s CSV line to `text` and, for vectors, its little-endian f32s to `floats`.
    fn write_row(&self, i: u64, text: &mut Vec<u8>, floats: &mut Vec<u8>) {
        let mut r = Rng(mix64(self.key.wrapping_add(i)));
        let mut line = Line {
            out: text,
            started: false,
        };
        match self.dataset {
            Dataset::Events => write_event(&mut r, i, &mut line),
            Dataset::Orders => write_order(&mut r, i, &mut line),
            Dataset::Vectors => self.write_vector(&mut r, i, &mut line, floats),
        }
        line.out.push(b'\n');
    }

    fn write_vector(&self, r: &mut Rng, i: u64, line: &mut Line, floats: &mut Vec<u8>) {
        let category = r.skewed(VECTOR_CATEGORIES as u64) as usize;
        line.int(i);
        let name = line.field();
        name.extend_from_slice(b"cat_");
        name.push(b'0' + (category / 10) as u8);
        name.push(b'0' + (category % 10) as u8);
        push_cents(line.field(), 100 + r.skewed(99_900)); // price
        let centroid = &self.centroids[category * VECTOR_DIM..(category + 1) * VECTOR_DIM];
        for c in centroid {
            // Roughly normal noise (sum of three uniforms), standard deviation ~0.1.
            let noise = (r.unit() + r.unit() + r.unit() - 1.5) * 0.2;
            floats.extend_from_slice(&(c + noise as f32).to_le_bytes());
        }
    }
}

const SECTIONS: &[&str] = &[
    "home", "search", "product", "category", "cart", "checkout", "account", "help", "blog", "deals",
];
const REFERERS: &[&str] = &[
    "",
    "https://www.google.com/",
    "https://www.bing.com/",
    "https://t.co/",
    "https://www.facebook.com/",
    "https://news.ycombinator.com/",
    "https://duckduckgo.com/",
    "https://www.reddit.com/",
];
const SEARCH_WORDS: &[&str] = &[
    "shoes",
    "laptop",
    "phone",
    "case",
    "red",
    "cheap",
    "best",
    "wireless",
    "headphones",
    "gift",
    "kids",
    "running",
    "coffee",
    "desk",
    "lamp",
    "camera",
    "sale",
    "black",
    "mini",
    "pro",
];
const COUNTRIES: &[&str] = &[
    "US", "DE", "GB", "FR", "IN", "BR", "JP", "CA", "IT", "ES", "NL", "PL", "SE", "AU", "MX", "KR",
    "TR", "CH", "BE", "NO",
];
const BROWSERS: &[&str] = &[
    "Chrome",
    "Safari",
    "Edge",
    "Firefox",
    "Samsung Internet",
    "Opera",
    "Yandex",
];
const OSES: &[&str] = &["Windows", "Android", "iOS", "macOS", "Linux", "ChromeOS"];
const DEVICES: &[&str] = &["desktop", "mobile", "tablet"];
/// Screen resolutions per device type, in `DEVICES` order.
const RESOLUTIONS: [&[(u64, u64)]; 3] = [
    &[
        (1920, 1080),
        (1366, 768),
        (2560, 1440),
        (1536, 864),
        (1440, 900),
    ],
    &[(390, 844), (412, 915), (360, 800), (414, 896), (375, 667)],
    &[(810, 1080), (768, 1024), (800, 1280), (1024, 1366)],
];
const STATUSES: &[&str] = &[
    "delivered",
    "shipped",
    "paid",
    "pending",
    "cancelled",
    "returned",
];

fn write_event(r: &mut Rng, i: u64, line: &mut Line) {
    let user = r.skewed(1_000_000);
    let time = EVENTS_START_MS + i * 250 + r.below(250);
    let section = r.pick_skewed(SECTIONS);
    let page = r.skewed(10_000);
    line.int(i);
    line.int(time);
    line.int(user);
    line.int(user * 1_000 + r.below(1_000)); // session_id
    let url = line.field();
    url.extend_from_slice(b"https://shop.example/");
    url.extend_from_slice(section.as_bytes());
    url.push(b'/');
    push_int(url, page);
    line.text(r.pick_skewed(REFERERS));
    let title = line.field();
    title.extend_from_slice(section.as_bytes());
    title.extend_from_slice(b" page ");
    push_int(title, page);
    let phrase = line.field();
    if r.below(100) < 15 {
        phrase.extend_from_slice(r.pick_skewed(SEARCH_WORDS).as_bytes());
        phrase.push(b' ');
        phrase.extend_from_slice(r.pick_skewed(SEARCH_WORDS).as_bytes());
    }
    let country = r.skewed(COUNTRIES.len() as u64);
    line.text(COUNTRIES[country as usize]);
    line.int(country * 100 + r.skewed(100)); // region_id
    line.text(r.pick_skewed(BROWSERS));
    line.text(r.pick_skewed(OSES));
    let device = r.skewed(DEVICES.len() as u64) as usize;
    line.text(DEVICES[device]);
    line.int(u64::from(device != 0)); // is_mobile
    let (w, h) = RESOLUTIONS[device][r.skewed(RESOLUTIONS[device].len() as u64) as usize];
    line.int(w);
    line.int(h);
    line.int(r.skewed(600_000)); // duration_ms
    let revenue = line.field(); // NULL (empty) for most rows
    if r.below(100) < 3 {
        push_cents(revenue, 1 + r.skewed(50_000));
    }
}

fn write_order(r: &mut Rng, i: u64, line: &mut Line) {
    line.int(i);
    line.int(r.skewed(200_000)); // customer_id
    line.int(r.skewed(20_000)); // product_id
    line.int(1 + r.skewed(10)); // quantity
    push_cents(line.field(), 199 + r.skewed(49_800)); // price
    line.text(r.pick_skewed(STATUSES));
}

/// Appends one CSV line's fields to a buffer, putting a comma before every field but the first.
struct Line<'a> {
    out: &'a mut Vec<u8>,
    started: bool,
}

impl Line<'_> {
    /// Starts the next field and returns the buffer to write it into.
    fn field(&mut self) -> &mut Vec<u8> {
        if self.started {
            self.out.push(b',');
        }
        self.started = true;
        self.out
    }

    fn int(&mut self, n: u64) {
        push_int(self.field(), n);
    }

    fn text(&mut self, s: &str) {
        self.field().extend_from_slice(s.as_bytes());
    }
}

/// Parses one CSV field as written by this module; empty in a nullable column is NULL.
fn parse_field(field: &str, data_type: DataType, nullable: bool) -> Value {
    if field.is_empty() && nullable {
        return Value::Null;
    }
    match data_type {
        DataType::Int => Value::Int(field.parse().expect("generated Int field")),
        DataType::Float => Value::Float(field.parse().expect("generated Float field")),
        DataType::Str => Value::Str(field.to_string()),
        DataType::Bytes | DataType::Vector { .. } => unreachable!("not stored in CSV"),
    }
}

/// Appends the decimal digits of `n`.
fn push_int(out: &mut Vec<u8>, mut n: u64) {
    let mut digits = [0u8; 20];
    let mut at = digits.len();
    loop {
        at -= 1;
        digits[at] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out.extend_from_slice(&digits[at..]);
}

/// Appends `cents / 100` with exactly two decimals (`1234` -> `12.34`).
fn push_cents(out: &mut Vec<u8>, cents: u64) {
    push_int(out, cents / 100);
    out.push(b'.');
    out.push(b'0' + (cents / 10 % 10) as u8);
    out.push(b'0' + (cents % 10) as u8);
}

/// SplitMix64's output function: a bijective 64-bit mixer.
fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// SplitMix64. In-crate so the generated bytes never change with a dependency upgrade.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        mix64(self.0)
    }

    /// Uniform in `[0, 1)`.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform in `0..n`.
    fn below(&mut self, n: u64) -> u64 {
        ((u128::from(self.next()) * u128::from(n)) >> 64) as u64
    }

    /// In `0..n`, skewed towards 0 (Zipf-like): `n * u^3` for uniform `u`.
    fn skewed(&mut self, n: u64) -> u64 {
        let u = self.unit();
        ((n as f64 * u * u * u) as u64).min(n - 1)
    }

    fn pick_skewed<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[self.skewed(xs.len() as u64) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_int_and_cents_format_like_display() {
        for n in [0u64, 7, 10, 99, 1_000, 1_704_067_200_000, u64::MAX] {
            let mut b = Vec::new();
            push_int(&mut b, n);
            assert_eq!(b, n.to_string().into_bytes());
        }
        for (cents, s) in [(0, "0.00"), (5, "0.05"), (199, "1.99"), (1_000, "10.00")] {
            let mut b = Vec::new();
            push_cents(&mut b, cents);
            assert_eq!(b, s.as_bytes());
        }
    }

    #[test]
    fn skewed_stays_in_range() {
        let mut r = Rng(1);
        for n in [1u64, 2, 3, 100] {
            for _ in 0..10_000 {
                assert!(r.skewed(n) < n);
            }
        }
    }
}
