//! 制表符展开成空格（review C1，2026-10-11）：画面库把带控制字符的字素整个丢掉，排版却照 1 列算，代码块、命令的输出、
//! 差异里的缩进没了，这一行后面的点击区、复制的列也偏一格。进排版以前照制表位展开：每行从头数，`tab` 列一个制表位
//! （`layout.json` 的 `output.tab`）；别的控制字符（换行除外）画不出来，去掉。外来的字里 kitty 的图格字 U+10EEEE（连同
//! 跟在后面写行列的附加符）和国旗以外的标签字符也去掉（发布前审查 T2-4）：图格字是界面自己放图写的，kitty 和这边几处照它
//! 认图，混进来会把字藏起来；标签字符看不见、复制时一起带走。

use std::borrow::Cow;

use unicode_width::UnicodeWidthChar;

/// kitty 放图时每一格写的字（Unicode 占位）。
const PLACEHOLDER: char = '\u{10EEEE}';

/// 标签字符：只在国旗（🏴 后面接一串、`U+E007F` 收尾）里有意思。
fn tag(c: char) -> bool {
    ('\u{E0020}'..='\u{E007F}').contains(&c)
}

/// 展开 `text` 里的制表符、去掉别的控制字符、图格字和国旗以外的标签字符；没有的原样交回。
pub fn expand(text: &str, tab: usize) -> Cow<'_, str> {
    if !text
        .chars()
        .any(|c| (c.is_control() && c != '\n') || c == PLACEHOLDER || tag(c))
    {
        return Cow::Borrowed(text);
    }
    let tab = tab.max(1);
    let mut out = String::with_capacity(text.len() + 8);
    let mut col = 0;
    // 在国旗的标签串里；刚去掉一个图格字（后面写行列的附加符一起去掉）。
    let (mut flag, mut placeholder) = (false, false);
    for c in text.chars() {
        if placeholder && c.width() == Some(0) {
            continue;
        }
        placeholder = c == PLACEHOLDER;
        let in_flag = flag && tag(c);
        flag = c == '\u{1F3F4}' || (in_flag && c != '\u{E007F}');
        match c {
            c if c == PLACEHOLDER => {}
            c if tag(c) && !in_flag => {}
            '\n' => {
                out.push('\n');
                col = 0;
            }
            '\t' => {
                let n = tab - col % tab;
                out.extend(std::iter::repeat_n(' ', n));
                col += n;
            }
            c if c.is_control() => {}
            c => {
                out.push(c);
                col += c.width().unwrap_or(0);
            }
        }
    }
    Cow::Owned(out)
}

/// 要跑的命令的制表位（终端、shell 照 8 列）：和代码、输出的 `output.tab` 分开。
pub const COMMAND_TAB: usize = 8;

/// 给人照着批的命令（确认抽屉、时间线的预览，发布前审查 T2-2，2026-10-30）：屏上要和真正要跑的一样，看不见的字一个都
/// 不能悄悄没了。制表符照制表位展开（shell 照它分参数，删了几个参数会连成一个词）；C0 照脱字写法写成 `^[`（ESC）、`^M`
/// 这种，只用 ASCII，纯 ASCII 主题下也一样；DEL 写 `^?`；C1 控制字符和双向控制字符（U+061C、U+200E、U+200F、U+202A–202E、U+2066–2069）写成 `<U+202E>`。
/// 换行照旧是换行。没有的原样交回。
pub fn shown(text: &str) -> Cow<'_, str> {
    if !text
        .chars()
        .any(|c| (c.is_control() && c != '\n') || bidi(c))
    {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 16);
    let mut col = 0;
    for c in text.chars() {
        let mark = match c {
            '\n' => {
                out.push('\n');
                col = 0;
                continue;
            }
            '\t' => " ".repeat(COMMAND_TAB - col % COMMAND_TAB),
            '\u{7f}' => "^?".to_string(),
            c if u32::from(c) < 0x20 => format!("^{}", char::from(b'@' + c as u8)),
            c if c.is_control() || bidi(c) => format!("<U+{:04X}>", u32::from(c)),
            c => {
                out.push(c);
                col += c.width().unwrap_or(0);
                continue;
            }
        };
        col += unicode_width::UnicodeWidthStr::width(mark.as_str());
        out.push_str(&mark);
    }
    Cow::Owned(out)
}

/// 改字的显示先后、本身看不见的双向控制字符。
fn bidi(c: char) -> bool {
    matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

#[cfg(test)]
mod tests;
