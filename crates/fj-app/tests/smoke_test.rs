use std::{
    path::Path,
    process::{Command, Output, Stdio},
};

use tempfile::NamedTempFile;

const FJ_APP: &str = env!("CARGO_BIN_EXE_fj-app");

#[test]
fn spacer_model() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let dest = NamedTempFile::new().unwrap();

    let mut cmd = Command::new(FJ_APP);
    cmd.arg("--model")
        .arg("cuboid")
        .arg("--export")
        .arg(dest.path())
        .env("RUST_BACKTRACE", "1")
        .env("RUST_LOG", "debug")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(&project_root);
    let Output {
        status,
        stdout,
        stderr,
    } = cmd.output().unwrap();

    let stdout = String::from_utf8(stdout).unwrap();
    let stderr = String::from_utf8(stderr).unwrap();

    if !status.success() {
        println!("---- Stdout ----");
        println!("{stdout}");
        println!("---- Stderr ----");
        println!("{stderr}");
        panic!("`{cmd:?}` failed with exit code {:?}", status.code());
    }
}
