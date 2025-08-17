use super::super::{run_convco_command, *};

#[test]
fn succeeds_on_conventional_commits() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["feat: initial conventional commit"])?;
    let repo = temp.path();

    run_convco_command(
        &["check"],
        Some(repo),
        true,
        "check_succeeds_on_conventional_commits",
    )?;

    Ok(())
}

#[test]
fn fails_on_non_conventional_commits() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["this is not conventional"])?;
    let repo = temp.path();

    run_convco_command(
        &["check"],
        Some(repo),
        false,
        "check_fails_on_non_conventional_commits",
    )?;

    Ok(())
}

#[test]
fn range_checks_commits_from_low_to_high() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["feat: base", "this is not conventional"])?;
    let repo = temp.path();

    let base = String::from_utf8(
        StdCommand::new("git")
            .args(["rev-parse", "HEAD~1"])
            .current_dir(repo)
            .output()?
            .stdout,
    )?;
    let head = String::from_utf8(
        StdCommand::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo)
            .output()?
            .stdout,
    )?;
    let range = format!("{}..{}", base.trim(), head.trim());

    run_convco_command(
        &["check", &range],
        Some(repo),
        false,
        "check_range_checks_commits_from_low_to_high",
    )?;

    Ok(())
}

#[test]
fn unsupported_type_reports_full_subject() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["wip: still deciding"])?;
    let repo = temp.path();

    run_convco_command(
        &["check"],
        Some(repo),
        false,
        "check_unsupported_type_reports_full_subject",
    )?;

    Ok(())
}

#[test]
fn respects_max_count_flag() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["feat: one", "feat: two", "fix: three", "chore: four"])?;
    let repo = temp.path();

    run_convco_command(
        &["check", "--max-count", "2"],
        Some(repo),
        true,
        "check_respects_max_count_flag",
    )?;

    Ok(())
}

#[test]
fn fails_on_non_conventional_merge_when_flag_is_set() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_merge_commit("Merge branch 'feature'", "feat: feature work")?;
    let repo = temp.path();

    run_convco_command(&["check"], Some(repo), true, "")?;

    run_convco_command(
        &["check", "--merges"],
        Some(repo),
        false,
        "check_fails_on_non_conventional_merge_when_flag_is_set",
    )?;

    Ok(())
}

#[test]
fn includes_merge_commits_when_flag_is_set() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_merge_commit("feat: merge feature", "feat: feature work")?;
    let repo = temp.path();

    run_convco_command(
        &["check", "--merges"],
        Some(repo),
        true,
        "check_includes_merge_commits_when_flag_is_set",
    )?;

    Ok(())
}

#[test]
fn first_parent_skips_non_conventional_branch_commits() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_merge_commit("feat: merge feature", "this is not conventional")?;
    let repo = temp.path();

    run_convco_command(
        &["check", "--merges"],
        Some(repo),
        false,
        "check_fails_on_non_conventional_branch_commits",
    )?;

    run_convco_command(
        &["check", "--merges", "--first-parent"],
        Some(repo),
        true,
        "check_merges_with_first_parent_skips_non_conventional_branch",
    )?;

    Ok(())
}

#[test]
fn ignore_reverts_skips_revert_commits() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_revert_commit()?;
    let repo = temp.path();

    run_convco_command(
        &["check"],
        Some(repo),
        false,
        "check_fails_on_revert_commits_without_flag",
    )?;

    run_convco_command(
        &["check", "--ignore-reverts"],
        Some(repo),
        true,
        "check_ignores_revert_commits_with_flag",
    )?;

    Ok(())
}

#[test]
fn ignore_message_pattern_flag_skips_matching_commits() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["this is not conventional"])?;
    let repo = temp.path();

    run_convco_command(
        &[
            "check",
            "--ignore-message-pattern",
            "this is not conventional",
        ],
        Some(repo),
        true,
        "check_ignore_message_pattern_flag_skips_matching_commits",
    )?;

    Ok(())
}

#[test]
fn ignore_message_pattern_config_skips_matching_commits() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = setup_repo_with_commits(&["this is not conventional"])?;
    let repo = temp.path();
    std::fs::write(
        repo.join(".convco"),
        "ignoreMessagePattern:\n  - 'this is not conventional'\n",
    )?;

    run_convco_command(
        &["check"],
        Some(repo),
        true,
        "check_ignore_message_pattern_config_skips_matching_commits",
    )?;

    Ok(())
}

#[test]
fn ignore_message_pattern_flag_overrides_config() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["feat: valid", "this is not conventional"])?;
    let repo = temp.path();
    std::fs::write(
        repo.join(".convco"),
        "ignoreMessagePattern:\n  - '^feat:'\n",
    )?;

    run_convco_command(
        &[
            "check",
            "--ignore-message-pattern",
            "this is not conventional",
        ],
        Some(repo),
        true,
        "check_ignore_message_pattern_flag_overrides_config",
    )?;

    Ok(())
}

#[test]
fn ignore_message_pattern_invalid_regex_fails() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["feat: valid"])?;
    let repo = temp.path();

    run_convco_command(
        &["check", "--ignore-message-pattern", "["],
        Some(repo),
        false,
        "check_ignore_message_pattern_invalid_regex_fails",
    )?;

    Ok(())
}

#[test]
fn strip_removes_comments_from_stdin() -> Result<(), Box<dyn std::error::Error>> {
    let message = "# comment\nfeat: valid change\n";

    // The following tests use stdin and need to be handled specially
    // as they can't use the helper function due to stdin handling
    let mut without_strip = Command::cargo_bin("convco")?;
    without_strip
        .arg("check")
        .arg("--from-stdin")
        .write_stdin(message);
    let assert = without_strip.assert().failure();
    let output = assert.get_output();
    let stdout = std::str::from_utf8(&output.stdout)?;
    let stderr = std::str::from_utf8(&output.stderr)?;
    let snapshot = format!("stdout:\n{stdout}---\nstderr:\n{stderr}");
    assert_snapshot!("check_without_strip_fails_on_comment_first_line", snapshot);

    let mut with_strip = Command::cargo_bin("convco")?;
    with_strip
        .args(["check", "--from-stdin", "--strip"])
        .write_stdin(message);
    let assert = with_strip.assert().success();
    let output = assert.get_output();
    let stdout = std::str::from_utf8(&output.stdout)?;
    let stderr = std::str::from_utf8(&output.stderr)?;
    let snapshot = format!("stdout:\n{stdout}---\nstderr:\n{stderr}");
    assert_snapshot!("check_with_strip_strips_comment_lines", snapshot);

    Ok(())
}
