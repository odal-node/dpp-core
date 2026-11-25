//! A usage error leaves no verdict behind. The suite reads whatever is at `--output`
//! and ignores the exit status, so a file from an earlier run must not survive one.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn a_usage_error_clears_the_stale_output_and_exits_2() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("usage-error");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let output = dir.join("out.json");
    std::fs::write(&output, r#"{"result":"success"}"#).expect("a stale verdict");

    let status = Command::new(env!("CARGO_BIN_EXE_vc-jose-cose-suite"))
        .args([
            "issue",
            "--input",
            "in.json",
            "--key",
            "key.json",
            "--feature",
            "credential_jose",
            "--output",
        ])
        .arg(&output)
        .args(["--bogus", "x"])
        .status()
        .expect("the harness runs");

    assert_eq!(status.code(), Some(2));
    assert!(!output.exists(), "a stale verdict was left at --output");
}
