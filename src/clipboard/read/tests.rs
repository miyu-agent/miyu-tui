//! 读剪贴板：按平台挑命令，第一个读得到的算数，卡住的放弃。

#[cfg(unix)]
use std::time::{Duration, Instant};

use super::{Os, candidates};
#[cfg(unix)]
use super::{Unreadable, read_with};

fn names(list: &[(&str, &[&str])]) -> Vec<String> {
    list.iter().map(|(cmd, _)| (*cmd).to_string()).collect()
}

#[test]
fn the_readers_follow_the_platform() {
    assert_eq!(
        names(&candidates(Os::Linux, true)),
        ["wl-paste", "xclip", "xsel"]
    );
    assert_eq!(
        names(&candidates(Os::Linux, false)),
        ["xclip", "xsel"],
        "没有 Wayland 不试 wl-paste"
    );
    assert_eq!(names(&candidates(Os::Mac, false)), ["pbpaste"]);
    assert_eq!(names(&candidates(Os::Windows, false)), ["powershell"]);
}

#[cfg(unix)]
#[test]
fn the_first_reader_that_works_wins() {
    let got = read_with(
        &[
            ("false", &[]),
            ("printf", &["甲\\n乙"]),
            ("printf", &["丙"]),
        ],
        Duration::from_secs(1),
    );
    assert_eq!(got, Ok("甲\n乙".to_string()));
    let none = read_with(
        &[("false", &[]), ("no-such-clipboard-tool", &[])],
        Duration::from_secs(1),
    );
    assert_eq!(none, Err(Unreadable));
}

#[cfg(unix)]
#[test]
fn a_reader_that_hangs_is_cut_off() {
    let t = Instant::now();
    let got = read_with(&[("sleep", &["5"])], Duration::from_millis(200));
    assert_eq!(got, Err(Unreadable));
    assert!(t.elapsed() < Duration::from_secs(2), "等到点就放弃");
}

#[cfg(unix)]
#[test]
fn a_big_clipboard_is_read_whole() {
    // 超过管道缓冲（64KB）的：边跑边读，不会被当成卡住。
    let got = read_with(
        &[("head", &["-c", "300000", "/dev/zero"])],
        Duration::from_secs(2),
    );
    assert_eq!(got.map(|t| t.len()), Ok(300_000));
}

#[cfg(unix)]
#[test]
fn a_picture_on_the_clipboard_is_not_read_as_text() {
    // 2026-10-22 项目主人在确认的理由里 Ctrl+V，剪贴板里是截图：wl-paste 交出 PNG 的原始字节，原来硬转成一大片乱码贴进去。
    let picture = read_with(&[("printf", &["\\211PNG\\r\\n"])], Duration::from_secs(1));
    assert_eq!(picture, Err(Unreadable), "不是 UTF-8 的当没读到");
    let next = read_with(
        &[("printf", &["\\211PNG\\r\\n"]), ("printf", &["字"])],
        Duration::from_secs(1),
    );
    assert_eq!(next, Ok("字".to_string()), "接着试下一个");
}
