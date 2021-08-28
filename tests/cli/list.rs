use super::*;
use crate::{expand_file, expand_file_dir_two};

// #[test]
// fn list_files() {
//     wutag_clean();
//     wutag_set("*.zsh", "zzz");
//     wutag_list_files();
//     // let tmp_dir = tempdir().expect("failed creating tempdir");
//     wutag()
//         .args(&["list", "files"])
//         .assert()
//         .stdout(predicate::str::contains("dpmas/samp.zsh"))
//         .stdout(predicate::str::contains("sampd/exec.zsh"))
//         .stdout(predicate::str::contains("sampd/pmas.zsh"));
// }

// #[test]
// fn list_files_global() {
//     wutag_clean();
//     wutag_set("*.zsh", "zshz");
//     wutag()
//         .args(&["list", "files"])
//         .assert()
//         .stdout(predicate::str::contains(expand_file!("dpmas/samp.zsh")))
//         .stdout(predicate::str::contains(expand_file!("sampd/exec.zsh")))
//         .stdout(predicate::str::contains(expand_file!("sampd/pmas.zsh")));
// }
