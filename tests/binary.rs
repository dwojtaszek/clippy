//! Runs the built binary against a scratch home folder, so the entry point is tested too.

// Setup helpers unwrap on purpose: a failed setup should fail the test loudly.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration tests fail loudly on setup errors"
)]

pub mod common;

use std::path::Path;
use std::process::{Command, Output};

use common::ScratchDir;

fn run_binary(args: &[&str], home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_droid-usage"))
        .args(args)
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap()
}

#[test]
fn offline_run_prints_the_report_and_warns_about_missing_snapshots() {
    let home = ScratchDir::new("binary-offline");
    let output = run_binary(&["--offline"], &home);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("By source"), "{stdout}");
    assert!(stderr.contains("no price snapshot saved"), "{stderr}");
}

#[test]
fn unknown_argument_fails_and_is_named() {
    let home = ScratchDir::new("binary-bad-arg");
    let output = run_binary(&["--bogus"], &home);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--bogus"), "{stderr}");
}
