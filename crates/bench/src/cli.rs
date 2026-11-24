//! Command-line arguments of `facet-gen`.

use std::path::PathBuf;

use crate::datagen::Dataset;

pub const USAGE: &str =
    "usage: facet-gen <events|orders|vectors> [--rows N] [--seed S] --out <dir>";

/// Default `--seed`.
pub const DEFAULT_SEED: u64 = 42;

/// Parsed `facet-gen` arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args {
    pub dataset: Dataset,
    pub rows: u64,
    pub seed: u64,
    pub out: PathBuf,
}

/// Parses the arguments after the program name.
pub fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut it = args.iter();
    let name = it.next().ok_or_else(|| USAGE.to_string())?;
    let dataset =
        Dataset::from_name(name).ok_or_else(|| format!("unknown dataset {name:?}\n{USAGE}"))?;
    let (mut rows, mut seed, mut out) = (dataset.default_rows(), DEFAULT_SEED, None);
    while let Some(flag) = it.next() {
        if !matches!(flag.as_str(), "--rows" | "--seed" | "--out") {
            return Err(format!("unexpected argument {flag:?}\n{USAGE}"));
        }
        let value = it.next().ok_or_else(|| format!("{flag} needs a value"))?;
        let number = || {
            value
                .parse::<u64>()
                .map_err(|_| format!("{flag} needs a non-negative integer, got {value:?}"))
        };
        match flag.as_str() {
            "--rows" => rows = number()?,
            "--seed" => seed = number()?,
            _ => out = Some(PathBuf::from(value)),
        }
    }
    let out = out.ok_or_else(|| format!("--out <dir> is required\n{USAGE}"))?;
    Ok(Args {
        dataset,
        rows,
        seed,
        out,
    })
}
