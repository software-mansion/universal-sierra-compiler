use anyhow::{Context, Error, Result};
use clap::{Parser, Subcommand};
use console::style;
use serde_json::{to_writer, Value};
use std::fs::File;
use std::path::PathBuf;
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

fn read_sierra(input_file_path: PathBuf) -> Result<Value> {
    let sierra_file = File::open(input_file_path).context("Unable to open sierra json file")?;

    serde_json::from_reader(sierra_file).context("Unable to read sierra json file")
}

fn output_casm(output_json: &Value, output_file_path: Option<PathBuf>) -> Result<()> {
    match output_file_path {
        Some(output_path) => {
            let casm_file =
                File::create(output_path).context("Unable to open/create casm json file")?;

            to_writer(casm_file, &output_json).context("Unable to save casm json file")?;
        }
        None => {
            println!("{}", serde_json::to_string(&output_json)?);
        }
    };

    Ok(())
}

fn main_execution() -> Result<bool> {
    let cli = Cli::parse();

    match cli.command {
        Commands::CompileContract(compile_contract) => {
            let sierra_json = read_sierra(compile_contract.sierra_input_path)?;

            let casm_json = commands::compile_contract::compile(sierra_json)?;

            output_casm(&casm_json, compile_contract.casm_output_path)?;
        }
        Commands::CompileRaw(compile_raw) => {
            let sierra_json = read_sierra(compile_raw.sierra_input_path)?;

            let cairo_program_json = commands::compile_raw::compile(sierra_json)?;

            output_casm(&cairo_program_json, compile_raw.cairo_program_output_path)?;
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
