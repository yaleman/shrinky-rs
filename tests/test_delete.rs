use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use tempfile::TempDir;

fn fixture_path() -> PathBuf {
    PathBuf::from("tests/test_images/bruny-oysters.png")
}

fn copy_fixture_to_tempdir(tempdir: &TempDir, filename: &str) -> PathBuf {
    let destination = tempdir.path().join(filename);
    fs::copy(fixture_path(), &destination).expect("failed to copy fixture image");
    destination
}

fn output_path_for(input_path: &Path) -> PathBuf {
    input_path.with_extension("jpg")
}

fn run_shrinky(args: &[&str], stdin: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_shrinky-rs"));
    command.args(args);

    if stdin.is_some() {
        command.stdin(Stdio::piped());
    }

    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn shrinky-rs");

    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .expect("stdin should be available")
            .write_all(input.as_bytes())
            .expect("failed to write test stdin");
    }

    child
        .wait_with_output()
        .expect("failed to wait for shrinky-rs")
}

#[test]
fn test_delete_removes_source_after_successful_write_when_confirmed() {
    let tempdir = TempDir::new().expect("failed to create tempdir");
    let input = copy_fixture_to_tempdir(&tempdir, "delete-yes.png");
    let output = output_path_for(&input);

    let result = run_shrinky(
        &[
            "--delete",
            "--output-type",
            "jpg",
            input.to_str().expect("utf-8 path"),
        ],
        Some("y\n"),
    );

    assert!(
        result.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "optimized output should exist");
    assert!(!input.exists(), "source file should be deleted");
}

#[test]
fn test_delete_keeps_source_when_declined() {
    let tempdir = TempDir::new().expect("failed to create tempdir");
    let input = copy_fixture_to_tempdir(&tempdir, "delete-no.png");
    let output = output_path_for(&input);

    let result = run_shrinky(
        &[
            "--delete",
            "--output-type",
            "jpg",
            input.to_str().expect("utf-8 path"),
        ],
        Some("n\n"),
    );

    assert!(
        result.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "optimized output should exist");
    assert!(
        input.exists(),
        "source file should remain when deletion is declined"
    );
}

#[test]
fn test_delete_skips_removal_when_output_existed_and_force_is_used() {
    let tempdir = TempDir::new().expect("failed to create tempdir");
    let input = copy_fixture_to_tempdir(&tempdir, "force-keep.png");
    let output = output_path_for(&input);
    fs::write(&output, b"placeholder").expect("failed to create existing output");

    let result = run_shrinky(
        &[
            "--delete",
            "--force",
            "--output-type",
            "jpg",
            input.to_str().expect("utf-8 path"),
        ],
        Some("y\n"),
    );

    assert!(
        result.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "optimized output should exist");
    assert!(
        input.exists(),
        "source file should remain when the output path already existed"
    );
}

#[test]
fn test_delete_never_removes_source_when_write_fails() {
    let tempdir = TempDir::new().expect("failed to create tempdir");
    let input = copy_fixture_to_tempdir(&tempdir, "write-failure.png");
    let output = output_path_for(&input);
    fs::create_dir(&output).expect("failed to create directory at output path");

    let result = run_shrinky(
        &[
            "--delete",
            "--force",
            "--output-type",
            "jpg",
            input.to_str().expect("utf-8 path"),
        ],
        Some("y\n"),
    );

    assert!(
        !result.status.success(),
        "command should fail when the output path is a directory"
    );
    assert!(
        input.exists(),
        "source file should remain after write failure"
    );
}
