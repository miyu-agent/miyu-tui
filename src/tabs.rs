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
        .any(|c| (c.is_control() && c != '\n') || hiding(c))
    {
        return Cow::Borrowed(text);
    }
    let tab = tab.max(1);
    let mut out = String::with_capacity(text.len() + 8);
    let mut col = 0;
    for c in unhidden(text) {
        match c {
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

/// 外来的字（软件源的包名和说明、包设置的记录、供应商的模型名、会话标题）进画面以前去掉画不出来、会藏字的：控制字符
/// （换行除外）、图格字和国旗以外的标签字符（发布前第五批审查 T5-3，同 T2-4）。没有的原样交回。
pub fn printable(text: &str) -> Cow<'_, str> {
    if !text
        .chars()
        .any(|c| (c.is_control() && c != '\n') || hiding(c))
    {
        return Cow::Borrowed(text);
    }
    Cow::Owned(
        unhidden(text)
            .filter(|c| *c == '\n' || !c.is_control())
            .collect(),
    )
}

/// 排好的行里每一段字都过 [`printable`]：外来的字拼进行里以后才画的地方用（「软件包」三页、记录列表）。
pub fn printable_lines(lines: &mut [ratatui::text::Line<'_>]) {
    for span in lines.iter_mut().flat_map(|l| l.spans.iter_mut()) {
        if let Cow::Owned(clean) = printable(&span.content) {
            span.content = Cow::Owned(clean);
        }
    }
}

/// 可能要去掉的字：图格字、标签字符。
fn hiding(c: char) -> bool {
    c == PLACEHOLDER || tag(c)
}

/// 去掉图格字（连同跟在后面写行列的附加符）和国旗以外的标签字符，别的一个个照原样交出。
fn unhidden(text: &str) -> impl Iterator<Item = char> + '_ {
    // 在国旗的标签串里；刚去掉一个图格字。
    let (mut flag, mut placeholder) = (false, false);
    text.chars().filter(move |&c| {
        if placeholder && c.width() == Some(0) {
            return false;
        }
        placeholder = c == PLACEHOLDER;
        let in_flag = flag && tag(c);
        flag = c == '\u{1F3F4}' || (in_flag && c != '\u{E007F}');
        !placeholder && (!tag(c) || in_flag)
    })
}

/// 要跑的命令的制表位（终端、shell 照 8 列）：和代码、输出的 `output.tab` 分开。
pub const COMMAND_TAB: usize = 8;

/// 给人照着批的命令（确认抽屉、时间线的预览，发布前审查 T2-2，2026-10-30）：屏上要和真正要跑的一样，看不见的字一个都
/// 不能悄悄没了。制表符照制表位展开（shell 照它分参数，删了几个参数会连成一个词）；C0 照脱字写法写成 `^[`（ESC）、`^M`
/// 这种，只用 ASCII，纯 ASCII 主题下也一样；DEL 写 `^?`；C1 控制字符、双向控制字符（U+061C、U+200E、U+200F、U+202A–202E、
/// U+2066–2069）和本身看不见的字（[`invisible`]，第五批审查 T5-2）写成 `<U+202E>`。换行照旧是换行。没有的原样交回。
pub fn shown(text: &str) -> Cow<'_, str> {
    let mut prev = None;
    let marked = |c: char, prev: Option<char>| c.is_control() || bidi(c) || invisible(c, prev);
    if !text.chars().any(|c| {
        let hit = c != '\n' && marked(c, prev);
        prev = Some(c);
        hit
    }) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 16);
    let mut col = 0;
    let mut prev = None;
    for c in text.chars() {
        let before = prev.replace(c);
        let mark = match c {
            '\n' => {
                out.push('\n');
                col = 0;
                continue;
            }
            '\t' => " ".repeat(COMMAND_TAB - col % COMMAND_TAB),
            '\u{7f}' => "^?".to_string(),
            c if u32::from(c) < 0x20 => format!("^{}", char::from(b'@' + c as u8)),
            c if marked(c, before) => format!("<U+{:04X}>", u32::from(c)),
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

/// 本身看不见、画面库整个丢掉的字（第五批审查 T5-2）：软连字符、零宽空格和连接符、词连接符、BOM、填充字、行段分隔符、
/// 标签字符、图格字。零宽连接符和变体选择符接在符号、表情（U+2000 以后的字）后面的是那个字的一部分，照写；接在 ASCII、
/// 拉丁字母后面、打头的看不见。
fn invisible(c: char, prev: Option<char>) -> bool {
    match c {
        '\u{200d}' | '\u{fe00}'..='\u{fe0f}' => prev.is_none_or(|p| p < '\u{2000}'),
        '\u{00ad}' | '\u{034f}' | '\u{115f}' | '\u{1160}' | '\u{180e}' | '\u{200b}'
        | '\u{200c}' => true,
        '\u{2028}'
        | '\u{2029}'
        | '\u{2060}'..='\u{2064}'
        | '\u{3164}'
        | '\u{feff}'
        | '\u{ffa0}' => true,
        c => hiding(c) || ('\u{E0000}'..='\u{E001F}').contains(&c),
    }
}

#[cfg(test)]
mod tests;
