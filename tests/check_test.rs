mod common;

use assert_cmd::Command;
use common::TestRepo;

#[test]
fn test_check_success() {
    let repo = TestRepo::new();
    repo.commit("feat: Add first file");

    let mut cmd = Command::cargo_bin("convco").unwrap();
    cmd.current_dir(repo.dir.path())
        .arg("check")
        .assert()
        .success();
}

#[test]
fn test_check_revert_commit() {
    let repo = TestRepo::new();
    repo.write_convco_config(
        r#"[[types]]
type = "revert"
section = "Reverts"
hidden = false"#,
    );
    repo.write_file("file.txt", "content");
    repo.add("file.txt");
    repo.commit("feat: Add first file");
    repo.revert();

    let mut cmd = Command::cargo_bin("convco").unwrap();
    cmd.current_dir(repo.dir.path())
        .arg("check")
        .arg("--ignore-reverts")
        .assert()
        .success();
}

#[test]
fn test_check_merge_commit() {
    let repo = TestRepo::new();
    // Create a commit on main
    repo.commit("feat: on main");

    // Create a branch, and a commit on it
    repo.branch("other");
    repo.checkout("other");
    repo.commit("feat: on other");

    // Merge back to main
    repo.checkout("main");
    repo.merge("other");

    let mut cmd = Command::cargo_bin("convco").unwrap();
    cmd.current_dir(repo.dir.path())
        .arg("check")
        .assert()
        .success();
}
