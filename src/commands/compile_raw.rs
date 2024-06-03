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

    // taken from cairo compiler.
    let builtins = vec![
            "pedersen_builtin",
            "range_check_builtin",
            "bitwise_builtin",
            "ec_op_builtin",
            "poseidon_builtin",
        ];
    
    let entry_point = calculate_entry_point(&sierra_program, "::main")?;
    
    Ok(json!({
        "assembled_cairo_program": {
            "bytecode": serde_json::to_value(assembled_cairo_program.bytecode)?,
            "hints": serde_json::to_value(assembled_cairo_program.hints)?,
            "builtins": serde_json::to_value(builtins)?,
            "entry_point": serde_json::to_value(entry_point)?
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

/// Finds first function ending with `name_suffix` and calculates the entry point.
fn calculate_entry_point(sierra_program: &Program, name_suffix: &str) -> Result<usize> {
    let main_func = sierra_program.funcs.iter().find(|f| {
        if let Some(name) = &f.id.debug_name {
            name.ends_with(name_suffix)
        } else {
            false
        }
    }).ok_or_else(|| anyhow::Error::msg("Main function not found in the Sierra program"))?;
    Ok(main_func.entry_point.0)
}