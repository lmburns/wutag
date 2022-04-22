mod color;
mod list;
mod rm;
mod set;

mod clean_cache;
mod clear;
mod cp;
mod edit;
mod print_completions;
mod search;
mod view;

use assert_cmd::cargo::CommandCargoExt;
// use assert_cmd::prelude::*;
use once_cell::sync::Lazy;
use predicates::{prelude::predicate, str::PredicateStrExt};
use rand::{distributions::Alphanumeric, Rng};
use serial_test::serial;
use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::from_utf8,
    sync::Once,
};
use tempfile::tempdir;

static INIT: Once = Once::new();

const FILE_DIR: &str = "tests/example_files";
const ANOTHER_FILE_DIR: &str = "tests/sample_dir";
const NEW_REGISTRY: &str = "tests/sample.reg";

static CWD: Lazy<PathBuf> = Lazy::new(|| {
    env::current_dir()
        .expect("unable to get CWD")
        .join(FILE_DIR)
});
static CWD_TWO: Lazy<PathBuf> = Lazy::new(|| {
    env::current_dir()
        .expect("unable to get CWD")
        .join(ANOTHER_FILE_DIR)
});

#[macro_export]
macro_rules! tag_out {
    ($file:literal => $($set:tt),* $(,)?) => {
        format!("{}/{}:\n\t{}\n", CWD.display(), $file, format!($($set),*))
    }
}

#[macro_export]
macro_rules! expand_file {
    ($file:literal) => {
        format!("{}/{}", CWD.display(), $file)
    };
}

#[macro_export]
macro_rules! expand_file_dir_two {
    ($file:literal) => {
        format!("{}/{}", CWD_TWO.display(), $file)
    };
}

pub fn create_temp_path() -> String {
    let mut tmp_path = env::temp_dir();
    tmp_path.push(
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(12)
            .map(char::from)
            .collect::<String>(),
    );
    tmp_path.display().to_string()
}

fn wutag_cmd_base_registry() -> Command {
    let mut cmd = Command::cargo_bin("wutag").unwrap();
    cmd.current_dir(FILE_DIR);
    cmd.env_remove("WUTAG_REGISTRY");
    cmd
}

// BASE REGISTRY
pub fn wutag_cmd_new_registry() -> Command {
    let mut cmd = wutag_cmd_base_registry();
    cmd.args(["--registry", NEW_REGISTRY, "--color=never"]);
    cmd
}

// BASE REGISTRY - GLOBAL
pub fn wutag_cmd_new_registry_global() -> Command {
    let mut cmd = wutag_cmd_base_registry();
    cmd.args(["--registry", NEW_REGISTRY, "--global", "--color=never"]);
    cmd
}

// RANDOM REGISTRY
pub fn wutag_cmd_random_registry() -> Command {
    let mut cmd = wutag_cmd_base_registry();
    cmd.args(["--registry", create_temp_path().as_str(), "--color=never"]);
    cmd
}

pub fn wutag() -> assert_cmd::Command {
    assert_cmd::Command::from_std(wutag_cmd_new_registry())
}

pub fn wutag_global() -> assert_cmd::Command {
    assert_cmd::Command::from_std(wutag_cmd_new_registry_global())
}

pub fn wutag_rr() -> assert_cmd::Command {
    assert_cmd::Command::from_std(wutag_cmd_random_registry())
}

fn rm_registry() {
    INIT.call_once(|| {
        match assert_cmd::Command::new("rm")
            .arg("-rf")
            .arg(NEW_REGISTRY)
            .ok()
        {
            Ok(_) => {},
            Err(err) => {
                eprintln!("Error removing registry {:?}", err);
            },
        };
    });
}

// ============================== SUBCMDS ===============================
// ======================================================================

// CLEAR
pub fn wutag_clear() {
    wutag_cmd_new_registry()
        .args(&["-m", "5", "clear", "*"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .expect("error removing tag");
}

pub fn wutag_clear_global() {
    wutag_cmd_new_registry_global()
        .args(&["-g", "clear", "*"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .expect("error removing tags globally");
}

// CLEAN
pub fn wutag_clean() {
    wutag_cmd_new_registry()
        .args(&["clean-cache"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .expect("error clearing out the registry");
}

// SET
pub fn wutag_set(pat: &str, tag: &str) {
    wutag_cmd_new_registry()
        .args(&["-m", "5", "set", pat, tag])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .expect("=== error setting tag ===");
}

// RM
pub fn wutag_rm(pat: &str, tag: &str) {
    wutag_cmd_new_registry()
        .args(&["rm", pat, tag])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .expect("=== error removing tag ===");
}

// LIST FILES
pub fn wutag_list_files() {
    wutag_cmd_new_registry()
        .args(&["list", "files"])
        .status()
        .expect("=== error listing tag ===");
}

// LIST TAGS
pub fn wutag_list_tags() {
    wutag_cmd_new_registry()
        .args(&["list", "tags"])
        .status()
        .expect("error removing tag");
}
