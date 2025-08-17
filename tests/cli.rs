#![cfg(feature = "integration-tests")]

mod commands;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command as StdCommand,
    sync::OnceLock,
};

use assert_cmd::{assert::Assert, Command};
use insta::assert_snapshot;
use regex::Regex;
use tempfile::{tempdir, TempDir};

fn git(repo: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = StdCommand::new("git")
        .args(args)
        .current_dir(repo)
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "git {} failed: {}{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
}

fn setup_repo_with_commits(messages: &[&str]) -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let repo = temp.path();

    git(repo, &["init"])?;
    git(repo, &["config", "user.name", "Convco Test"])?;
    git(repo, &["config", "user.email", "test@example.com"])?;

    for message in messages {
        git(repo, &["commit", "--allow-empty", "-m", message])?;
    }

    Ok(temp)
}

fn setup_repo_with_merge_commit(
    merge_message: &str,
    feature_branch_commit: &str,
) -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let repo = temp.path();

    git(repo, &["init"])?;
    git(repo, &["config", "user.name", "Convco Test"])?;
    git(repo, &["config", "user.email", "test@example.com"])?;

    git(repo, &["commit", "--allow-empty", "-m", "feat: base"])?;
    git(repo, &["checkout", "-b", "feature"])?;
    git(
        repo,
        &["commit", "--allow-empty", "-m", feature_branch_commit],
    )?;
    git(repo, &["checkout", "-"])?;
    git(repo, &["commit", "--allow-empty", "-m", "feat: main work"])?;
    git(repo, &["merge", "--no-ff", "feature", "-m", merge_message])?;

    Ok(temp)
}

fn setup_repo_with_revert_commit() -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let repo = temp.path();

    git(repo, &["init"])?;
    git(repo, &["config", "user.name", "Convco Test"])?;
    git(repo, &["config", "user.email", "test@example.com"])?;

    fs::write(repo.join("file.txt"), "base")?;
    git(repo, &["add", "file.txt"])?;
    git(repo, &["commit", "-m", "feat: base"])?;

    fs::write(repo.join("file.txt"), "change")?;
    git(repo, &["add", "file.txt"])?;
    git(repo, &["commit", "-m", "feat: change"])?;

    git(repo, &["revert", "--no-edit", "HEAD"])?;

    Ok(temp)
}

fn mask_short_oids(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if i + 7 <= chars.len()
            && chars[i..i + 7].iter().all(|c| {
                c.is_ascii_hexdigit() && (!c.is_ascii_alphabetic() || c.is_ascii_lowercase())
            })
            && (i == 0 || !chars[i - 1].is_ascii_hexdigit())
            && (i + 7 == chars.len() || !chars[i + 7].is_ascii_hexdigit())
        {
            result.push_str("<OID>");
            i += 7;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

fn sanitize_output(input: &str) -> String {
    mask_short_oids(input)
}

/// Runs a convco command with the given arguments and returns the sanitized output.
///
/// # Arguments
/// * `args` - Command line arguments to pass to convco
/// * `cwd` - Working directory for the command
/// * `expect_success` - Whether the command is expected to succeed (true) or fail (false)
/// * `snapshot_name` - Name to use for the snapshot
///
/// # Returns
/// The sanitized output of the command (stdout and stderr)
pub fn run_convco_command(
    args: &[&str],
    cwd: Option<&Path>,
    expect_success: bool,
    snapshot_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("convco")?;

    if let Some(cwd_path) = cwd {
        cmd.current_dir(cwd_path);
    }

    let assert = if expect_success {
        cmd.args(args).assert().success()
    } else {
        cmd.args(args).assert().failure()
    };

    let output = assert.get_output();
    let stdout = std::str::from_utf8(&output.stdout)?;
    let stderr = std::str::from_utf8(&output.stderr)?;
    let snapshot = format!("stdout:\n{stdout}---\nstderr:\n{stderr}");
    let sanitized = sanitize_output(&snapshot);

    if !snapshot_name.is_empty() {
        assert_snapshot!(snapshot_name, sanitized);
    }

    Ok(sanitized)
}
