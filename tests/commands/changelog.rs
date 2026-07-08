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
fn groups_default_types_case_insensitively() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["FEAT: uppercase feature"])?;
    let repo = temp.path();

    let output = run_convco_command(&["changelog", "--no-links"], Some(repo), true, "")?;

    assert!(output.contains("### Features"), "got:\n{output}");
    assert!(output.contains("uppercase feature"), "got:\n{output}");

    Ok(())
}

#[test]
fn preserves_scope_casing_in_output() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["feat(Parser): scoped feature"])?;
    let repo = temp.path();

    let output = run_convco_command(&["changelog", "--no-links"], Some(repo), true, "")?;

    assert!(
        output.contains("**Parser:** scoped feature"),
        "got:\n{output}"
    );

    Ok(())
}

#[test]
fn groups_custom_types_case_insensitively() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["CUSTOM: uppercase custom"])?;
    let repo = temp.path();
    fs::write(
        repo.join(".convco"),
        r#"types:
- type: custom
  section: Custom
  hidden: false
"#,
    )?;

    let output = run_convco_command(&["changelog", "--no-links"], Some(repo), true, "")?;

    assert!(output.contains("### Custom"), "got:\n{output}");
    assert!(output.contains("uppercase custom"), "got:\n{output}");

    Ok(())
}

#[test]
fn hidden_types_are_filtered_case_insensitively() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["DOCS: uppercase docs"])?;
    let repo = temp.path();

    let output = run_convco_command(&["changelog", "--no-links"], Some(repo), true, "")?;
    assert!(!output.contains("uppercase docs"), "got:\n{output}");

    let output = run_convco_command(
        &["changelog", "--no-links", "--include-hidden-sections"],
        Some(repo),
        true,
        "",
    )?;
    assert!(output.contains("uppercase docs"), "got:\n{output}");

    Ok(())
}

#[test]
fn non_linear_history_uses_highest_reachable_semver() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_non_linear_version_tags()?;
    let repo = temp.path();

    let output = run_convco_command(&["changelog", "--no-links"], Some(repo), true, "")?;

    assert!(
        output.contains("## v2.0.0"),
        "expected v2.0.0 section, got:\n{output}"
    );
    assert!(
        output.contains("## v1.0.0"),
        "expected v1.0.0 section, got:\n{output}"
    );

    Ok(())
}

#[test]
fn links_references_with_remote_derived_repository_metadata(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let repo = temp.path();

    git(repo, &["init"])?;
    git(repo, &["config", "user.name", "Convco Test"])?;
    git(repo, &["config", "user.email", "test@example.com"])?;
    git(
        repo,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/acme/demo.git",
        ],
    )?;
    git(
        repo,
        &["commit", "--allow-empty", "-m", "feat: base closes #123"],
    )?;

    run_convco_command(
        &["changelog"],
        Some(repo),
        true,
        "changelog_links_references_with_remote_derived_repository_metadata",
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
fn pathspec_excludes_filter_changelog_commits() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let repo = temp.path();

    git(repo, &["init"])?;
    git(repo, &["config", "user.name", "Convco Test"])?;
    git(repo, &["config", "user.email", "test@example.com"])?;

    fs::create_dir_all(repo.join("src/generated"))?;
    fs::create_dir_all(repo.join("charts"))?;

    fs::write(repo.join("src/app.txt"), "base")?;
    git(repo, &["add", "src/app.txt"])?;
    git(repo, &["commit", "-m", "feat: base"])?;
    git(repo, &["tag", "v1.0.0"])?;

    fs::write(repo.join("charts/chart.txt"), "chart")?;
    git(repo, &["add", "charts/chart.txt"])?;
    git(repo, &["commit", "-m", "feat: chart only"])?;

    fs::write(repo.join("src/generated/schema.txt"), "generated")?;
    git(repo, &["add", "src/generated/schema.txt"])?;
    git(repo, &["commit", "-m", "feat: generated only"])?;

    fs::write(repo.join("src/app.txt"), "source fix")?;
    git(repo, &["add", "src/app.txt"])?;
    git(repo, &["commit", "-m", "fix: source only"])?;

    let output = run_convco_command(
        &[
            "changelog",
            "--no-links",
            "--paths",
            "src,:(exclude)src/generated",
            "--skip-empty",
        ],
        Some(repo),
        true,
        "",
    )?;

    assert!(output.contains("source only"), "got:\n{output}");
    assert!(!output.contains("generated only"), "got:\n{output}");
    assert!(!output.contains("chart only"), "got:\n{output}");

    let output = run_convco_command(
        &[
            "changelog",
            "--no-links",
            "--paths",
            "src",
            "--paths",
            ":(exclude)src/generated",
            "--skip-empty",
        ],
        Some(repo),
        true,
        "",
    )?;

    assert!(output.contains("source only"), "got:\n{output}");
    assert!(!output.contains("generated only"), "got:\n{output}");
    assert!(!output.contains("chart only"), "got:\n{output}");

    Ok(())
}

#[test]
fn paths_filter_merge_commits_against_first_parent() -> Result<(), Box<dyn std::error::Error>> {
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

    run_convco_command(
        &[
            "changelog",
            "--no-links",
            "--merges",
            "--paths",
            "lib",
            "--skip-empty",
        ],
        Some(repo),
        true,
        "changelog_paths_filter_merge_commits_against_first_parent",
    )?;

    Ok(())
}

#[test]
fn explicit_branch_revision_is_used_as_section_title() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["feat: base"])?;
    let repo = temp.path();

    git(repo, &["tag", "v1.0.0"])?;
    git(repo, &["checkout", "-b", "release-next"])?;
    git(
        repo,
        &["commit", "--allow-empty", "-m", "fix: branch patch"],
    )?;

    run_convco_command(
        &["changelog", "--no-links", "release-next"],
        Some(repo),
        true,
        "changelog_explicit_branch_revision_is_used_as_section_title",
    )?;

    Ok(())
}

#[test]
fn explicit_sha_range_uses_upper_sha_as_section_title() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let repo = temp.path();

    git(repo, &["init"])?;
    git(repo, &["config", "user.name", "Convco Test"])?;
    git(repo, &["config", "user.email", "test@example.com"])?;
    git(
        repo,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/acme/demo.git",
        ],
    )?;
    git(repo, &["commit", "--allow-empty", "-m", "feat: base"])?;
    git(repo, &["tag", "v1.0.0"])?;
    git(repo, &["commit", "--allow-empty", "-m", "fix: first"])?;
    let low_sha = String::from_utf8(
        StdCommand::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo)
            .output()?
            .stdout,
    )?;
    git(repo, &["commit", "--allow-empty", "-m", "fix: second"])?;
    let high_sha = String::from_utf8(
        StdCommand::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo)
            .output()?
            .stdout,
    )?;
    let range = format!("{}..{}", low_sha.trim(), high_sha.trim());

    run_convco_command(
        &["changelog", &range],
        Some(repo),
        true,
        "changelog_explicit_sha_range_uses_upper_sha_as_section_title",
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
fn heading_levels_use_current_version_patch_status() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_commits(&["fix: patch release"])?;
    let repo = temp.path();
    git(repo, &["tag", "v1.0.1"])?;
    git(
        repo,
        &["commit", "--allow-empty", "-m", "feat: major release"],
    )?;
    git(repo, &["tag", "v2.0.0"])?;
    git(
        repo,
        &["commit", "--allow-empty", "-m", "feat: unreleased feature"],
    )?;

    run_convco_command(
        &["changelog", "--no-links"],
        Some(repo),
        true,
        "changelog_heading_levels_use_current_version_patch_status",
    )?;

    run_convco_command(
        &["changelog", "--no-links", "--unreleased", "2.0.1"],
        Some(repo),
        true,
        "changelog_unreleased_semver_patch_uses_patch_heading",
    )?;

    Ok(())
}

#[test]
fn max_versions_limits_rendered_sections() -> Result<(), Box<dyn std::error::Error>> {
    let temp = setup_repo_with_version_tags()?;
    let repo = temp.path();

    run_convco_command(
        &["changelog", "--no-links", "--max-versions", "1"],
        Some(repo),
        true,
        "changelog_max_versions_one_with_unreleased",
    )?;

    git(repo, &["tag", "v4.0.0"])?;

    run_convco_command(
        &["changelog", "--no-links", "--max-versions", "1"],
        Some(repo),
        true,
        "changelog_max_versions_one_without_unreleased",
    )?;

    Ok(())
}

#[test]
fn annotated_tag_date_uses_tagger_date() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let repo = temp.path();

    git(repo, &["init"])?;
    git(repo, &["config", "user.name", "Convco Test"])?;
    git(repo, &["config", "user.email", "test@example.com"])?;
    let output = StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "feat: initial"])
        .env("GIT_AUTHOR_DATE", "2020-01-01T00:00:00+0000")
        .env("GIT_COMMITTER_DATE", "2020-01-01T00:00:00+0000")
        .current_dir(repo)
        .output()?;
    assert!(output.status.success());
    let output = StdCommand::new("git")
        .args(["tag", "-a", "v1.0.0", "-m", "release v1.0.0"])
        .env("GIT_COMMITTER_DATE", "2021-02-03T00:00:00+0000")
        .current_dir(repo)
        .output()?;
    assert!(output.status.success());

    let mut cmd = Command::cargo_bin("convco")?;
    let assert = cmd
        .current_dir(repo)
        .args(["changelog", "--no-links"])
        .assert()
        .success();
    let stdout = std::str::from_utf8(&assert.get_output().stdout)?;

    assert!(stdout.contains("## v1.0.0 (2021-02-03)"));

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
