use std::{fs, process::Command as StdCommand};

use super::super::{run_convco_command, *};

fn setup_repo_with_version_tags() -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let repo = temp.path();

    git(repo, &["init"])?;
    git(repo, &["config", "user.name", "Convco Test"])?;
    git(repo, &["config", "user.email", "test@example.com"])?;

    // v1.0.0
    git(
        repo,
        &["commit", "--allow-empty", "-m", "feat: initial feature"],
    )?;
    git(repo, &["tag", "v1.0.0"])?;

    // v2.0.0
    git(
        repo,
        &["commit", "--allow-empty", "-m", "feat: second feature"],
    )?;
    git(repo, &["tag", "v2.0.0"])?;

    // v3.0.0
    git(
        repo,
        &["commit", "--allow-empty", "-m", "feat: third feature"],
    )?;
    git(repo, &["tag", "v3.0.0"])?;

    // unreleased commit after v3.0.0
    git(
        repo,
        &["commit", "--allow-empty", "-m", "feat: unreleased feature"],
    )?;

    Ok(temp)
}

#[test]
fn succeeds_on_conventional_commits() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["feat: initial conventional commit"])?;
    let repo = temp.path();

    run_convco_command(
        &["changelog"],
        Some(repo),
        true,
        "changelog_succeeds_on_conventional_commits",
    )?;

    Ok(())
}

#[test]
fn range_with_tag_upper_bound() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_version_tags()?;
    let repo = temp.path();

    // ..v2.0.0 should show v1.0.0..v2.0.0 and root..v1.0.0
    run_convco_command(
        &["changelog", "..v2.0.0"],
        Some(repo),
        true,
        "changelog_range_tag_upper_bound",
    )?;

    Ok(())
}

#[test]
fn range_with_tag_lower_bound() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_version_tags()?;
    let repo = temp.path();

    // v1.0.0.. should show v3.0.0..HEAD (unreleased), v2.0.0..v3.0.0, v1.0.0..v2.0.0
    run_convco_command(
        &["changelog", "v1.0.0.."],
        Some(repo),
        true,
        "changelog_range_tag_lower_bound",
    )?;

    Ok(())
}

#[test]
fn range_with_both_tags() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_version_tags()?;
    let repo = temp.path();

    // v1.0.0..v2.0.0 should show only v1.0.0..v2.0.0
    run_convco_command(
        &["changelog", "v1.0.0..v2.0.0"],
        Some(repo),
        true,
        "changelog_range_both_tags",
    )?;

    Ok(())
}

#[test]
fn range_with_sha_bounds() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_version_tags()?;
    let repo = temp.path();

    // Get the SHA of the v1.0.0 and v2.0.0 commits
    let v1_sha = String::from_utf8(
        StdCommand::new("git")
            .args(["rev-list", "-1", "v1.0.0"])
            .current_dir(repo)
            .output()?
            .stdout,
    )?;
    let v1_sha = v1_sha.trim();

    let v2_sha = String::from_utf8(
        StdCommand::new("git")
            .args(["rev-list", "-1", "v2.0.0"])
            .current_dir(repo)
            .output()?
            .stdout,
    )?;
    let v2_sha = v2_sha.trim();

    // Using SHAs should produce the same result as using tags
    let range = format!("{v1_sha}..{v2_sha}");
    run_convco_command(
        &["changelog", &range],
        Some(repo),
        true,
        "changelog_range_sha_bounds",
    )?;

    Ok(())
}

#[test]
fn range_with_sha_upper_bound() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_version_tags()?;
    let repo = temp.path();

    // Get the SHA of the HEAD commit (unreleased, after v3.0.0)
    let head_sha = String::from_utf8(
        StdCommand::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo)
            .output()?
            .stdout,
    )?;
    let head_sha = head_sha.trim();

    // ..<sha> should work like ..HEAD (show all sections including unreleased)
    let range = format!("..{head_sha}");
    run_convco_command(
        &["changelog", &range],
        Some(repo),
        true,
        "changelog_range_sha_upper_bound",
    )?;

    Ok(())
}

#[test]
fn paths_limit_commits_to_specified_directories() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let repo = temp.path();

    git(repo, &["init"])?;
    git(repo, &["config", "user.name", "Convco Test"])?;
    git(repo, &["config", "user.email", "test@example.com"])?;

    fs::create_dir_all(repo.join("packages/app"))?;
    fs::write(repo.join("packages/app/app.txt"), "app")?;
    git(repo, &["add", "packages/app/app.txt"])?;
    git(repo, &["commit", "-m", "feat(app): include app change"])?;

    fs::create_dir_all(repo.join("packages/lib"))?;
    fs::write(repo.join("packages/lib/lib.txt"), "lib")?;
    git(repo, &["add", "packages/lib/lib.txt"])?;
    git(repo, &["commit", "-m", "feat(lib): include lib change"])?;

    run_convco_command(
        &["changelog"],
        Some(repo),
        true,
        "changelog_paths_without_filter_includes_all_commits",
    )?;

    run_convco_command(
        &["changelog", "--paths", "packages/app"],
        Some(repo),
        true,
        "changelog_paths_with_filter_includes_only_matching",
    )?;

    Ok(())
}

#[test]
fn line_length_limits_wrapping() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&[
        "feat: subject with multiple words to test configurable line length wrapping behavior",
    ])?;
    let repo = temp.path();

    run_convco_command(
        &["changelog", "--line-length", "20"],
        Some(repo),
        true,
        "changelog_line_length_limits_wrapping",
    )?;

    Ok(())
}

#[test]
fn no_wrap_retains_long_lines() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["feat: a very long subject that should exceed the default wrapping limit to verify behavior"])?;
    let repo = temp.path();

    run_convco_command(
        &["changelog"],
        Some(repo),
        true,
        "changelog_wrap_applies_default_wrapping",
    )?;

    run_convco_command(
        &["changelog", "--no-wrap"],
        Some(repo),
        true,
        "changelog_no_wrap_retains_long_lines",
    )?;

    Ok(())
}

#[test]
fn custom_unreleased_title() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["feat: initial conventional commit"])?;
    let repo = temp.path();

    run_convco_command(
        &["changelog", "--unreleased", "Upcoming"],
        Some(repo),
        true,
        "changelog_custom_unreleased_title",
    )?;

    Ok(())
}

#[test]
fn merges_with_first_parent_skips_branch_commits() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_merge_commit("feat: merge feature", "feat: branch change")?;
    let repo = temp.path();

    run_convco_command(
        &["changelog", "--merges"],
        Some(repo),
        true,
        "changelog_merges_includes_merge_commits",
    )?;

    run_convco_command(
        &["changelog", "--merges", "--first-parent"],
        Some(repo),
        true,
        "changelog_merges_with_first_parent_skips_branch_commits",
    )?;

    Ok(())
}
