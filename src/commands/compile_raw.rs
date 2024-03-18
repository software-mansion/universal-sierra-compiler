use anyhow::{Context, Result};
use cairo_lang_sierra::program::Program;
use cairo_lang_sierra_to_casm::compiler::{CairoProgramDebugInfo, SierraToCasmConfig};
use cairo_lang_sierra_to_casm::metadata::{calc_metadata, MetadataComputationConfig};
use clap::Args;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Args)]
pub struct CompileRaw {
    /// Path to the sierra program json file, which should have
    /// `type_declarations`, `libfunc_declarations`, `statements` and `funcs` fields
    #[arg(short, long)]
    pub sierra_path: PathBuf,

    /// Path to where compilation result json file will be saved.
    /// It will consist of `assembled_cairo_program` and `debug_info` fields
    #[arg(short, long)]
    pub output_path: Option<PathBuf>,
}

/// Compiles Sierra of the plain Cairo code.
pub fn compile(sierra_program: Value) -> Result<Value> {
    let sierra_program: Program = serde_json::from_value(sierra_program)
        .context("Unable to deserialize Sierra program. Make sure it is in a correct format")?;
    let metadata_config = MetadataComputationConfig::default();
    let metadata = calc_metadata(&sierra_program, metadata_config)?;

    let cairo_program = cairo_lang_sierra_to_casm::compiler::compile(
        &sierra_program,
        &metadata,
        SierraToCasmConfig {
            gas_usage_check: true,
            max_bytecode_size: usize::MAX,
        },
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
