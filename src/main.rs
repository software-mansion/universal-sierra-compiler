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

#[tracing::instrument(skip_all, level = "info")]
fn read_json(file_path: PathBuf) -> Result<Value> {
    let sierra_file = File::open(file_path).context("Unable to open json file")?;
    let sierra_file_reader = BufReader::new(sierra_file);

    serde_json::from_reader(sierra_file_reader).context("Unable to read json file")
}

#[tracing::instrument(skip_all, level = "info")]
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
    let _g = init_logging();

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

#[cfg(not(feature = "tracing"))]
fn init_logging() -> Option<impl Drop> {
    struct Zst;
    impl Drop for Zst {
        fn drop(&mut self) {}
    }
    Option::<Zst>::None
}

#[cfg(feature = "tracing")]
fn init_logging() -> Option<impl Drop> {
    use chrono::Local;
    use std::fs;

    use std::path::PathBuf;
    use tracing_chrome::ChromeLayerBuilder;
    use tracing_subscriber::filter::{EnvFilter, LevelFilter, Targets};
    use tracing_subscriber::fmt::time::Uptime;
    use tracing_subscriber::fmt::Layer;
    use tracing_subscriber::prelude::*;

    let mut guard = None;

    let fmt_layer = Layer::new()
        .with_writer(std::io::stderr)
        .with_timer(Uptime::default())
        .with_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::TRACE.into())
                .with_env_var("USC_LOG")
                .from_env_lossy(),
        );

    // Disabled unless explicitly enabled with env var.
    let tracing_profile = is_truthy_env("USC_TRACING_PROFILE", false);

    let profile_layer = if tracing_profile {
        let mut path = PathBuf::from(format!("./usc-profile-{}.json", Local::now().to_rfc3339()));

        // Create the file now, so that we early panic, and `fs::canonicalize` will work.
        let profile_file = fs::File::create(&path).expect("failed to create profile file");

        // Try to canonicalise the path so that it is easier to find the file from logs.
        if let Ok(canonical) = fs::canonicalize(&path) {
            path = canonical;
        }

        eprintln!(
            "this USC run will output tracing profile to: {}",
            path.display()
        );
        eprintln!(
            "open that file with https://ui.perfetto.dev (or chrome://tracing) to analyze it"
        );

        let (profile_layer, profile_layer_guard) = ChromeLayerBuilder::new()
            .writer(profile_file)
            .include_args(true)
            .build();

        // Filter out less important logs because they're too verbose,
        // and with them the profile file quickly grows to several GBs of data.
        let profile_layer = profile_layer.with_filter(
            Targets::new()
                .with_default(LevelFilter::TRACE)
                .with_target("salsa", LevelFilter::WARN),
        );

        guard = Some(profile_layer_guard);
        Some(profile_layer)
    } else {
        None
    };

    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(profile_layer),
    )
    .expect("could not set up global logger");

    guard
}

#[cfg(feature = "tracing")]
#[must_use]
pub fn is_truthy_env(name: &str, default: bool) -> bool {
    std::env::var(name).ok().map_or(default, |var| {
        let s = var.as_str();
        s == "true" || s == "1"
    })
}
