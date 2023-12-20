use anyhow::{Context, Error, Result};
use clap::Parser;
use console::style;
use serde_json::to_writer;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use universal_sierra_compiler::compile;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Path to the sierra json file
    #[arg(short, long)]
    sierra_input_path: PathBuf,

    /// Path to where casm json file will be saved
    #[arg(short, long)]
    casm_output_path: Option<PathBuf>,
}

fn print_error_message(error: &Error) {
    let error_tag = style("ERROR").red();
    println!("[{error_tag}] {error}");
}

fn main_execution() -> Result<bool> {
    let args = Args::parse();

    let sierra_file =
        File::open(args.sierra_input_path).context("Unable to open sierra json file")?;
    let sierra_json =
        serde_json::from_reader(sierra_file).context("Unable to read sierra json file")?;

    let casm = compile(sierra_json)?;
    let casm_json = serde_json::to_value(casm)?;

    match args.casm_output_path {
        Some(output_path) => {
            let casm_file = OpenOptions::new()
                .write(true)
                .create(true)
                .append(false)
                .open(output_path)
                .context("Unable to open/create casm json file")?;

            to_writer(casm_file, &casm_json).context("Unable to save casm json file")?;
        }
        None => {
            println!("{}", serde_json::to_string(&casm_json)?);
        }
    };

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
