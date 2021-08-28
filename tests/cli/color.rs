use super::*;
use crate::tag_out;

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
