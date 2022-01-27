use std::{fs, iter::FromIterator};

use clap::IntoApp;
use clap_complete::{generate_to, Shell};
include!("src/cli.rs");

fn main() {
    let app = &mut Opt::into_app();
    let out_dir = &["target", "completions"];
    let out_dir: PathBuf = PathBuf::from_iter(out_dir.iter());
    let out_dir = out_dir.as_path();

    fs::create_dir_all(out_dir).unwrap();

    // Generate completions for all shells available in `clap`.
    generate_to(Shell::Bash, app, "convco", out_dir).unwrap();
    generate_to(Shell::Fish, app, "convco", out_dir).unwrap();
    generate_to(Shell::Zsh, app, "convco", out_dir).unwrap();
    generate_to(Shell::Elvish, app, "convco", out_dir).unwrap();
    generate_to(Shell::PowerShell, app, "convco", out_dir).unwrap();
}
