use anyhow::{Context, Result};
use cairo_lang_sierra::program::Program;
use cairo_lang_sierra_to_casm::compiler::{CairoProgramDebugInfo, SierraToCasmConfig};
use cairo_lang_sierra_to_casm::metadata::{calc_metadata, Metadata, MetadataComputationConfig};
use cairo_lang_sierra_type_size::ProgramRegistryInfo;
use clap::Args;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::trace_span;

#[derive(Args)]
pub struct CompileRaw {
    /// Path to the sierra program json file, which should have
    /// `type_declarations`, `libfunc_declarations`, `statements` and `funcs` fields
    #[arg(short, long)]
    pub sierra_path: PathBuf,

    /// Path to where compilation result json file will be saved.
    /// It will consist of `assembled_cairo_program`, `debug_info` and `function_costs` fields
    #[arg(short, long)]
    pub output_path: Option<PathBuf>,

    /// Directory where compiled CASM entries should be cached.
    #[arg(long)]
    pub cache_dir: Option<PathBuf>,
}

/// Compiles Sierra of the plain Cairo code.
#[tracing::instrument(skip_all, level = "info")]
pub fn compile(sierra_program: &Program) -> Result<Value> {
    let metadata_config = MetadataComputationConfig::default();
    let span = trace_span!("calc_metadata");
    let program_info =
        ProgramRegistryInfo::new(sierra_program).with_context(|| "Failed building registry.")?;
    let metadata = {
        let _g = span.enter();
        calc_metadata(sierra_program, &program_info, metadata_config)?
    };

    let span = trace_span!("compile_sierra_to_casm");
    let cairo_program = {
        let _g = span.enter();
        cairo_lang_sierra_to_casm::compiler::compile(
            sierra_program,
            &program_info,
            &metadata,
            SierraToCasmConfig {
                gas_usage_check: true,
                max_bytecode_size: usize::MAX,
            },
        )?
    };
    let span = trace_span!("assemble_cairo_program");
    let assembled_cairo_program = {
        let _g = span.enter();
        cairo_program.assemble()
    };

    let span = trace_span!("serialize_result");
    Ok({
        let _g = span.enter();
        json!({
            "assembled_cairo_program": {
                "bytecode": serde_json::to_value(assembled_cairo_program.bytecode)?,
                "hints": serde_json::to_value(assembled_cairo_program.hints)?
            },
            "debug_info": serde_json::to_value(serialize_cairo_program_debug_info(&cairo_program.debug_info))?,
            "function_costs": serialize_function_costs(sierra_program, &metadata)
        })
    })
}

fn serialize_function_costs(
    sierra_program: &Program,
    metadata: &Metadata,
) -> HashMap<usize, Map<String, Value>> {
    sierra_program
        .funcs
        .iter()
        .map(|function| {
            let costs = metadata.gas_info.function_costs[&function.id]
                .iter()
                .map(|(token_type, value)| (token_type.name(), Value::from(*value)))
                .collect();

            (function.entry_point.0, costs)
        })
        .collect()
}

fn serialize_cairo_program_debug_info(debug_info: &CairoProgramDebugInfo) -> Vec<(usize, usize)> {
    debug_info
        .sierra_statement_info
        .iter()
        .map(|statement_debug_info| {
            (
                statement_debug_info.start_offset,
                statement_debug_info.instruction_idx,
            )
        })
        .collect()
}
