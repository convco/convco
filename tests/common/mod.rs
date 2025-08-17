use std::{path::Path, process::Command};

use tempfile::TempDir;

pub struct TestRepo {
    pub dir: TempDir,
}

impl TestRepo {
    pub fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        init_git_repo(path);
        Self { dir }
    }

    pub fn commit(&self, message: &str) {
        commit(self.dir.path(), message);
    }

    pub fn branch(&self, name: &str) {
        branch(self.dir.path(), name);
    }

    pub fn checkout(&self, name: &str) {
        checkout(self.dir.path(), name);
    }

    pub fn merge(&self, name: &str) {
        merge(self.dir.path(), name);
    }

    pub fn revert(&self) {
        revert(self.dir.path());
    }

    pub fn write_convco_config(&self, content: &str) {
        std::fs::write(self.dir.path().join(".convco.toml"), content).unwrap();
    }

    pub fn write_file(&self, name: &str, content: &str) {
        std::fs::write(self.dir.path().join(name), content).unwrap();
    }

    pub fn add(&self, name: &str) {
        let output = Command::new("git")
            .arg("add")
            .arg(name)
            .current_dir(self.dir.path())
            .output()
            .unwrap();
        assert!(output.status.success());
    }
}

fn init_git_repo(path: &Path) {
    let output = Command::new("git")
        .arg("init")
        .arg("--initial-branch=main")
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git init failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new("git")
        .args(["config", "user.name", "Convco Test"])
        .current_dir(path)
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()
        .unwrap();
    assert!(output.status.success());
}

fn commit(path: &Path, message: &str) {
    let output = Command::new("git")
        .arg("commit")
        .arg("--allow-empty")
        .arg("-m")
        .arg(message)
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "commit failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn branch(path: &Path, name: &str) {
    let output = Command::new("git")
        .arg("branch")
        .arg(name)
        .current_dir(path)
        .output()
        .unwrap();
    assert!(output.status.success());
}

fn checkout(path: &Path, name: &str) {
    let output = Command::new("git")
        .arg("checkout")
        .arg(name)
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "checkout failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn merge(path: &Path, name: &str) {
    let output = Command::new("git")
        .arg("merge")
        .arg(name)
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "merge failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn revert(path: &Path) {
    let output = Command::new("git")
        .arg("revert")
        .arg("--no-edit")
        .arg("HEAD")
        .current_dir(path)
        .output()
        .unwrap();
    // print ls output
    let ls = Command::new("ls").current_dir(path).output().unwrap();
    println!("{}", String::from_utf8_lossy(&ls.stdout));
    eprintln!("{}", String::from_utf8_lossy(&ls.stderr));
    assert!(
        output.status.success(),
        "revert failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
