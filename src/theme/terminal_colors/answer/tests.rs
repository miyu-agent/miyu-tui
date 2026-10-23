//! 读终端的回话：16 色、字色和底色。

use super::{Rebuilt, all_query, answered, ground, sixteen};

/// 照 kitty 的样子答 16 色：4 位十六进制、`ESC \` 收尾；第 4 号（蓝）是 `#4080ff`。
fn reply() -> Vec<u8> {
    let mut got = String::new();
    for n in 0..16 {
        let (r, g, b) = if n == 4 {
            (0x40, 0x80, 0xff)
        } else {
            (n * 8, n * 8, n * 8)
        };
        got.push_str(&format!(
            "\x1b]4;{n};rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}\x1b\\"
        ));
    }
    got.push_str("\x1b[?62;22c");
    got.into_bytes()
}

#[test]
fn the_sixteen_answers_are_read_whatever_the_digits_and_ending() {
    let colors = sixteen(&reply()).unwrap();
    assert_eq!(colors[4], (0x40, 0x80, 0xff));
    assert_eq!(colors[15], (120, 120, 120));
    // 两位十六进制、BEL 收尾（有的终端这么答）。
    let short: String = (0..16)
        .map(|n| format!("\x1b]4;{n};rgb:ff/00/80\x07"))
        .collect();
    assert_eq!(sixteen(short.as_bytes()).unwrap()[0], (255, 0, 128));
    // 少答一个的不算。
    let missing: String = (0..15)
        .map(|n| format!("\x1b]4;{n};rgb:ff/00/80\x07"))
        .collect();
    assert_eq!(sixteen(missing.as_bytes()), None);
    assert!(answered(&reply()));
    assert!(!answered(b"\x1b]4;0;rgb:ff/00/80\x07"));
}

#[test]
fn kittys_own_reply_reads_into_sixteen_colors() {
    // 真 kitty（无头，2026-10-14）答的原样：一个通道 4 位、`ESC \\` 收尾，DA1 带着分号结尾。
    let got = "\x1b]4;0;rgb:4c4c/4c4c/4c4c\x1b\\\x1b]4;1;rgb:acac/8a8a/8c8c\x1b\\\x1b]4;2;rgb:9a9a/b3b3/8d8d\x1b\\\x1b]4;3;rgb:acac/a9a9/8a8a\x1b\\\x1b]4;4;rgb:c9c9/bebe/ffff\x1b\\\x1b]4;5;rgb:eded/b8b8/cdcd\x1b\\\x1b]4;6;rgb:c9c9/c3c3/dcdc\x1b\\\x1b]4;7;rgb:f0f0/f0f0/f0f0\x1b\\\x1b]4;8;rgb:9393/8f8f/9999\x1b\\\x1b]4;9;rgb:c4c4/9e9e/a0a0\x1b\\\x1b]4;10;rgb:b0b0/c9c9/a3a3\x1b\\\x1b]4;11;rgb:c4c4/c1c1/9e9e\x1b\\\x1b]4;12;rgb:a3a3/9e9e/c4c4\x1b\\\x1b]4;13;rgb:ffff/d8d8/e6e6\x1b\\\x1b]4;14;rgb:c9c9/c3c3/dcdc\x1b\\\x1b]4;15;rgb:e7e7/e7e7/e7e7\x1b\\\x1b[?62;52;c";
    let colors = sixteen(got.as_bytes()).unwrap();
    assert_eq!(colors[0], (0x4c, 0x4c, 0x4c));
    assert_eq!(colors[4], (0xc9, 0xbe, 0xff));
    assert_eq!(colors[15], (0xe7, 0xe7, 0xe7));
    assert!(answered(got.as_bytes()));
}

#[test]
fn the_ground_reply_reads_both_colors_and_judges_dark_like_colorsaurus() {
    // 拿回焦点再问时自己读 OSC 10、11 的回话（H15，2026-10-15）：两句都答了才算，深浅和起来时 `terminal-colorsaurus` 判的
    // 一样。
    let got = "\x1b]10;rgb:c0c0/caca/f5f5\x1b\\\x1b]11;rgb:1a1a/1b1b/2626\x07\x1b[?62c";
    let dark = ground(got.as_bytes()).unwrap();
    assert_eq!((dark.fg, dark.bg), ((0xc0, 0xca, 0xf5), (0x1a, 0x1b, 0x26)));
    assert!(dark.dark);
    let light = ground(b"\x1b]10;rgb:34/3b/58\x1b\\\x1b]11;rgb:e1/e2/e7\x1b\\").unwrap();
    assert!(!light.dark, "浅底深字");
    assert_eq!(
        ground(b"\x1b]10;rgb:34/3b/58\x07\x1b[?62c"),
        None,
        "只答了字色"
    );
    // 一次问全：字色、底色在前，16 色、DA1 在后。
    let query = all_query();
    assert!(query.starts_with("\x1b]10;?\x1b\\\x1b]11;?\x1b\\\x1b]4;0;?"));
    assert!(query.ends_with("\x1b[c"));
}

#[test]
fn windows_events_without_their_esc_still_read_into_sixteen_colors() {
    // 2026-10-24 虚拟机里记下的：Windows 上回话打头的 ESC 读不到。照 reply() 去掉所有 ESC 一个字一个字喂进去，拼回来照样读得出。
    let lost: String = String::from_utf8(reply())
        .unwrap()
        .chars()
        .filter(|&c| c != '\x1b')
        .collect();
    let mut got = Rebuilt::default();
    for c in lost.chars() {
        got.push_char(c);
    }
    assert!(answered(&got.bytes), "DA1 认得出");
    assert_eq!(sixteen(&got.bytes), sixteen(&reply()));
    assert!(sixteen(&got.bytes).is_some());
    // ESC 在的照原样，不多补。
    let mut kept = Rebuilt::default();
    for byte in reply() {
        kept.push(byte);
    }
    assert_eq!(kept.bytes, reply());
}
