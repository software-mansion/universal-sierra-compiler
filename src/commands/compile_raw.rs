use anyhow::{Context, Result};
use cairo_lang_sierra::program::Program;
use cairo_lang_sierra_to_casm::compiler::CairoProgramDebugInfo;
use cairo_lang_sierra_to_casm::metadata::{calc_metadata, MetadataComputationConfig};
use clap::Args;
use serde::Deserialize;
use serde_json::{json, Value};
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

// `sierra_artifact` should be a json containing `sierra_program`
pub fn compile(sierra_artifact: Value) -> Result<Value> {
    let sierra_artifact: SierraArtifact = serde_json::from_value(sierra_artifact)
        .context("Unable to deserialize Sierra. Make sure it is in a correct format")?;
    let metadata_config = MetadataComputationConfig::default();
    let metadata = calc_metadata(&sierra_artifact.sierra_program, metadata_config)?;

    let cairo_program = cairo_lang_sierra_to_casm::compiler::compile(
        &sierra_artifact.sierra_program,
        &metadata,
        true,
    )?;
    let assembled_cairo_program = cairo_program.assemble();

    Ok(json!({
        "assembled_cairo_program": {
            "bytecode": serde_json::to_value(assembled_cairo_program.bytecode)?,
            "hints": serde_json::to_value(assembled_cairo_program.hints)?
        },
        "debug_info": serde_json::to_value(serialize_cairo_program_debug_info(&cairo_program.debug_info))?
    }))
}

fn serialize_cairo_program_debug_info(debug_info: &CairoProgramDebugInfo) -> Vec<(usize, usize)> {
    debug_info
        .sierra_statement_info
        .iter()
        .map(|statement_debug_info| {
            (
                statement_debug_info.code_offset,
                statement_debug_info.instruction_idx,
            )
        })
        .collect()
}
