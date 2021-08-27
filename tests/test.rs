#![allow(unused)]
use assert_cmd::cargo::CommandCargoExt;
use predicates::{prelude::predicate, str::PredicateStrExt};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serial_test::serial;
use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::from_utf8,
};
use tempfile::tempdir;

const FILE_DIR: &str = "tests/example_files";
const ANOTHER_FILE_DIR: &str = "tests/sample_dir";
const NEW_REGISTRY: &str = "tests/sample.reg";

lazy_static::lazy_static! {
    static ref CWD: PathBuf = env::current_dir()
        .expect("unable to get CWD").join(FILE_DIR);
    static ref CWD_TWO: PathBuf = env::current_dir().expect("unable to get CWD")
        .join(ANOTHER_FILE_DIR);
}

macro_rules! tag_out {
    ($file:literal => $($set:tt),* $(,)?) => {
        format!("{}/{}:\n\t{}\n", CWD.display(), $file, format!($($set),*))
    }
}

macro_rules! expand_file {
    ($file:literal) => {
        format!("{}/{}", CWD.display(), $file)
    };
}

macro_rules! expand_file_dir_two {
    ($file:literal) => {
        format!("{}/{}", CWD_TWO.display(), $file)
    };
}

fn create_temp_path() -> String {
    let mut rng = thread_rng();
    let mut tmp_path = env::temp_dir();
    tmp_path.push(
        std::iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .map(char::from)
            .take(12)
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

fn wutag_cmd_new_registry() -> Command {
    let mut cmd = wutag_cmd_base_registry();
    cmd.args(["--registry", NEW_REGISTRY, "--color=never"]);
    cmd
}

fn wutag_cmd_new_registry_global() -> Command {
    let mut cmd = wutag_cmd_base_registry();
    cmd.args(["--registry", NEW_REGISTRY, "--global", "--color=never"]);
    cmd
}

fn wutag_cmd_random_registry() -> Command {
    let mut cmd = wutag_cmd_base_registry();
    cmd.args([
        "--registry",
        create_temp_path().as_str(),
        "--global",
        "--color=never",
    ]);
    cmd
}

// ============================== SUBCMDS ===============================
// ======================================================================

fn wutag() -> assert_cmd::Command {
    assert_cmd::Command::from_std(wutag_cmd_new_registry())
}

fn wutag_global() -> assert_cmd::Command {
    assert_cmd::Command::from_std(wutag_cmd_new_registry_global())
}

fn wutag_rr() -> assert_cmd::Command {
    assert_cmd::Command::from_std(wutag_cmd_random_registry())
}

// CLEAR
fn wutag_clear() {
    wutag_cmd_new_registry()
        .args(&["clear", "*"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("error removing tag");
}

fn wutag_clear_global() {
    wutag_cmd_new_registry_global()
        .args(&["clear", "*"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("error removing tags globally");
}

// CLEAN
fn wutag_clean() {
    wutag_cmd_new_registry()
        .args(&["clean-cache"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("error clearing out the registry");
}

// SET
fn wutag_set(pat: &str, tag: &str) {
    wutag_cmd_new_registry()
        .args(&["set", pat, tag])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("error setting tag");
}

// RM
fn wutag_rm(pat: &str, tag: &str) {
    wutag_cmd_new_registry()
        .args(&["rm", pat, tag])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("error removing tag");
}

// LIST
fn wutag_list_files() {
    wutag_cmd_new_registry()
        .args(&["list", "files"])
        .status()
        .expect("error removing tag");
}

// ============================== COLOR ===============================
// ====================================================================

#[test]
fn set_base() {
    wutag_clear();
    wutag()
        .args(&["set", "*hello.c", "tag"])
        .assert()
        .success()
        .stdout(tag_out!("hello.c" => "+ {}", "tag"));
}

#[test]
fn set_color_hash() {
    wutag_clear();
    wutag()
        .args(&["set", "--color", "#83a598", "*hello.c", "tag_hc"])
        .assert()
        .success()
        .stdout(tag_out!("hello.c" => "+ {}", "tag_hc"));
}

#[test]
fn set_color_hex() {
    wutag_clear();
    wutag()
        .args(&["set", "--color", "0x83a598", "*hello.c", "tag_xcx"])
        .assert()
        .success()
        .stdout(tag_out!("hello.c" => "+ {}", "tag_xcx"));
}

#[test]
fn set_color_plain() {
    wutag_clear();
    wutag()
        .args(&["set", "--color", "83a598", "*hello.c", "tag_pc"])
        .assert()
        .success()
        .stdout(tag_out!("hello.c" => "+ {}", "tag_pc"));
}

// =============================== GLOB/REGEX ===============================
// ==========================================================================

// Notice that stdout is being checked for the file name, not stderr
#[test]
fn set_multiple_files_same_ft_glob() {
    wutag_clean();
    wutag()
        .args(&["set", "*.zsh", "tag_msg"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")));
}

#[test]
fn set_multiple_files_diff_ft_glob() {
    wutag_clean();
    wutag()
        .args(&["set", "*sh", "tag_mdg"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")))
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.bash")))
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.sh")));
}

#[test]
fn set_multiple_files_same_ft_regex() {
    wutag_clean();
    wutag()
        .args(&["--regex", "set", ".*\\.zsh$", "tag_msr"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")));
}

#[test]
fn set_multiple_files_diff_ft_regex() {
    wutag_clean();
    wutag()
        .args(&["--regex", "set", ".*sh$", "tag_msr"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")))
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.bash")))
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.sh")));
}

#[test]
fn set_multiple_files_same_ft_extension() {
    wutag_clean();
    wutag()
        .args(&["--ext", "zsh", "set", "*", "tag_mse"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")));
}

#[test]
fn set_multiple_multiple_extension() {
    wutag_clean();
    wutag()
        .args(&["-e", "zsh", "-e", "bash", "-e", "sh", "set", "*", "tag_mde"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")))
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.bash")))
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.sh")));
}

#[test]
fn set_multiple_files_multiple_glob() {
    wutag_clean();
    wutag()
        .args(&["set", "*.{zsh,sh,bash}", "tag_mfmg"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")))
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.bash")))
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.sh")));
}

#[test]
fn set_multiple_files_multiple_regex() {
    wutag_clean();
    wutag()
        .args(&["--regex", "set", ".*\\.(zsh|sh|bash)$", "tag_mfmg"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmas.sh")))
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.bash")))
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.sh")));
}

// ============================== EXCLUDE ==============================
// =====================================================================
#[test]
fn set_multiple_multiple_extension_exclude() {
    wutag_clean();
    wutag()
        .args(&["-e", "zsh", "-E", "dpmas/", "set", "*", "tag_mee"])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("dpmas/samp.zsh").as_str())
        }))
        .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")));
}

#[test]
fn set_multiple_files_multiple_glob_exclude() {
    wutag_clean();
    wutag()
        .args(&["-E", "sampd/", "set", "*.{zsh,bash}", "tag_mfmge"])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/pmas.zsh").as_str())
        }))
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.bash")));
}

#[test]
fn set_multiple_files_multiple_regex_exclude() {
    wutag_clean();
    wutag()
        .args(&[
            "--regex",
            "-E",
            "sampd/",
            "set",
            ".*\\.(zsh|bash)$",
            "tag_mfmre",
        ])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/pmas.zsh").as_str())
        }))
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.bash")));
}

// ============================== DEPTH ===============================
// =====================================================================
#[test]
fn set_file_depth_not_deep_enought() {
    wutag_clean();
    wutag()
        .args(&["-m", "2", "set", "*4deep.zsh", "tag_4deep_fail"])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/d1/d2/4deep.zsh").as_str())
        }));
}

#[test]
fn set_file_depth_deep_enought() {
    wutag_clean();
    wutag()
        .args(&["-m", "4", "set", "*4deep.zsh", "tag_4deep_success"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!(
            "sampd/d1/d2/4deep.zsh"
        )));
}

// ============================== TYPES ================================
// =====================================================================
#[test]
fn set_file_type_executable() {
    wutag_clean();
    wutag()
        .args(&["-t", "x", "set", "*exec*.zsh", "tag_exec_t"])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/exec-not.zsh").as_str())
        }))
        .stdout(predicate::str::contains(expand_file!("sampd/exec.zsh")));
}

#[test]
fn set_file_type_directory() {
    wutag_clean();
    wutag()
        .args(&["-m", "4", "-t", "d", "set", "*d2", "tag_dir_t"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("sampd/d1/d2")));
}

// ============================== CASE INSENSITIVE
// ================================
// ================================================================================
#[test]
fn set_ignore_case_glob() {
    wutag_clean();
    wutag()
        .args(&["set", "*upper*", "tag_upper_fail"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("sampd/UPPER.rs")))
        .stdout(predicate::str::contains(expand_file!("sampd/upper-not.rs")));
}

#[test]
fn set_ignore_upperchar_glob() {
    wutag_clean();
    wutag()
        .args(&["set", "*UPPER*", "tag_upperchar"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("sampd/UPPER.rs")))
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/upper-not.rs").as_str())
        }));
}

#[test]
fn set_ignore_upperchar_glob_fail() {
    wutag_clean();
    wutag()
        .args(&["set", "*uPpEr*", "tag_upperchar_fail"])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/UPPER.rs").as_str())
        }))
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/upper-not.rs").as_str())
        }));
}

#[test]
fn set_ignore_case_flag_glob() {
    wutag_clean();
    wutag()
        .args(&["-i", "set", "*upper*", "tag_upper_glob"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("sampd/UPPER.rs")))
        .stdout(predicate::str::contains(expand_file!("sampd/upper-not.rs")));
}

#[test]
fn set_ignore_case_flag_regex() {
    wutag_clean();
    wutag()
        .args(&["-i", "-r", "set", ".*upper.*\\.rs", "tag_upper_regex"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("sampd/UPPER.rs")))
        .stdout(predicate::str::contains(expand_file!("sampd/upper-not.rs")));
}

#[test]
fn set_case_flag_glob() {
    wutag_clean();
    wutag()
        .args(&["-s", "set", "*upper*", "tag_upper_glob"])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/UPPER.rs").as_str())
        }))
        .stdout(predicate::str::contains(expand_file!("sampd/upper-not.rs")));
}

#[test]
fn set_case_flag_regex() {
    wutag_clean();
    wutag()
        .args(&["-s", "-r", "set", ".*upper.*\\.rs", "tag_upper_regex"])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/UPPER.rs").as_str())
        }))
        .stdout(predicate::str::contains(expand_file!("sampd/upper-not.rs")));
}

#[test]
fn set_case_overrite_ignore_glob() {
    wutag_clean();
    wutag()
        .args(&["-s", "-i", "set", "*upper*", "tag_upper_glob"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("sampd/UPPER.rs")))
        .stdout(predicate::str::contains(expand_file!("sampd/upper-not.rs")));
}

#[test]
fn set_case_overrite_ignore_regex() {
    wutag_clean();
    wutag()
        .args(&["-s", "-i", "-r", "set", ".*upper.*\\.rs", "tag_upper_regex"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("sampd/UPPER.rs")))
        .stdout(predicate::str::contains(expand_file!("sampd/upper-not.rs")));
}

#[test]
fn set_ignore_overrite_case_glob() {
    wutag_clean();
    wutag()
        .args(&["-i", "-s", "set", "*upper*", "tag_upper_glob"])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/UPPER.rs").as_str())
        }))
        .stdout(predicate::str::contains(expand_file!("sampd/upper-not.rs")));
}

#[test]
fn set_ignore_overrite_case_regex() {
    wutag_clean();
    wutag()
        .args(&["-i", "-s", "-r", "set", ".*upper.*\\.rs", "tag_upper_regex"])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/UPPER.rs").as_str())
        }))
        .stdout(predicate::str::contains(expand_file!("sampd/upper-not.rs")));
}

#[test]
fn set_second_dir_glob() {
    wutag_clean();
    wutag()
        .args(&[
            "-d",
            CWD_TWO.display().to_string().as_str(),
            "set",
            "*.rs",
            "adir_rust",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file_dir_two!(
            "another_dir.rs"
        )));
}

#[test]
fn set_second_dir_regex() {
    wutag_clean();
    wutag()
        .args(&[
            "-d",
            CWD_TWO.display().to_string().as_str(),
            "-r",
            "set",
            ".*\\.(rs|zsh)$",
            "adir_rzsh",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file_dir_two!(
            "another_dir.rs"
        )))
        .stdout(predicate::str::contains(expand_file_dir_two!("d1/a.zsh")));
}

// ====================================================================
// ============================== REMOVE ==============================
// ====================================================================
#[test]
fn rm_local_glob() {
    wutag_clean();
    wutag_set("*.toml", "toml");
    wutag()
        .args(&["rm", "*.toml", "toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/asdf.toml")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmAs.toml")));
}

#[test]
fn rm_local_case_sensitive_glob() {
    wutag_clean();
    wutag_set("*.toml", "toml");
    wutag()
        .args(&["rm", "*A*.toml", "toml"])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("dpmas/asdf.rs").as_str())
        }))
        .stdout(predicate::str::contains(expand_file!("sampd/pmAs.toml")));
}

#[test]
fn rm_local_case_sensitive_flag_glob() {
    wutag_clean();
    wutag_set("*.toml", "toml");
    wutag()
        .args(&["-s", "rm", "*a*.toml", "toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/asdf.toml")))
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/pmAs.rs").as_str())
        }));
}

#[test]
fn rm_local_regex() {
    wutag_clean();
    wutag_set("*.toml", "toml");
    wutag()
        .args(&["-r", "rm", ".*\\.toml", "toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/asdf.toml")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmAs.toml")));
}

#[test]
fn rm_local_case_sensitive_regex() {
    wutag_clean();
    wutag_set("*.toml", "toml");
    wutag()
        .args(&["-r", "rm", ".*A.*\\.toml", "toml"])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("dpmas/asdf.rs").as_str())
        }))
        .stdout(predicate::str::contains(expand_file!("sampd/pmAs.toml")));
}

#[test]
fn rm_local_case_sensitive_flag_regex() {
    wutag_clean();
    wutag_set("*.toml", "toml");
    wutag()
        .args(&["-r", "-s", "rm", ".*a.*\\.toml", "toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/asdf.toml")))
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/pmAs.rs").as_str())
        }));
}

// #[test]
// fn rm_global_glob() {
//     wutag_clean_global();
//     wutag_set("*.zsh", "zzz");
//     // let tmp_dir = tempdir().expect("failed creating tempdir");
//     wutag()
//         .args(&["-g", "rm", "*.zsh", "zzz"])
//         .assert()
//         .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
//         .stdout(predicate::str::contains(expand_file!("sampd/exec.zsh")))
//         .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")));
// }

#[test]
fn list_files() {
    wutag_clean();
    wutag_set("*.zsh", "zzz");
    wutag_list_files();
    // let tmp_dir = tempdir().expect("failed creating tempdir");
    wutag()
        .args(&["list", "files"])
        .assert()
        .stdout(predicate::str::contains("dpmas/samp.zsh"))
        .stdout(predicate::str::contains("sampd/exec.zsh"))
        .stdout(predicate::str::contains("sampd/pmas.zsh"));
}

// #[test]
// fn list_files_global() {
//     wutag_clean();
//     wutag_set("*.zsh", "zzz");
//     // let tmp_dir = tempdir().expect("failed creating tempdir");
//     wutag()
//         .args(&["list", "files"])
//         .assert()
//         .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
//         .stdout(predicate::str::contains(expand_file!("sampd/exec.zsh")))
//         .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")));
// }

// #[test]
// fn rm_global_case_sensitive_glob() {
//     wutag_clean();
//     wutag_set("*.zsh", "zzz");
//     wutag_global()
//         .args(&["rm", "*eX*.zsh", "zzz"])
//         .assert()
//         .stdout(predicate::str::contains(expand_file!("sampd/eXeC-cap.zsh")))
//         .stdout(predicate::function(|f: &str| {
//             !f.contains(expand_file!("sampd/exec-not.rs").as_str())
//         }))
//         .stdout(predicate::function(|f: &str| {
//             !f.contains(expand_file!("sampd/exec.rs").as_str())
//         }));
// }

// #[test]
// fn rm_global_case_sensitive_flag_glob() {
//     wutag_clean();
//     wutag_set("*.zsh", "zzz");
//     wutag_global()
//         .args(&["rm", "*.zsh", "zzz"])
//         .assert()
//         .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
//         .stdout(predicate::str::contains(expand_file!("sampd/exec.zsh")))
//         .stdout(predicate::str::contains(expand_file!("sampd/exec-not.zsh")))
//         .stdout(predicate::str::contains(expand_file!("sampd/eXeC-cap.zsh")))
//         .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")));
// }
//
