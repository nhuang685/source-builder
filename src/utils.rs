use std::fs;
use std::path::Path;

use anyhow::Context;

pub(crate) fn parse_file(path: &Path) -> anyhow::Result<syn::File> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    syn::parse_file(&contents).with_context(|| format!("failed to parse {}", path.display()))
}
