//! 单行输入框比窗宽时横着滚（蓝图「配置页」第 15 条，审计 #084）：只露光标附近那一截，光标碰到左右边才挪、往回在框里
//! 不挪，同主输入框碰到上下边才滚（「输入框」第 3 条）。前面的名字、`/` 这些不跟着滚，两头不另加记号。

use ratatui::text::{Line, Span};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// 一行里的输入框从第几列起、光标在第几列、这一行能画几列，`left` 是上一帧框里露的那一截从字的第几列起。交回这一帧画的
/// 那一行、光标挪到第几列、露的那一截从第几列起（下一帧再交回来）。放得下的整行原样画。
pub fn slide(
    line: &Line<'static>,
    (from, caret): (u16, u16),
    width: u16,
    left: usize,
) -> (Line<'static>, u16, usize) {
    let room = usize::from(width.saturating_sub(from));
    let col = usize::from(caret.saturating_sub(from));
    if room == 0 {
        return (line.clone(), caret, 0);
    }
    let typed = line.width().saturating_sub(usize::from(from));
    // 删短了的退回能露全的位置：末尾光标那一格刚好在最右边。
    let mut left = left.min((typed + 1).saturating_sub(room));
    // 光标碰到左右边才挪。
    if col < left {
        left = col;
    } else if col >= left + room {
        left = col + 1 - room;
    }
    if left == 0 {
        return (line.clone(), caret, 0);
    }
    let lead = usize::from(from);
    let mut spans = Vec::new();
    let mut at = 0;
    for span in &line.spans {
        let mut kept = String::new();
        for g in span.content.graphemes(true) {
            let w = g.width();
            let start = at;
            at += w;
            if at <= lead {
                kept.push_str(g);
                continue;
            }
            let (s, e) = (start.saturating_sub(lead), at - lead);
            if s >= left && e <= left + room {
                kept.push_str(g);
            } else if s < left && e > left {
                // 宽字跨着左边：露出来的那半格写空白。
                kept.push_str(&" ".repeat(e - left));
            }
        }
        if !kept.is_empty() {
            spans.push(Span::styled(kept, span.style));
        }
    }
    let x = from + u16::try_from(col - left).unwrap_or(u16::MAX);
    (Line::from(spans).style(line.style), x, left)
}

#[cfg(test)]
mod tests {
    use super::slide;
    use ratatui::text::{Line, Span};

    fn plain(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    /// 「名字 」后面接 `text`，光标在字的第 `col` 列，这一行 12 列。
    fn field(text: &str, col: u16, left: usize) -> (String, u16, usize) {
        let line = Line::from(vec![Span::raw("名字 "), Span::raw(text.to_string())]);
        let (line, x, left) = slide(&line, (5, 5 + col), 12, left);
        (plain(&line), x, left)
    }

    #[test]
    fn a_short_value_is_drawn_as_it_is() {
        assert_eq!(field("abc", 3, 0), ("名字 abc".into(), 8, 0));
    }

    #[test]
    fn the_caret_pushes_the_view_only_at_the_edges() {
        // 框 7 列：打到第 10 个字，露最后 6 个字加光标那一格。
        assert_eq!(field("0123456789", 10, 0), ("名字 456789".into(), 11, 4));
        // 往回在框里不挪。
        assert_eq!(field("0123456789", 6, 4), ("名字 456789".into(), 7, 4));
        // 碰到左边才挪。
        assert_eq!(field("0123456789", 2, 4), ("名字 2345678".into(), 5, 2));
        // 删短了退回能露全的位置。
        assert_eq!(field("01234", 5, 4), ("名字 01234".into(), 10, 0));
    }

    #[test]
    fn a_wide_letter_cut_by_the_left_edge_leaves_a_blank() {
        // 「中文」两个宽字：露的那一截从字的第 3 列起，「文」只露右半格，写空白。
        assert_eq!(field("中文abcdefg", 9, 0), ("名字  abcdef".into(), 11, 3));
    }
}
