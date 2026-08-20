use std::collections::HashSet;
use std::fmt::Write;

/// Accumulates errors found while walking a syntax tree, where the `syn`
/// visitor and folder traits do not allow returning a `Result`.
#[derive(Default)]
pub struct Errors(Vec<anyhow::Error>);

impl Errors {
    pub fn push(&mut self, error: anyhow::Error) {
        self.0.push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Turns the collected errors into a single error listing all of them.
    pub fn into_result(self) -> anyhow::Result<()> {
        if self.0.is_empty() {
            return Ok(());
        }
        let mut message = String::from("encountered errors:");
        let mut seen = HashSet::new();
        for error in &self.0 {
            let error = format!("{error:#}");
            if seen.insert(error.clone()) {
                let _ = write!(message, "\n  - {error}");
            }
        }
        Err(anyhow::anyhow!(message))
    }
}
