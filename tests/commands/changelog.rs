use std::fs;

use super::super::{run_convco_command, *};

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
