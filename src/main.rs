use anyhow::{Context, Error, Result};
use clap::{Parser, Subcommand};
use console::style;
use serde_json::{to_writer, Value};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

mod commands;

use commands::compile_contract::CompileContract;
use commands::compile_raw::CompileRaw;

#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // Compile sierra of the contract
    CompileContract(CompileContract),

    // Compile sierra program (cairo_lang_sierra::program::Program)
    CompileRaw(CompileRaw),
}

fn print_error_message(error: &Error) {
    let error_tag = style("ERROR").red();
    eprintln!("[{error_tag}] {error}");
}

fn read_json(file_path: PathBuf) -> Result<Value> {
    let sierra_file = File::open(file_path).context("Unable to open json file")?;
    let sierra_file_reader = BufReader::new(sierra_file);

    serde_json::from_reader(sierra_file_reader).context("Unable to read json file")
}

fn output_casm(output_json: &Value, output_file_path: Option<PathBuf>) -> Result<()> {
    match output_file_path {
        Some(output_path) => {
            let casm_file =
                File::create(output_path).context("Unable to open/create casm json file")?;
            let casm_file_writer = BufWriter::new(casm_file);

            to_writer(casm_file_writer, &output_json).context("Unable to save casm json file")?;
        }
        None => {
            println!("{}", serde_json::to_string(&output_json)?);
        }
    }

    Ok(())
}

fn main_execution() -> Result<bool> {
    let cli = Cli::parse();

    match cli.command {
        Commands::CompileContract(compile_contract) => {
            let sierra_json = read_json(compile_contract.sierra_path)?;

            let casm_json = commands::compile_contract::compile(sierra_json)?;

            output_casm(&casm_json, compile_contract.output_path)?;
        }
        Commands::CompileRaw(compile_raw) => {
            let sierra_json = read_json(compile_raw.sierra_path)?;

            let cairo_program_json = commands::compile_raw::compile(sierra_json)?;

            output_casm(&cairo_program_json, compile_raw.output_path)?;
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
