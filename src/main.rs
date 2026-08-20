use anyhow::Context;
use clap::Parser;
use source_builder::{file_determine, inserter, macro_finder};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    source: PathBuf,
    #[arg(short, long)]
    library: String,
    #[arg(short, long)]
    path: PathBuf,
    #[arg(short, long)]
    output: PathBuf,
}

fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let Cli {
        source,
        library: library_name,
        path: library_path,
        output,
    } = cli;
    let library_path = library_path.join("src");

    // step 1: precompute macro file map
    let macro_file_map = macro_finder::precompute_macro_file_map(&library_path)
        .context("failed to collect exported macros of the library")?;
    dbg!(&macro_file_map);

    // step 2: find library files to include
    let mut files: Vec<PathBuf> =
        file_determine::determine_files(&source, &library_name, &library_path, &macro_file_map)
            .context("failed to determine which library files to include")?
            .into_iter()
            .collect();
    files.sort();
    files.insert(0, library_path.join("lib.rs"));
    dbg!(&files);

    let content = inserter::gen_file(
        &source,
        &library_name,
        &library_path,
        files,
        &macro_file_map,
    )
    .context("failed to generate output file")?;
    let mut out = fs::File::create(&output)
        .with_context(|| format!("failed to create `{}`", output.display()))?;
    out.write_all(content.as_bytes())
        .with_context(|| format!("failed to write `{}`", output.display()))?;
    out.flush()
        .with_context(|| format!("failed to flush `{}`", output.display()))?;
    Ok(())
}
