//! 界面块的字怎么折（蓝图「界面块」第 6 条）：照显示宽度折，有 `/`、空格的先断在它后面（路径、句子不从词中间劈开），
//! 一行里都没有的照字折。`\n` 另起一行。

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// 折成几行，每行不超过 `width` 列。
pub fn soft(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut rest: Vec<&str> = paragraph.graphemes(true).collect();
        if rest.is_empty() {
            lines.push(String::new());
            continue;
        }
        while !rest.is_empty() {
            let mut used = 0;
            let mut fit = 0;
            for g in &rest {
                let w = g.width();
                if used + w > width {
                    break;
                }
                used += w;
                fit += 1;
            }
            if fit == rest.len() {
                lines.push(rest.concat());
                break;
            }
            // 断在这一截里最后一个 `/`、空格后面；打头的不算，不然留下一个孤零零的记号。
            let cut = rest[..fit]
                .iter()
                .rposition(|g| *g == "/" || *g == " ")
                .filter(|at| *at > 0)
                .map_or(fit.max(1), |at| at + 1);
            lines.push(rest[..cut].concat().trim_end().to_string());
            rest.drain(..cut);
            // 下一行不以断开处的空格打头。
            while rest.first() == Some(&" ") {
                rest.remove(0);
            }
        }
    }
    lines
}

/// 前面放得进 `width` 列的那一截。
pub fn take(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for g in text.graphemes(true) {
        let w = g.width();
        if used + w > width {
            break;
        }
        used += w;
        out.push_str(g);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{soft, take};

    #[test]
    fn a_line_breaks_after_a_slash_or_a_space() {
        assert_eq!(
            soft("~/Documents/github/miyu-tui", 14),
            ["~/Documents/", "github/", "miyu-tui"]
        );
        assert_eq!(soft("晴 傍晚有雨", 6), ["晴", "傍晚有", "雨"]);
        assert_eq!(soft("a\n\nb", 5), ["a", "", "b"]);
        assert_eq!(
            soft("abcdefgh", 3),
            ["abc", "def", "gh"],
            "没有可断的照字折"
        );
    }

    #[test]
    fn take_keeps_what_fits() {
        assert_eq!(take("天气晴", 5), "天气");
        assert_eq!(take("ab", 5), "ab");
    }
}
