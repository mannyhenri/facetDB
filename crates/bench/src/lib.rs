//! Benchmarks for the Facet engine: deterministic data generators and benchmark runners.

pub mod cli;
pub mod datagen;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {
        assert_eq!(2 + 2, 4);
    }
}
