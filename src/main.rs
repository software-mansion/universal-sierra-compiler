use anyhow::{Context, Error, Result};
use clap::{Parser, Subcommand};
use console::style;
use serde_json::{json, to_writer};
use std::fs::File;
use universal_sierra_compiler::commands;
use universal_sierra_compiler::commands::compile_contract::CompileContract;
use universal_sierra_compiler::commands::compile_raw::CompileRaw;

#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    CompileContract(CompileContract),
    CompileRaw(CompileRaw),
}

fn print_error_message(error: &Error) {
    let error_tag = style("ERROR").red();
    println!("[{error_tag}] {error}");
}

fn main_execution() -> Result<bool> {
    let cli = Cli::parse();

    match cli.command {
        Commands::CompileContract(compile_contract) => {
            let sierra_file = File::open(compile_contract.sierra_input_path)
                .context("Unable to open sierra json file")?;
            let sierra_json =
                serde_json::from_reader(sierra_file).context("Unable to read sierra json file")?;

            let casm = commands::compile_contract::compile(sierra_json)?;
            let casm_json = serde_json::to_value(casm)?;

            match compile_contract.casm_output_path {
                Some(output_path) => {
                    let casm_file = File::create(output_path)
                        .context("Unable to open/create casm json file")?;

                    to_writer(casm_file, &casm_json).context("Unable to save casm json file")?;
                }
                None => {
                    println!("{}", serde_json::to_string(&casm_json)?);
                }
            };
        }
        Commands::CompileRaw(compile_raw) => {
            let sierra_file = File::open(compile_raw.sierra_input_path)
                .context("Unable to open sierra json file")?;
            let sierra_json =
                serde_json::from_reader(sierra_file).context("Unable to read sierra json file")?;

            let cairo_program = commands::compile_raw::compile(sierra_json)?;
            let assembled_cairo_program = cairo_program.assemble();

            let bytecode = serde_json::to_value(assembled_cairo_program.bytecode)?;
            let hints = serde_json::to_value(assembled_cairo_program.hints)?;
            let debug_info: Vec<(usize, usize)> = cairo_program
                .debug_info
                .sierra_statement_info
                .iter()
                .map(|statement_debug_info| {
                    (
                        statement_debug_info.code_offset,
                        statement_debug_info.instruction_idx,
                    )
                })
                .collect();
            let debug_info = serde_json::to_value(debug_info)?;

            let value = json!({
                "assembled_cairo_program": {
                    "bytecode": bytecode,
                    "hints": hints
                },
                "debug_info": debug_info
            });

            match compile_raw.cairo_program_output_path {
                Some(output_path) => {
                    let casm_file = File::create(output_path)
                        .context("Unable to open/create casm json file")?;

                    to_writer(casm_file, &value).context("Unable to save casm json file")?;
                }
                None => {
                    println!("{}", serde_json::to_string(&value)?);
                }
            };
        }
    }

    Ok(true)
}

fn main() {
    match main_execution() {
        Ok(true) => std::process::exit(0),
        Ok(false) => std::process::exit(1),
        Err(error) => {
            print_error_message(&error);
            std::process::exit(2);
        }
    };
}
