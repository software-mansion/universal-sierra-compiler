use anyhow::{Context, Result};
use cairo_lang_sierra::program::Program;
use cairo_lang_sierra_to_casm::compiler::CairoProgram;
use cairo_lang_sierra_to_casm::metadata::{calc_metadata, MetadataComputationConfig};
use clap::Args;
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Args)]
pub struct CompileRaw {
    /// Path to the sierra json file
    #[arg(short, long)]
    pub sierra_input_path: PathBuf,

    /// Path to where casm json file will be saved
    #[arg(short, long)]
    pub cairo_program_output_path: Option<PathBuf>,
}

#[derive(Deserialize)]
pub struct SierraArtifact {
    sierra_program: Program,
}

// TODO: Support other sierra versions.
// `sierra_artifact` should be a json containing `sierra_program`
pub fn compile(sierra_artifact: Value) -> Result<CairoProgram> {
    let sierra_artifact: SierraArtifact = serde_json::from_value(sierra_artifact)
        .context("Unable to deserialize Sierra. Make sure it is in a correct format")?;
    let metadata_config = MetadataComputationConfig::default();
    let metadata = calc_metadata(&sierra_artifact.sierra_program, metadata_config)?;

    Ok(cairo_lang_sierra_to_casm::compiler::compile(
        &sierra_artifact.sierra_program,
        &metadata,
        true,
    )?)
}
