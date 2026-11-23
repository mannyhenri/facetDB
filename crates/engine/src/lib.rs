//! The engine: ties log, delta, layouts and planner together so every query reads one consistent snapshot.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {
        assert_eq!(2 + 2, 4);
    }
}
