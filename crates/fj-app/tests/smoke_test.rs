use std::{
    fs::File,
    path::Path,
    process::{Command, Output, Stdio},
};

use stl::BinaryStlFile;
use tempfile::TempDir;

const FJ_APP: &str = env!("CARGO_BIN_EXE_fj-app");

#[test]
fn spacer() {
    let BinaryStlFile { header, triangles } = execute_model("spacer");
    assert_eq!(header.num_triangles, 42);
    assert_eq!(header.num_triangles as usize, triangles.len());
}

#[test]
fn test() {
    let BinaryStlFile { header, triangles } = execute_model("test");
    assert_eq!(header.num_triangles, 42);
    assert_eq!(header.num_triangles as usize, triangles.len());
}

#[test]
fn cuboid() {
    let BinaryStlFile { header, triangles } = execute_model("cuboid");
    assert_eq!(header.num_triangles, 42);
    assert_eq!(header.num_triangles as usize, triangles.len());
}

#[test]
fn star() {
    let BinaryStlFile { header, triangles } = execute_model("star");
    assert_eq!(header.num_triangles, 42);
    assert_eq!(header.num_triangles as usize, triangles.len());
}

fn execute_model(name: &str) -> BinaryStlFile {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let temp = TempDir::new().unwrap();
    let dest = temp.path().join("output.stl");

    let mut cmd = Command::new(FJ_APP);
    cmd.arg("--model")
        .arg(name)
        .arg("--export")
        .arg(&dest)
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

    let mut f = File::open(&dest).unwrap();
    stl::read_stl(&mut f).unwrap()
}
