//! 行内：一段段带样子的字（`Piece`），和把它们按显示宽度折成行（蓝图 `tui.md`「她的回答：Markdown」第 2 条）。
//!
//! 折行按字素簇、按显示宽度；英文尽量在空格处断，一个词比一行还长时才从词中间断；中文哪里都能断。
//! 字里的 `\n` 是硬换行。

use ratatui::style::Style;
use ratatui::text::Span;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// 一段带样子的字；是链接的一部分时带着地址。
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    /// 字，可以带 `\n`。
    pub text: String,
    /// 样子。
    pub style: Style,
    /// 属于哪个链接。
    pub link: Option<String>,
}

impl Piece {
    /// 一段字。
    pub fn new(text: impl Into<String>, style: Style) -> Self {
        Self {
            text: text.into(),
            style,
            link: None,
        }
    }

    /// 一段属于链接的字。
    pub fn linked(text: impl Into<String>, style: Style, url: &str) -> Self {
        Self {
            text: text.into(),
            style,
            link: Some(url.to_string()),
        }
    }
}

/// 折好的一行：片段、链接落在第几列到第几列（从这一行的开头算）、是不是上一行折下来的。
#[derive(Debug, Clone, Default)]
pub struct Folded {
    /// 这一行的片段。
    pub spans: Vec<Span<'static>>,
    /// 链接：起列、止列（不含）、地址。
    pub links: Vec<(u16, u16, String)>,
    /// 这一行是上一行折下来的（软折行）。
    pub joined: bool,
}

/// 折到一半的那一行已经占了几列：前面几个片段一共几列、最后一个片段几列。合起来就是各片段宽度的和，
/// 每接一个字素簇不用把整行的宽度从头再算一遍（原来一行有多长就从头算多少遍，流式出字时这一项约占终端界面 CPU 的 7%）。
#[derive(Debug, Default, Clone, Copy)]
struct Cols {
    before: usize,
    last: usize,
}

/// 一个字素簇照 unicode-width 的规矩占几列：一个字节的（ASCII，连控制字符在内）一定是 1，不查表；别的照查。
fn cell_width(text: &str) -> usize {
    if text.len() == 1 { 1 } else { text.width() }
}

impl Folded {
    /// 接上一个字素簇，`cols` 跟着更新。片段的宽度照 unicode-width 对整个片段算（和 `Span::width` 一样）：在一个片段
    /// 后面接一个字节的字（换行除外）宽度正好多 1，因为它算出来不带上下文，不影响前面的字；接别的字照旧把这个片段重算，
    /// 阿拉伯文的连字、emoji 的组合这类和前一个字连着算的照旧算对。
    fn push(&mut self, text: &str, piece: &Piece, cols: &mut Cols) {
        let start = u16::try_from(cols.before + cols.last).unwrap_or(u16::MAX);
        let end = start.saturating_add(u16::try_from(cell_width(text)).unwrap_or(0));
        // 和前一段样子一样的接在一起，少几个片段；链接按列另记，不受影响。
        match self.spans.last_mut() {
            Some(last) if last.style == piece.style => {
                last.content.to_mut().push_str(text);
                cols.last = if text.len() == 1 && text != "\n" {
                    cols.last + 1
                } else {
                    last.width()
                };
            }
            _ => {
                cols.before += cols.last;
                cols.last = cell_width(text);
                self.spans.push(Span::styled(text.to_string(), piece.style));
            }
        }
        if let Some(url) = &piece.link {
            match self.links.last_mut() {
                Some((_, to, last)) if last == url && *to == start => *to = end,
                _ => self.links.push((start, end, url.clone())),
            }
        }
    }
}

/// 把片段按 `width` 列折成行：换行是硬换行，一段里照 `linebreak.rs` 的规则折（英文照空格断、避头尾）。
pub fn fold(pieces: &[Piece], width: u16) -> Vec<Folded> {
    let mut out = Vec::new();
    // 一段里的字素簇：（字、它属于哪个片段）。
    let mut para: Vec<(&str, usize)> = Vec::new();
    for (index, piece) in pieces.iter().enumerate() {
        for g in piece.text.graphemes(true) {
            if g == "\n" {
                lay(&mut out, &para, pieces, width);
                para.clear();
                continue;
            }
            para.push((g, index));
        }
    }
    lay(&mut out, &para, pieces, width);
    out
}

/// 一段折好放进 `out`：第一行接着上一段的硬换行，后面的是折下来的。
fn lay(out: &mut Vec<Folded>, para: &[(&str, usize)], pieces: &[Piece], width: u16) {
    let cells: Vec<&str> = para.iter().map(|(g, _)| *g).collect();
    for (k, range) in crate::linebreak::lines(&cells, usize::from(width.max(1)))
        .into_iter()
        .enumerate()
    {
        let mut line = Folded {
            joined: k > 0,
            ..Folded::default()
        };
        let mut cols = Cols::default();
        for &(g, index) in &para[range] {
            line.push(g, &pieces[index], &mut cols);
        }
        out.push(line);
    }
}

/// 一行里的字，拼起来。
pub fn plain(folded: &Folded) -> String {
    folded.spans.iter().map(|s| s.content.as_ref()).collect()
}

#[cfg(test)]
mod tests {
    use ratatui::style::Style;

    use super::{Piece, fold, plain};

    fn lines(text: &str, width: u16) -> Vec<(String, bool)> {
        fold(&[Piece::new(text, Style::new())], width)
            .iter()
            .map(|l| (plain(l), l.joined))
            .collect()
    }

    #[test]
    fn english_breaks_at_spaces_and_chinese_anywhere() {
        assert_eq!(
            lines("hello wonderful world", 12),
            vec![
                ("hello ".into(), false),
                ("wonderful ".into(), true),
                ("world".into(), true)
            ]
        );
        assert_eq!(
            lines("你好世界", 5),
            vec![("你好".into(), false), ("世界".into(), true)]
        );
    }

    #[test]
    fn a_full_stop_never_starts_a_line() {
        // 2026-09-30 项目主人：回答里 `。` 落在行首（蓝图「她的回答：Markdown」第 2 条）。
        let got = lines("会先睡十五秒再读文件。", 20);
        assert!(got.iter().all(|(l, _)| !l.starts_with('。')), "{got:?}");
    }

    #[test]
    fn a_word_longer_than_the_line_is_cut() {
        assert_eq!(
            lines("abcdefgh", 3),
            vec![
                ("abc".into(), false),
                ("def".into(), true),
                ("gh".into(), true)
            ]
        );
    }

    #[test]
    fn newlines_are_hard_breaks() {
        assert_eq!(
            lines("a\nb", 10),
            vec![("a".into(), false), ("b".into(), false)]
        );
    }

    /// 折好的一行：各片段的字、链接、是不是折下来的。
    type Line = (Vec<String>, Vec<(u16, u16, String)>, bool);

    /// 原来的做法：每接一个字素簇把整行各片段的宽度从头加一遍。
    fn slow_fold(pieces: &[Piece], width: u16) -> Vec<Line> {
        use ratatui::text::Span;
        use unicode_segmentation::UnicodeSegmentation;
        use unicode_width::UnicodeWidthStr;
        let mut out = Vec::new();
        let mut paras: Vec<Vec<(&str, usize)>> = vec![Vec::new()];
        for (index, piece) in pieces.iter().enumerate() {
            for g in piece.text.graphemes(true) {
                if g == "\n" {
                    paras.push(Vec::new());
                } else if let Some(last) = paras.last_mut() {
                    last.push((g, index));
                }
            }
        }
        for para in &paras {
            let cells: Vec<&str> = para.iter().map(|(g, _)| *g).collect();
            for (k, range) in crate::linebreak::lines(&cells, usize::from(width.max(1)))
                .into_iter()
                .enumerate()
            {
                let mut spans: Vec<Span<'static>> = Vec::new();
                let mut links: Vec<(u16, u16, String)> = Vec::new();
                for &(g, index) in &para[range] {
                    let piece = &pieces[index];
                    let used: usize = spans.iter().map(Span::width).sum();
                    let start = u16::try_from(used).unwrap_or(u16::MAX);
                    let end = start.saturating_add(u16::try_from(g.width()).unwrap_or(0));
                    match spans.last_mut() {
                        Some(last) if last.style == piece.style => {
                            last.content.to_mut().push_str(g)
                        }
                        _ => spans.push(Span::styled(g.to_string(), piece.style)),
                    }
                    if let Some(url) = &piece.link {
                        match links.last_mut() {
                            Some((_, to, last)) if last == url && *to == start => *to = end,
                            _ => links.push((start, end, url.clone())),
                        }
                    }
                }
                let texts = spans.iter().map(|s| s.content.to_string()).collect();
                out.push((texts, links, k > 0));
            }
        }
        out
    }

    #[test]
    fn keeping_a_running_width_folds_exactly_like_summing_the_line_every_time() {
        // BO-T2：折行时不再每接一个字就把整行的宽度重算一遍。中文、emoji（带 ZWJ、VS16、肤色）、国旗、阿拉伯文连字、
        // 回车换行、控制字符、ASCII 混在一起，几种样子、链接交错，几种宽度下折出来的每一行、每个链接的列和原来一样。
        use ratatui::style::{Color, Style};
        let red = Style::new().fg(Color::Red);
        let texts = [
            "hello 世界 👩‍💻 ok ❤️ 👍🏽 🇨🇳🇺🇸 لا مرحبا end",
            "\r\u{7}tab\tbell\u{1b}[0m a\r\nb ﾞﾟ x\u{301} e\u{301}",
            "长长长长长长长长长长长长长长长长长长长长长长长长长长长长 word",
            "⚠️ warn ☺︎ text １２３ ＡＢＣ 。、「引号」 done",
        ];
        for width in [1, 3, 7, 12, 20, 40, 120] {
            for text in texts {
                let mut pieces = Vec::new();
                for (i, part) in text.split(' ').enumerate() {
                    let piece = match i % 3 {
                        0 => Piece::new(format!("{part} "), Style::new()),
                        1 => Piece::new(format!("{part} "), red),
                        _ => Piece::linked(format!("{part} "), red, "https://a.b"),
                    };
                    pieces.push(piece);
                }
                pieces.push(Piece::new(text, Style::new()));
                let got: Vec<_> = super::fold(&pieces, width)
                    .into_iter()
                    .map(|f| {
                        let texts = f.spans.iter().map(|s| s.content.to_string()).collect();
                        (texts, f.links, f.joined)
                    })
                    .collect();
                assert_eq!(got, slow_fold(&pieces, width), "宽 {width}：{text:?}");
            }
        }
    }

    #[test]
    fn links_keep_their_columns_across_pieces() {
        let pieces = [
            Piece::new("see ", Style::new()),
            Piece::linked("docs", Style::new().bold(), "https://a.b"),
        ];
        let folded = fold(&pieces, 20);
        assert_eq!(folded[0].links, vec![(4, 8, "https://a.b".to_string())]);
    }
}
