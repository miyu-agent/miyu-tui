//! 制表符展开成空格（review C1，2026-10-11）：画面库把带控制字符的字素整个丢掉，排版却照 1 列算，代码块、命令的输出、
//! 差异里的缩进没了，这一行后面的点击区、复制的列也偏一格。进排版以前照制表位展开：每行从头数，`tab` 列一个制表位
//! （`layout.json` 的 `output.tab`）；别的控制字符（换行除外）画不出来，去掉。

use std::borrow::Cow;

use unicode_width::UnicodeWidthChar;

/// 展开 `text` 里的制表符、去掉别的控制字符；没有的原样交回。
pub fn expand(text: &str, tab: usize) -> Cow<'_, str> {
    if !text.chars().any(|c| c.is_control() && c != '\n') {
        return Cow::Borrowed(text);
    }
    let tab = tab.max(1);
    let mut out = String::with_capacity(text.len() + 8);
    let mut col = 0;
    for c in text.chars() {
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

#[cfg(test)]
mod tests;
