use anyhow::Result;
use cairo_lang_sierra::program::Program;
use cairo_lang_sierra_to_casm::compiler::{CairoProgramDebugInfo, SierraToCasmConfig};
use cairo_lang_sierra_to_casm::metadata::{calc_metadata, MetadataComputationConfig};
use clap::Args;
use serde_json::{json, Value};
use std::path::PathBuf;
use tracing::trace_span;

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
#[tracing::instrument(skip_all, level = "info")]
pub fn compile(sierra_program: &Program) -> Result<Value> {
    let metadata_config = MetadataComputationConfig::default();
    let span = trace_span!("calc_metadata");
    let metadata = {
        let _g = span.enter();
        calc_metadata(sierra_program, metadata_config)?
    };

    let span = trace_span!("compile_sierra_to_casm");
    let cairo_program = {
        let _g = span.enter();
        cairo_lang_sierra_to_casm::compiler::compile(
            sierra_program,
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
            "debug_info": serde_json::to_value(serialize_cairo_program_debug_info(&cairo_program.debug_info))?
        })
    })
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
