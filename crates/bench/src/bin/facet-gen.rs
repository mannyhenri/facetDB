//! `facet-gen`: writes a benchmark dataset to disk. See `facet_bench::cli::USAGE`.

use std::process::ExitCode;
use std::thread;
use std::time::Instant;

use facet_bench::cli::parse_args;
use facet_bench::datagen::write_files;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("facet-gen: {e}");
            return ExitCode::from(2);
        }
    };
    let threads = thread::available_parallelism().map_or(1, |n| n.get());
    let start = Instant::now();
    match write_files(args.dataset, args.seed, args.rows, &args.out, threads) {
        Ok(()) => {
            eprintln!(
                "facet-gen: wrote {} {} rows (seed {}) to {} in {:.2} s",
                args.rows,
                args.dataset.name(),
                args.seed,
                args.out.display(),
                start.elapsed().as_secs_f64()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("facet-gen: {}: {e}", args.out.display());
            ExitCode::FAILURE
        }
    }
}
