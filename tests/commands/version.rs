use std::fs;

use assert_cmd::Command;
use tempfile::tempdir;

use super::super::{
    git, run_convco_command, setup_repo_with_commits, setup_repo_with_non_linear_version_tags,
};

fn assert_version(
    repo: &std::path::Path,
    args: &[&str],
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = run_convco_command(args, Some(repo), true, "")?;
    assert!(
        output.contains(&format!("stdout:\n{expected}\n---")),
        "expected version {expected}, got:\n{output}"
    );
    Ok(())
}

#[test]
fn forced_bump_env_vars_are_supported() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["feat: base"])?;
    let repo = temp.path();
    git(repo, &["tag", "v1.0.0"])?;

    let mut cmd = Command::cargo_bin("convco")?;
    let assert = cmd
        .current_dir(repo)
        .env("CONVCO_FORCE_MINOR_BUMP", "true")
        .arg("version")
        .assert()
        .success();
    let stdout = std::str::from_utf8(&assert.get_output().stdout)?;
    assert_eq!(stdout, "1.1.0\n");

    Ok(())
}

#[test]
fn non_linear_history_uses_highest_reachable_semver() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_non_linear_version_tags()?;
    let repo = temp.path();

    assert_version(repo, &["version"], "2.0.0")?;
    assert_version(repo, &["version", "--bump"], "2.1.0")?;

    Ok(())
}

#[test]
fn semver_like_blob_tags_are_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let repo = temp.path();

    git(repo, &["init"])?;
    git(repo, &["config", "user.name", "Convco Test"])?;
    git(repo, &["config", "user.email", "test@example.com"])?;
    fs::write(repo.join("file.txt"), "base")?;
    git(repo, &["add", "file.txt"])?;
    git(repo, &["commit", "-m", "feat: base"])?;
    git(repo, &["tag", "v1.0.0"])?;

    let output = std::process::Command::new("git")
        .args(["hash-object", "-w", "file.txt"])
        .current_dir(repo)
        .output()?;
    assert!(output.status.success());
    let blob = String::from_utf8(output.stdout)?;
    git(repo, &["tag", "v9.9.9", blob.trim()])?;

    assert_version(repo, &["version"], "1.0.0")?;

    Ok(())
}

#[test]
fn prerelease_bump_reuses_only_prerelease_tag_at_head() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["feat: first"])?;
    let repo = temp.path();
    git(repo, &["tag", "v0.1.0-rc.1"])?;

    assert_version(
        repo,
        &["version", "--bump", "--prerelease", "rc"],
        "0.1.0-rc.1",
    )?;
    assert_version(
        repo,
        &[
            "version",
            "--bump",
            "--prerelease",
            "rc",
            "--ignore-prereleases",
        ],
        "0.1.0-rc.1",
    )?;

    Ok(())
}

#[test]
fn prerelease_bump_reuses_matching_prerelease_tag() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["feat: base"])?;
    let repo = temp.path();
    git(repo, &["tag", "v1.0.0"])?;
    git(repo, &["commit", "--allow-empty", "-m", "feat: next"])?;
    git(repo, &["tag", "v1.1.0-rc.1"])?;

    assert_version(
        repo,
        &["version", "--bump", "--prerelease", "rc"],
        "1.1.0-rc.1",
    )?;
    assert_version(
        repo,
        &[
            "version",
            "--bump",
            "--prerelease",
            "rc",
            "--ignore-prereleases",
        ],
        "1.1.0-rc.1",
    )?;

    Ok(())
}

#[test]
fn prerelease_bump_advances_latest_prerelease_after_followup_commit(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["feat: base"])?;
    let repo = temp.path();
    git(repo, &["tag", "v1.0.0"])?;
    git(repo, &["commit", "--allow-empty", "-m", "feat: next"])?;
    git(repo, &["tag", "v1.1.0-rc.1"])?;
    git(repo, &["commit", "--allow-empty", "-m", "fix: followup"])?;

    assert_version(
        repo,
        &["version", "--bump", "--prerelease", "rc"],
        "1.1.0-rc.2",
    )?;
    assert_version(
        repo,
        &[
            "version",
            "--bump",
            "--prerelease",
            "rc",
            "--ignore-prereleases",
        ],
        "1.1.0-rc.2",
    )?;

    Ok(())
}

#[test]
fn paths_filter_merge_commits_against_first_parent_for_bump(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let repo = temp.path();

    git(repo, &["init"])?;
    git(repo, &["config", "user.name", "Convco Test"])?;
    git(repo, &["config", "user.email", "test@example.com"])?;

    fs::create_dir_all(repo.join("app"))?;
    fs::create_dir_all(repo.join("lib"))?;
    fs::write(repo.join("app/app.txt"), "base")?;
    git(repo, &["add", "app/app.txt"])?;
    git(repo, &["commit", "-m", "feat: base"])?;
    git(repo, &["tag", "v1.0.0"])?;

    git(repo, &["checkout", "-b", "feature"])?;
    fs::write(repo.join("app/app.txt"), "feature")?;
    git(repo, &["add", "app/app.txt"])?;
    git(repo, &["commit", "-m", "feat: app branch"])?;

    git(repo, &["checkout", "-"])?;
    fs::write(repo.join("lib/lib.txt"), "main")?;
    git(repo, &["add", "lib/lib.txt"])?;
    git(repo, &["commit", "-m", "fix: lib main"])?;
    git(
        repo,
        &["merge", "--no-ff", "feature", "-m", "feat: merge feature"],
    )?;

    assert_version(repo, &["version", "--paths", "lib", "--bump"], "1.0.1")?;

    Ok(())
}
