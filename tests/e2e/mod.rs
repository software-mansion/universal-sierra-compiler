use snapbox::cmd::{cargo_bin, Command};
use std::path::PathBuf;
use tempfile::TempDir;

mod compile_contract;
mod compile_raw;

#[must_use]
fn runner(args: Vec<&str>, temp_dir: &TempDir) -> Command {
    Command::new(cargo_bin!("universal-sierra-compiler"))
        .current_dir(temp_dir.path())
        .args(args)
}

#[must_use]
fn temp_dir_with_sierra_file(dir_name: &str, file_name: &str) -> TempDir {
    let temp_dir = TempDir::new().expect("Unable to create a temporary directory");

    let src_dir = PathBuf::from("tests/data");

    fs_extra::file::copy(
        src_dir.join(dir_name).join(file_name),
        temp_dir.path().join(file_name),
        &fs_extra::file::CopyOptions::new().overwrite(true),
    )
    .unwrap_or_else(|_| panic!("Unable to copy {dir_name}/{file_name}"));

    temp_dir
}
