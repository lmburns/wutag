use super::*;
use crate::{expand_file, expand_file_dir_two};

#[test]
fn local_glob() {
    wutag_set("*.toml", "toml");
    wutag()
        .args(&["rm", "*.toml", "toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/asdf.toml")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmAs.toml")));
}

#[test]
fn local_case_sensitive_glob() {
    wutag_set("*.toml", "toml_t");
    wutag()
        .args(&["rm", "*A*.toml", "toml_t"])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("dpmas/asdf.rs").as_str())
        }))
        .stdout(predicate::str::contains(expand_file!("sampd/pmAs.toml")));
}

#[test]
fn local_case_sensitive_flag_glob() {
    wutag_set("*.toml", "toml_s");
    wutag()
        .args(&["-s", "rm", "*a*.toml", "toml_s"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/asdf.toml")))
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/pmAs.rs").as_str())
        }));
}

#[test]
fn local_regex() {
    wutag_set("*.toml", "toml_x");
    wutag()
        .args(&["-r", "rm", ".*\\.toml", "toml_x"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/asdf.toml")))
        .stdout(predicate::str::contains(expand_file!("sampd/pmAs.toml")));
}

#[test]
fn local_case_sensitive_regex() {
    wutag_set("*.toml", "toml_k");
    wutag()
        .args(&["-r", "rm", ".*A.*\\.toml", "toml_k"])
        .assert()
        .success()
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("dpmas/asdf.rs").as_str())
        }))
        .stdout(predicate::str::contains(expand_file!("sampd/pmAs.toml")));
}

#[test]
fn local_case_sensitive_flag_regex() {
    wutag_set("*.toml", "tomlaa");
    wutag()
        .args(&["-r", "-s", "rm", ".*a.*\\.toml", "tomlaa"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/asdf.toml")))
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/pmAs.rs").as_str())
        }));
}

#[test]
fn local_extension_glob() {
    wutag_set("*.c", "cext");
    wutag()
        .args(&["-e", "c", "rm", "*", "cext"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("hello.c")));
}

#[test]
fn local_exclude_glob() {
    wutag_set("*.zsh", "extg");
    wutag()
        .args(&["-E", "sampd/", "rm", "*.zsh", "extg"])
        .assert()
        .success()
        .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
        .stdout(predicate::function(|f: &str| {
            !f.contains(expand_file!("sampd/exec.zsh").as_str())
        }));
}

#[test]
fn global_glob() {
    INIT.call_once(|| {
        wutag_set("*.c", "zzzk");
        wutag_global()
            .args(&["rm", "*.c", "zzzk"])
            .assert()
            .success()
            .stdout(predicate::str::contains(expand_file!("hello.c")));
    });
}

//
#[test]
fn global_case_sensitive_glob() {
    INIT.call_once(|| {
        wutag_set("*.zsh", "zshz");
        wutag_global()
            .args(&["rm", "*eX*.zsh", "zshz"])
            .assert()
            .success()
            .stdout(predicate::str::contains(expand_file!("sampd/eXeC-cap.zsh")))
            .stdout(predicate::function(|f: &str| {
                !f.contains(expand_file!("sampd/exec-not.rs").as_str())
            }))
            .stdout(predicate::function(|f: &str| {
                !f.contains(expand_file!("sampd/exec.rs").as_str())
            }));
    })
}

#[test]
fn global_case_sensitive_flag_glob() {
    INIT.call_once(|| {
        wutag_set("*.zsh", "zkz");
        wutag_global()
            .args(&["rm", "*.zsh", "zkz"])
            .assert()
            .success()
            .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
            .stdout(predicate::str::contains(expand_file!("sampd/exec.zsh")))
            .stdout(predicate::str::contains(expand_file!("sampd/exec-not.zsh")))
            .stdout(predicate::str::contains(expand_file!("sampd/eXeC-cap.zsh")))
            .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")));
    });
}
