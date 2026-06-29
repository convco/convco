#![cfg(feature = "integration-tests")]

use std::{
    env, fs,
    panic::{self, AssertUnwindSafe},
    path::Path,
    process::Command,
    sync::{Mutex, OnceLock},
};

use convco::{open_repo, CommitParser, CommitTrait, Repo, RevWalkOptions};
use tempfile::{tempdir, TempDir};

fn cwd_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "git {} failed: {}{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn setup_repo() -> TempDir {
    let temp = tempdir().unwrap();
    let repo = temp.path();

    git(repo, &["init"]);
    git(repo, &["config", "user.name", "Convco Test"]);
    git(repo, &["config", "user.email", "test@example.com"]);

    temp
}

fn setup_sha256_repo() -> TempDir {
    let temp = tempdir().unwrap();
    let repo = temp.path();

    git(repo, &["init", "--object-format=sha256"]);
    git(repo, &["config", "user.name", "Convco Test"]);
    git(repo, &["config", "user.email", "test@example.com"]);

    temp
}

fn with_repo<T>(repo: &Path, test: impl FnOnce() -> T) -> T {
    let _guard = cwd_lock().lock().unwrap_or_else(|err| err.into_inner());
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(repo).unwrap();
    let result = panic::catch_unwind(AssertUnwindSafe(test));
    env::set_current_dir(original_dir).unwrap();
    match result {
        Ok(result) => result,
        Err(err) => panic::resume_unwind(err),
    }
}

#[test]
fn open_repo_discovers_repository_from_nested_directory() {
    let temp = setup_repo();
    let nested = temp.path().join("nested").join("directory");
    fs::create_dir_all(&nested).unwrap();

    with_repo(&nested, || {
        let repo = open_repo().unwrap();
        let _ = Repo::url(&repo, "origin").unwrap();
    });
}

#[test]
fn sha256_repositories_support_core_backend_operations() {
    let temp = setup_sha256_repo();
    let repo = temp.path();

    fs::create_dir_all(repo.join("packages/app")).unwrap();
    fs::write(repo.join("packages/app/app.txt"), "app").unwrap();
    git(repo, &["add", "packages/app/app.txt"]);
    git(repo, &["commit", "-m", "feat(app): app change"]);
    git(repo, &["tag", "v1.0.0"]);

    fs::create_dir_all(repo.join("packages/lib")).unwrap();
    fs::write(repo.join("packages/lib/lib.txt"), "lib").unwrap();
    git(repo, &["add", "packages/lib/lib.txt"]);
    git(
        repo,
        &["commit", "-m", "feat(lib): lib change", "-m", "body line"],
    );

    with_repo(repo, || {
        let repo = open_repo().unwrap();
        let head = Repo::revparse_single(&repo, "HEAD").unwrap();
        assert_eq!(CommitTrait::id(&head).len(), 64);
        assert_eq!(
            head.commit_message().unwrap().as_ref(),
            "feat(lib): lib change\n\nbody line\n"
        );

        let semvers = Repo::semver_tags(&repo, "v").unwrap();
        assert_eq!(semvers.len(), 1);
        assert_eq!(semvers[0].0.to_string(), "1.0.0");

        let version = Repo::find_last_version(&repo, &head, false, &semvers).unwrap();
        assert_eq!(
            version.map(|(version, _)| version.to_string()),
            Some("1.0.0".to_owned())
        );

        let parser = CommitParser::builder().build();
        let commits = Repo::revwalk(
            &repo,
            RevWalkOptions {
                from_rev: vec![],
                to_rev: head,
                first_parent: false,
                no_merge_commits: false,
                no_revert_commits: false,
                paths: vec!["packages/app".to_owned()],
                parser: &parser,
            },
        )
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        let messages = commits
            .iter()
            .map(|commit| commit.commit.commit_message().unwrap().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(messages, ["feat(app): app change\n"]);
    });
}

#[test]
fn remote_url_returns_none_when_origin_is_missing() {
    let temp = setup_repo();

    with_repo(temp.path(), || {
        let repo = open_repo().unwrap();
        assert_eq!(Repo::url(&repo, "origin").unwrap(), None);
    });
}

#[test]
fn remote_url_supports_https_and_ssh_origins() {
    let temp = setup_repo();

    with_repo(temp.path(), || {
        git(
            temp.path(),
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/convco/convco.git",
            ],
        );
        let repo = open_repo().unwrap();
        assert_eq!(
            Repo::url(&repo, "origin").unwrap().as_deref(),
            Some("https://github.com/convco/convco.git")
        );
        drop(repo);

        git(
            temp.path(),
            &[
                "remote",
                "set-url",
                "origin",
                "git@github.com:convco/convco.git",
            ],
        );
        let repo = open_repo().unwrap();
        assert_eq!(
            Repo::url(&repo, "origin").unwrap().as_deref(),
            Some("git@github.com:convco/convco.git")
        );
    });
}

#[test]
fn commit_message_returns_full_multiline_message() {
    let temp = setup_repo();

    git(
        temp.path(),
        &[
            "commit",
            "--allow-empty",
            "-m",
            "feat: subject",
            "-m",
            "body line",
            "-m",
            "Refs: #1",
        ],
    );

    with_repo(temp.path(), || {
        let repo = open_repo().unwrap();
        let commit = Repo::revparse_single(&repo, "HEAD").unwrap();
        assert_eq!(
            commit.commit_message().unwrap().as_ref(),
            "feat: subject\n\nbody line\n\nRefs: #1\n"
        );
    });
}

#[test]
fn semver_tags_are_sorted_and_resolved_to_commits() {
    let temp = setup_repo();

    git(temp.path(), &["commit", "--allow-empty", "-m", "feat: one"]);
    git(temp.path(), &["tag", "v1.0.0"]);
    git(temp.path(), &["commit", "--allow-empty", "-m", "feat: two"]);
    git(temp.path(), &["tag", "v2.0.0"]);
    git(temp.path(), &["tag", "not-a-version"]);

    with_repo(temp.path(), || {
        let repo = open_repo().unwrap();
        let versions = Repo::semver_tags(&repo, "v").unwrap();
        let version_numbers = versions
            .iter()
            .map(|(version, _)| version.to_string())
            .collect::<Vec<_>>();

        assert_eq!(version_numbers, ["2.0.0", "1.0.0"]);
        assert_eq!(
            versions[0].1.commit_message().unwrap().as_ref(),
            "feat: two\n"
        );
    });
}

#[test]
fn semver_tags_ignore_tags_that_do_not_resolve_to_commits() {
    let temp = setup_repo();
    let repo = temp.path();

    fs::write(repo.join("file.txt"), "base").unwrap();
    git(repo, &["add", "file.txt"]);
    git(repo, &["commit", "-m", "feat: base"]);
    git(repo, &["tag", "v1.0.0"]);

    let output = Command::new("git")
        .args(["hash-object", "-w", "file.txt"])
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git hash-object failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let blob = String::from_utf8(output.stdout).unwrap();
    git(repo, &["tag", "v9.9.9", blob.trim()]);

    with_repo(repo, || {
        let repo = open_repo().unwrap();
        let versions = Repo::semver_tags(&repo, "v").unwrap();
        let version_numbers = versions
            .iter()
            .map(|(version, _)| version.to_string())
            .collect::<Vec<_>>();

        assert_eq!(version_numbers, ["1.0.0"]);
    });
}

#[test]
fn find_last_version_selects_highest_reachable_semver_in_non_linear_history() {
    let temp = setup_repo();
    let repo = temp.path();

    git(repo, &["commit", "--allow-empty", "-m", "feat: base"]);
    git(repo, &["checkout", "-b", "release-one"]);
    git(repo, &["commit", "--allow-empty", "-m", "feat: one"]);
    git(repo, &["tag", "v1.0.0"]);
    git(repo, &["checkout", "-"]);
    git(repo, &["checkout", "-b", "release-two"]);
    git(repo, &["commit", "--allow-empty", "-m", "feat: two"]);
    git(repo, &["tag", "v2.0.0"]);
    git(repo, &["checkout", "release-one"]);
    git(
        repo,
        &[
            "merge",
            "--no-ff",
            "release-two",
            "-m",
            "feat: merge releases",
        ],
    );

    with_repo(repo, || {
        let repo = open_repo().unwrap();
        let head = Repo::revparse_single(&repo, "HEAD").unwrap();
        let semvers = Repo::semver_tags(&repo, "v").unwrap();
        let version = Repo::find_last_version(&repo, &head, false, &semvers).unwrap();

        assert_eq!(
            version.map(|(version, _)| version.to_string()),
            Some("2.0.0".to_owned())
        );
    });
}

#[test]
fn revparse_single_resolves_head_and_revwalk_filters_paths() {
    let temp = setup_repo();
    let repo = temp.path();

    fs::create_dir_all(repo.join("packages/app")).unwrap();
    fs::write(repo.join("packages/app/app.txt"), "app").unwrap();
    git(repo, &["add", "packages/app/app.txt"]);
    git(repo, &["commit", "-m", "feat(app): app change"]);

    fs::create_dir_all(repo.join("packages/lib")).unwrap();
    fs::write(repo.join("packages/lib/lib.txt"), "lib").unwrap();
    git(repo, &["add", "packages/lib/lib.txt"]);
    git(repo, &["commit", "-m", "feat(lib): lib change"]);

    with_repo(repo, || {
        let repo = open_repo().unwrap();
        let head = Repo::revparse_single(&repo, "HEAD").unwrap();
        assert_eq!(
            head.commit_message().unwrap().as_ref(),
            "feat(lib): lib change\n"
        );

        let parser = CommitParser::builder().build();
        let commits = Repo::revwalk(
            &repo,
            RevWalkOptions {
                from_rev: vec![],
                to_rev: head,
                first_parent: false,
                no_merge_commits: false,
                no_revert_commits: false,
                paths: vec!["packages/app".to_owned()],
                parser: &parser,
            },
        )
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        let messages = commits
            .iter()
            .map(|commit| commit.commit.commit_message().unwrap().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(messages, ["feat(app): app change\n"]);
    });
}

fn setup_repo_with_pathspec_commits() -> TempDir {
    let temp = setup_repo();
    let repo = temp.path();

    fs::create_dir_all(repo.join("src/generated")).unwrap();
    fs::create_dir_all(repo.join("charts")).unwrap();

    fs::write(repo.join("src/app.txt"), "base").unwrap();
    git(repo, &["add", "src/app.txt"]);
    git(repo, &["commit", "-m", "feat: base"]);
    git(repo, &["tag", "v1.0.0"]);

    fs::write(repo.join("charts/chart.txt"), "chart").unwrap();
    git(repo, &["add", "charts/chart.txt"]);
    git(repo, &["commit", "-m", "feat: chart only"]);

    fs::write(repo.join("src/app.txt"), "source fix").unwrap();
    git(repo, &["add", "src/app.txt"]);
    git(repo, &["commit", "-m", "fix: source only"]);

    fs::write(repo.join("src/generated/schema.txt"), "generated").unwrap();
    git(repo, &["add", "src/generated/schema.txt"]);
    git(repo, &["commit", "-m", "feat: generated only"]);

    fs::write(repo.join("charts/chart.txt"), "chart mixed").unwrap();
    fs::write(repo.join("src/mixed.txt"), "source mixed").unwrap();
    git(repo, &["add", "charts/chart.txt", "src/mixed.txt"]);
    git(repo, &["commit", "-m", "feat: mixed source and chart"]);

    temp
}

fn revwalk_messages(repo: &Path, paths: Vec<String>) -> Vec<String> {
    with_repo(repo, || {
        let repo = open_repo().unwrap();
        let head = Repo::revparse_single(&repo, "HEAD").unwrap();
        let parser = CommitParser::builder().build();
        let commits = Repo::revwalk(
            &repo,
            RevWalkOptions {
                from_rev: vec![],
                to_rev: head,
                first_parent: false,
                no_merge_commits: false,
                no_revert_commits: false,
                paths,
                parser: &parser,
            },
        )
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

        commits
            .iter()
            .map(|commit| commit.commit.commit_message().unwrap().into_owned())
            .collect::<Vec<_>>()
    })
}

#[test]
fn revwalk_supports_exclude_only_pathspecs() {
    let temp = setup_repo_with_pathspec_commits();
    let messages = revwalk_messages(temp.path(), vec![":(exclude)charts".to_owned()]);

    assert_eq!(
        messages,
        [
            "feat: mixed source and chart\n",
            "feat: generated only\n",
            "fix: source only\n",
            "feat: base\n",
        ]
    );
}

#[test]
fn revwalk_supports_shorthand_exclude_pathspecs() {
    let temp = setup_repo_with_pathspec_commits();
    let messages = revwalk_messages(temp.path(), vec![":!charts".to_owned()]);

    assert_eq!(
        messages,
        [
            "feat: mixed source and chart\n",
            "feat: generated only\n",
            "fix: source only\n",
            "feat: base\n",
        ]
    );
}

#[test]
fn revwalk_supports_include_and_exclude_pathspecs() {
    let temp = setup_repo_with_pathspec_commits();
    let messages = revwalk_messages(
        temp.path(),
        vec!["src".to_owned(), ":(exclude)src/generated".to_owned()],
    );

    assert_eq!(
        messages,
        [
            "feat: mixed source and chart\n",
            "fix: source only\n",
            "feat: base\n",
        ]
    );
}

#[test]
fn revwalk_without_paths_works_in_bare_repository() {
    let temp = setup_repo();
    let repo = temp.path();
    git(
        repo,
        &["commit", "--allow-empty", "-m", "feat: bare commit"],
    );

    let bare = temp.path().join("bare.git");
    let output = Command::new("git")
        .args([
            "clone",
            "--bare",
            repo.to_str().unwrap(),
            bare.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git clone --bare failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    with_repo(&bare, || {
        let repo = open_repo().unwrap();
        let head = Repo::revparse_single(&repo, "HEAD").unwrap();
        let parser = CommitParser::builder().build();
        let commits = Repo::revwalk(
            &repo,
            RevWalkOptions {
                from_rev: vec![],
                to_rev: head,
                first_parent: false,
                no_merge_commits: false,
                no_revert_commits: false,
                paths: vec![],
                parser: &parser,
            },
        )
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

        assert_eq!(commits.len(), 1);
    });
}
