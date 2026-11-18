//! 命令的输出给人看之前洗一遍（蓝图「后台命令、子代理和侧边栏」第 3、5 条）：终端转义序列（颜色、光标移动、标题，
//! kitty、sixel 的图这种整段给终端的字串）去掉；一行里回车覆盖的只留最后一段（进度条）；制表符换成空格；别的控制字符去掉。

use std::iter::Peekable;
use std::str::Chars;

/// 洗过的输出；`tab` 是制表符换成几个空格。
pub fn clean(text: &str, tab: usize) -> String {
    let mut out = String::with_capacity(text.len());
    let mut line = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => skip_escapes(&mut chars, true),
            // 8 位的写法（发布前审查 T2-3）：CSI、OSC 和几种字串同 7 位的。
            '\u{9b}' => skip_csi(&mut chars),
            '\u{9d}' | '\u{90}' | '\u{98}' | '\u{9e}' | '\u{9f}' => {
                let next = skip_string(&mut chars);
                skip_escapes(&mut chars, next);
            }
            '\n' => {
                out.push_str(&line);
                out.push('\n');
                line.clear();
            }
            // `\r\n` 照换行；单独的回车是这一行从头重写（进度条）。
            '\r' if chars.peek() == Some(&'\n') => {}
            '\r' => line.clear(),
            '\t' => line.push_str(&" ".repeat(tab)),
            c if c.is_control() => {}
            c => line.push(c),
        }
    }
    out.push_str(&line);
    out
}

/// 结束了的命令，正文里那一行点开看的全文：洗过、去掉末尾的空行；前面还有没交的，第一行写 `…`；一个字都没有的写
/// `nothing`（「没有输出」）。
pub fn note_detail(output: &crate::core::JobOutput, tab: usize, nothing: &str) -> String {
    let text = clean(&output.text, tab);
    let text = text.trim_end();
    match (text.is_empty(), output.truncated) {
        (true, _) => nothing.to_string(),
        (false, true) => format!("{}\n{text}", crate::sheet::ellipsis()),
        (false, false) => text.to_string(),
    }
}

/// `pending`（刚读过一个 `ESC`）时跳过它起头的那一段；那一段是没收尾的字串、又碰上一个 `ESC` 的，接着跳下一段。
/// 一段一段地循环，不递归：一长串没收尾的字串也不会把栈用完。
fn skip_escapes(chars: &mut Peekable<Chars>, mut pending: bool) {
    while pending {
        pending = skip_escape(chars);
    }
}

/// 跳过 `ESC` 后面的一段：CSI（`ESC [` 到结尾那个字）；OSC（`ESC ]`）、DCS（`ESC P`，sixel、tmux 透传）、APC（`ESC _`，
/// kitty 的图）、PM（`ESC ^`）、SOS（`ESC X`）到字串结尾（发布前审查 T2-3：原来只认前两种，图的 base64 整段铺在面板里）；
/// 别的只跳一个字。交回字串是不是停在了下一段的 `ESC` 上（见 [`skip_string`]）。
fn skip_escape(chars: &mut Peekable<Chars>) -> bool {
    match chars.next() {
        Some('[') => {
            skip_csi(chars);
            false
        }
        Some(']' | 'P' | '_' | '^' | 'X') => skip_string(chars),
        _ => false,
    }
}

/// CSI 的参数到结尾那个字。
fn skip_csi(chars: &mut Peekable<Chars>) {
    for c in chars.by_ref() {
        if ('\x40'..='\x7e').contains(&c) {
            break;
        }
    }
}

/// 字串到结尾：`BEL`、`ESC \\` 或 8 位的 ST（U+009C）。字串里的 `ESC` 后面不是 `\\` 的，是字串没收尾就开始了下一段
/// 转义（终端照这样认）：字串到此为止，交回 `true`，那个 `ESC` 已经读过了，由调用的接着认下一段（审计 #064）。
fn skip_string(chars: &mut Peekable<Chars>) -> bool {
    while let Some(c) = chars.next() {
        match c {
            '\x07' | '\u{9c}' => return false,
            '\x1b' => {
                if chars.peek() == Some(&'\\') {
                    chars.next();
                    return false;
                }
                return true;
            }
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{clean, note_detail};
    use crate::core::JobOutput;

    #[test]
    fn an_ended_command_opens_on_its_cleaned_output() {
        let out = |text: &str, truncated| JobOutput {
            text: text.into(),
            lines: 0,
            running: false,
            truncated,
            ..JobOutput::default()
        };
        assert_eq!(
            note_detail(&out("\x1b[1mok\x1b[0m\n\n", false), 4, "没有输出"),
            "ok"
        );
        assert_eq!(
            note_detail(&out("tail\n", true), 4, "没有输出"),
            "…\ntail",
            "前面还有没交的"
        );
        assert_eq!(note_detail(&out("\n", false), 4, "没有输出"), "没有输出");
    }

    #[test]
    fn escapes_carriage_returns_and_tabs_are_cleaned() {
        assert_eq!(
            clean("\x1b[31merror\x1b[0m: x\n", 4),
            "error: x\n",
            "颜色去掉"
        );
        assert_eq!(
            clean("10%\r50%\r100%\ndone", 4),
            "100%\ndone",
            "进度条只留最后一段"
        );
        assert_eq!(clean("a\r\nb\r\n", 4), "a\nb\n", "\\r\\n 照换行");
        assert_eq!(clean("a\tb", 2), "a  b");
        assert_eq!(
            clean("\x1b]0;title\x07ok\x1b]8;;http://x\x1b\\link", 4),
            "oklink",
            "标题、链接去掉"
        );
        assert_eq!(
            clean("bell\x07 back\x08", 4),
            "bell back",
            "别的控制字符去掉"
        );
    }

    #[test]
    fn string_escapes_and_eight_bit_csi_leave_nothing_behind() {
        // 发布前审查（2026-10-30）：只认 `ESC [`、`ESC ]` 两种，别的只跳过 `ESC` 后面一个字。APC（kitty 的图片协议，
        // `kitten icat` 的输出）、DCS（sixel、tmux 透传）到 `ESC \` 为止整段是给终端的，不是给人看的字。
        assert_eq!(clean("\x1b_Ga=T,f=100;iVBORw0KGgo=\x1b\\ok", 4), "ok");
        assert_eq!(clean("\x1bPq#0;2;0;0;0~-\x1b\\ok", 4), "ok");
        // 8 位的 CSI（U+009B）：控制字符本身去掉了，参数留成了字。
        assert_eq!(clean("\u{9b}31mred\u{9b}0m", 4), "red");
    }

    /// 审计 2026-11-18 #064（`skip_string` 那一半）：字串（OSC 这些）里碰到的 `ESC` 后面不是 `\` 的，是字串没收尾、后面接着一段新的
    /// 转义（终端照这样认：`ESC` 结束字串，从它起认下一段）。原来不管后面是什么都吃掉一个字就当收尾，后面那段 CSI 的参数成了字。
    #[test]
    fn an_escape_inside_an_unfinished_string_starts_the_next_sequence() {
        assert_eq!(clean("\x1b]0;title\x1b[31mred\x1b[0m", 4), "red");
        assert_eq!(
            clean("\u{9d}0;title\x1b[31mred", 4),
            "red",
            "8 位的字串同样"
        );
        assert_eq!(
            clean("\x1b]0;a\x1b]0;b\x1b[1mok", 4),
            "ok",
            "一串没收尾的字串一段段认"
        );
    }
}
