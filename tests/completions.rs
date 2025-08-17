#![cfg(feature = "completions")]

use assert_cmd::Command;

#[test]
fn completions_run_without_git_repository() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("convco")?;
    let assert = cmd
        .current_dir(temp.path())
        .args(["completions", "bash"])
        .assert()
        .success();
    let stdout = std::str::from_utf8(&assert.get_output().stdout)?;

    assert!(stdout.contains("_convco"));

    Ok(())
}
