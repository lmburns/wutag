use super::*;
use crate::{expand_file, expand_file_dir_two};

#[test]
fn multiple_files_same_ft_glob() {
    wutag_clear();
    wutag()
        .args(&["set", "*.zsh", "tag_msg"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")));
}

#[test]
fn multiple_files_diff_ft_glob() {
    wutag_clear();
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
fn multiple_files_same_ft_regex() {
    wutag_clear();
    wutag()
        .args(&["--regex", "set", ".*\\.zsh$", "tag_msr"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")));
}

#[test]
fn multiple_files_diff_ft_regex() {
    wutag_clear();
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
fn multiple_files_same_ft_extension() {
    wutag_clear();
    wutag()
        .args(&["--ext", "zsh", "set", "*", "tag_mse"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")));
}

#[test]
fn multiple_multiple_extension() {
    wutag_clear();
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
fn multiple_files_multiple_glob() {
    wutag_clear();
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
fn multiple_files_multiple_regex() {
    wutag_clear();
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
fn multiple_multiple_extension_exclude() {
    wutag_clear();
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
fn multiple_files_multiple_glob_exclude() {
    wutag_clear();
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
fn multiple_files_multiple_regex_exclude() {
    wutag_clear();
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
fn file_depth_not_deep_enough() {
    wutag_clear();
    wutag()
        .args(&["-m", "2", "set", "*4deep.zsh", "tag_4deep_fail"])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/d1/d2/4deep.zsh").as_str())
        }));
}

#[test]
fn file_depth_deep_enough() {
    wutag_clear();
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
fn file_type_executable() {
    wutag_clear();
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
fn file_type_directory() {
    wutag_clear();
    wutag()
        .args(&["-m", "4", "-t", "d", "set", "*d2", "tag_dir_t"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("sampd/d1/d2")));
}

// ======================== CASE INSENSITIVE ==========================
// ====================================================================
#[test]
fn ignore_case_glob() {
    wutag_clear();
    wutag()
        .args(&["set", "*upper*", "tag_upper_fail"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("sampd/UPPER.rs")))
        .stdout(predicate::str::contains(expand_file!("sampd/upper-not.rs")));
}

#[test]
fn ignore_upperchar_glob() {
    wutag_clear();
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
fn ignore_upperchar_glob_fail() {
    wutag_clear();
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

// Default action without -i
#[test]
fn ignore_case_flag_glob() {
    wutag_clear();
    wutag()
        .args(&["-i", "set", "*upper*", "tag_upper_glob"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("sampd/UPPER.rs")))
        .stdout(predicate::str::contains(expand_file!("sampd/upper-not.rs")));
}

#[test]
fn ignore_case_flag_regex() {
    wutag_clear();
    wutag()
        .args(&["-i", "-r", "set", ".*upper.*\\.rs", "tag_upper_regex"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("sampd/UPPER.rs")))
        .stdout(predicate::str::contains(expand_file!("sampd/upper-not.rs")));
}

#[test]
fn case_flag_glob() {
    wutag_clear();
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
fn case_flag_regex() {
    wutag_clear();
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
fn case_overwrite_ignore_glob() {
    wutag_clear();
    wutag()
        .args(&["-s", "-i", "set", "*upper*", "tag_upper_glob"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("sampd/UPPER.rs")))
        .stdout(predicate::str::contains(expand_file!("sampd/upper-not.rs")));
}

#[test]
fn case_overwrite_ignore_regex() {
    wutag_clear();
    wutag()
        .args(&["-s", "-i", "-r", "set", ".*upper.*\\.rs", "tag_upper_regex"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("sampd/UPPER.rs")))
        .stdout(predicate::str::contains(expand_file!("sampd/upper-not.rs")));
}

#[test]
fn ignore_overrite_case_glob() {
    wutag_clear();
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
fn ignore_overrite_case_regex() {
    wutag_clear();
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
fn second_dir_glob() {
    wutag_clear();
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
fn second_dir_regex() {
    wutag_clear();
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
