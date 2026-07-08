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

fn setup_major_zero_repo(commit: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["fix: base"])?;
    let repo = temp.path();
    git(repo, &["tag", "v0.1.0"])?;
    git(repo, &["commit", "--allow-empty", "-m", commit])?;
    Ok(temp)
}

#[test]
fn major_zero_default_bump_uses_pre_major_rules() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_major_zero_repo("feat: next")?;
    assert_version(temp.path(), &["version", "--bump"], "0.1.1")?;

    let temp = setup_major_zero_repo("feat!: next")?;
    assert_version(temp.path(), &["version", "--bump"], "0.2.0")?;

    Ok(())
}

#[test]
fn bump_matches_default_types_case_insensitively() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["fix: base"])?;
    let repo = temp.path();
    git(repo, &["tag", "v1.0.0"])?;
    git(
        repo,
        &["commit", "--allow-empty", "-m", "FEAT: uppercase feature"],
    )?;

    assert_version(repo, &["version", "--bump"], "1.1.0")?;

    let temp = setup_repo_with_commits(&["feat: base"])?;
    let repo = temp.path();
    git(repo, &["tag", "v1.0.0"])?;
    git(
        repo,
        &["commit", "--allow-empty", "-m", "FIX: uppercase fix"],
    )?;

    assert_version(repo, &["version", "--bump"], "1.0.1")?;

    Ok(())
}

#[test]
fn bump_matches_custom_type_case_insensitively() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["fix: base"])?;
    let repo = temp.path();
    git(repo, &["tag", "v1.0.0"])?;
    git(
        repo,
        &["commit", "--allow-empty", "-m", "CUSTOM: uppercase custom"],
    )?;
    fs::write(
        repo.join(".convco"),
        r#"types:
- type: custom
  increment: Patch
  section: Custom
  hidden: false
"#,
    )?;

    assert_version(repo, &["version", "--bump"], "1.0.1")?;

    Ok(())
}

#[test]
fn major_zero_can_be_treated_as_stable_with_cli_flag() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_major_zero_repo("feat: next")?;
    assert_version(
        temp.path(),
        &["version", "--bump", "--treat-major-zero-as-stable"],
        "0.2.0",
    )?;

    let temp = setup_major_zero_repo("feat!: next")?;
    assert_version(
        temp.path(),
        &["version", "--bump", "--treat-major-zero-as-stable"],
        "1.0.0",
    )?;

    Ok(())
}

#[test]
fn major_zero_can_be_treated_as_stable_with_env_var() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_major_zero_repo("feat: next")?;
    let repo = temp.path();

    let mut cmd = Command::cargo_bin("convco")?;
    let assert = cmd
        .current_dir(repo)
        .env("CONVCO_TREAT_MAJOR_ZERO_AS_STABLE", "true")
        .args(["version", "--bump"])
        .assert()
        .success();
    let stdout = std::str::from_utf8(&assert.get_output().stdout)?;
    assert_eq!(stdout, "0.2.0\n");

    Ok(())
}

#[test]
fn major_zero_can_be_treated_as_stable_with_config() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_major_zero_repo("feat: next")?;
    let repo = temp.path();
    fs::write(repo.join(".versionrc"), "treatMajorZeroAsStable: true\n")?;

    assert_version(repo, &["version", "--bump"], "0.2.0")?;

    Ok(())
}

#[test]
fn treat_major_zero_as_stable_flag_overrides_false_config() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = setup_major_zero_repo("feat: next")?;
    let repo = temp.path();
    fs::write(repo.join(".versionrc"), "treatMajorZeroAsStable: false\n")?;

    assert_version(
        repo,
        &["version", "--bump", "--treat-major-zero-as-stable"],
        "0.2.0",
    )?;

    Ok(())
}

#[test]
fn treat_major_zero_as_stable_requires_bump() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_major_zero_repo("feat: next")?;

    let mut cmd = Command::cargo_bin("convco")?;
    let assert = cmd
        .current_dir(temp.path())
        .args(["version", "--treat-major-zero-as-stable"])
        .assert()
        .failure();
    let stderr = std::str::from_utf8(&assert.get_output().stderr)?;
    assert!(stderr.contains("--bump"));

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
fn prerelease_bump_fails_when_base_version_is_already_released(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["fix: base"])?;
    let repo = temp.path();
    git(repo, &["tag", "v0.1.1"])?;
    git(repo, &["commit", "--allow-empty", "-m", "fix: prerelease"])?;
    git(repo, &["tag", "v0.1.2-beta.1"])?;
    git(repo, &["tag", "v0.1.2"])?;
    git(repo, &["commit", "--allow-empty", "-m", "chore: followup"])?;

    let output = run_convco_command(
        &["version", "--bump", "--prerelease", "beta"],
        Some(repo),
        false,
        "",
    )?;

    assert!(
        output.contains("version 0.1.2 is already released; cannot create prerelease 0.1.2-beta.2"),
        "expected prerelease conflict error, got:\n{output}"
    );

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

#[test]
fn exclude_only_pathspec_ignores_chart_only_commits_for_bump(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let repo = temp.path();

    git(repo, &["init"])?;
    git(repo, &["config", "user.name", "Convco Test"])?;
    git(repo, &["config", "user.email", "test@example.com"])?;

    fs::create_dir_all(repo.join("src"))?;
    fs::write(repo.join("src/app.txt"), "base")?;
    git(repo, &["add", "src/app.txt"])?;
    git(repo, &["commit", "-m", "feat: base"])?;
    git(repo, &["tag", "v1.0.0"])?;

    fs::create_dir_all(repo.join("charts"))?;
    fs::write(repo.join("charts/chart.txt"), "chart")?;
    git(repo, &["add", "charts/chart.txt"])?;
    git(repo, &["commit", "-m", "feat: chart only"])?;

    fs::write(repo.join("src/app.txt"), "source fix")?;
    git(repo, &["add", "src/app.txt"])?;
    git(repo, &["commit", "-m", "fix: source only"])?;

    assert_version(
        repo,
        &[
            "version",
            "--bump",
            "--paths",
            "src,:(exclude)src/generated",
        ],
        "1.0.1",
    )?;
    assert_version(
        repo,
        &[
            "version",
            "--bump",
            "--paths",
            "src",
            "--paths",
            ":(exclude)src/generated",
        ],
        "1.0.1",
    )?;
    assert_version(
        repo,
        &["version", "--bump", "--paths", ":(exclude)charts"],
        "1.0.1",
    )?;
    assert_version(repo, &["version", "--bump", "--paths", ":!charts"], "1.0.1")?;

    Ok(())
}

#[test]
fn mixed_excluded_and_included_commit_is_used_for_bump() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let repo = temp.path();

    git(repo, &["init"])?;
    git(repo, &["config", "user.name", "Convco Test"])?;
    git(repo, &["config", "user.email", "test@example.com"])?;

    fs::create_dir_all(repo.join("src"))?;
    fs::create_dir_all(repo.join("charts"))?;
    fs::write(repo.join("src/app.txt"), "base")?;
    git(repo, &["add", "src/app.txt"])?;
    git(repo, &["commit", "-m", "feat: base"])?;
    git(repo, &["tag", "v1.0.0"])?;

    fs::write(repo.join("charts/chart.txt"), "chart")?;
    fs::write(repo.join("src/app.txt"), "source feature")?;
    git(repo, &["add", "charts/chart.txt", "src/app.txt"])?;
    git(repo, &["commit", "-m", "feat: mixed source and chart"])?;

    assert_version(
        repo,
        &["version", "--bump", "--paths", ":(exclude)charts"],
        "1.1.0",
    )?;

    Ok(())
}
