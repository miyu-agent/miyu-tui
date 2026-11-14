//! 纯 ASCII 主题下把界面上的字换成 ASCII（蓝图 `tui.md`「主题」第 6 条，check 13 ①，2026-11-14 主人验收：Linux 控制台里
//! 换成英文界面也「有很多无法显示的字符」）。两层：
//!
//! - 读界面字表、命令说明、运行状态行时整份换（[`plain`]）：能换成几个字的照几个字（`…` 换成 `...`、`↑/↓` 换成 `Up/Down`）。
//! - 画完一帧最后一道（[`frame`]）：剩下的标点、箭头、框线、几何图形、盲文、私用区（Nerd Font）这些符号块里的字，一格换成
//!   一个 ASCII 字，接住核心给的字、代码里写死的几个。字母、汉字这些不动（Linux 控制台的字体画得了带重音的拉丁字母）。

use std::borrow::Cow;

use ratatui::buffer::Buffer;
use unicode_width::UnicodeWidthStr;

/// 整串换的：先长后短（`↑/↓` 在 `↑` 前面）。引号换成单引号：这一层也用在 JSON 原文上，双引号会把字符串截断。
const WORDS: [(&str, &str); 30] = [
    ("↑/↓", "Up/Down"),
    ("↑↓", "Up/Down"),
    ("←/→", "Left/Right"),
    ("…", "..."),
    ("⋯", "..."),
    ("·", "-"),
    ("↑", "Up"),
    ("↓", "Down"),
    ("←", "<-"),
    ("→", "->"),
    ("×", "x"),
    ("●", "*"),
    ("↻", "~"),
    ("⋮", ":"),
    ("✓", "+"),
    ("✗", "x"),
    ("“", "'"),
    ("”", "'"),
    ("‘", "'"),
    ("’", "'"),
    ("▶", ">"),
    ("▼", "v"),
    ("▪", "*"),
    ("•", "*"),
    ("◦", "o"),
    ("◆", "*"),
    ("›", ">"),
    ("‹", "<"),
    ("—", "-"),
    ("Σ", "S"),
];

/// 一串字里能换的都换掉；没有要换的原样交回。
pub fn plain(text: &str) -> Cow<'_, str> {
    if text.is_ascii() {
        return Cow::Borrowed(text);
    }
    let mut out = text.to_string();
    for (from, to) in WORDS {
        if out.contains(from) {
            out = out.replace(from, to);
        }
    }
    Cow::Owned(out)
}

/// 一格里的符号换成一个 ASCII 字；不是符号块里的（字母、汉字、假名……）是 `None`。
pub fn symbol(c: char) -> Option<char> {
    if let Some((_, to)) = WORDS.iter().find(|(from, _)| from.chars().eq([c])) {
        return to.chars().next();
    }
    let code = u32::from(c);
    Some(match code {
        // 框线：横的、竖的、拐角
        0x2500..=0x257F => box_line(c),
        // 方块、几何图形、杂项符号、装饰符号、盲文（转圈）、私用区（Nerd Font）、表情
        0x2580..=0x25FF => '#',
        0x2800..=0x28FF => '*',
        0x2190..=0x21FF => '>',
        0x2000..=0x206F | 0x2100..=0x218F | 0x2200..=0x23FF | 0x2600..=0x27BF => '*',
        0xE000..=0xF8FF | 0xF_0000..=0x10_FFFD | 0x1_F000..=0x1_FAFF => '*',
        0x00B7 => '-',
        0x00D7 => 'x',
        _ => return None,
    })
}

/// 框线照方向换：横 `-`、竖 `|`、别的 `+`。
fn box_line(c: char) -> char {
    match c {
        '─' | '━' | '╌' | '╍' | '┄' | '┅' | '┈' | '┉' | '═' => '-',
        '│' | '┃' | '╎' | '╏' | '┆' | '┇' | '┊' | '┋' | '║' => '|',
        _ => '+',
    }
}

/// 画完的一帧：符号块里剩下的字一格换成一个 ASCII 字。占两格的（表情这类）换了以后后面那一格补空格。
pub fn frame(buf: &mut Buffer) {
    let width = usize::from(buf.area.width);
    let cells = buf.content.len();
    let mut i = 0;
    while i < cells {
        let symbol = buf.content[i].symbol();
        let wide = symbol.width() > 1;
        let swap = symbol
            .chars()
            .next()
            .filter(|c| !c.is_ascii())
            .and_then(self::symbol);
        if let Some(to) = swap {
            buf.content[i].set_char(to);
            if wide && (i + 1) % width != 0 && i + 1 < cells {
                buf.content[i + 1].set_char(' ');
            }
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    use super::{frame, plain};

    #[test]
    fn words_become_ascii_and_ascii_is_left_alone() {
        assert_eq!(
            plain("↑/↓ select · Enter switch"),
            "Up/Down select - Enter switch"
        );
        assert_eq!(plain("Reading…"), "Reading...");
        assert_eq!(plain("{a} → {b}"), "{a} -> {b}");
        assert!(matches!(
            plain("plain"),
            std::borrow::Cow::Borrowed("plain")
        ));
        // 用在 JSON 原文上：换完还是能读的 JSON
        let json = "{\"a\": \"say “hi” …\"}";
        let parsed: serde_json::Value = serde_json::from_str(&plain(json)).unwrap();
        assert_eq!(parsed["a"], "say 'hi' ...");
    }

    #[test]
    fn a_frame_keeps_letters_and_turns_symbols_into_ascii() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 12, 1));
        buf.set_string(0, 0, "╭─é\u{f130}中⠋", ratatui::style::Style::new());
        frame(&mut buf);
        let cells: Vec<&str> = buf.content.iter().map(|c| c.symbol()).collect();
        assert_eq!(cells[..5], ["+", "-", "é", "*", "中"], "{cells:?}");
        assert_eq!(cells[6], "*", "盲文的转圈：{cells:?}");
    }
}
