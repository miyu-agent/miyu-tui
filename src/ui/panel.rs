//! 贴在输入框上面的三样共用的样子（蓝图 `tui.md`「斜杠命令列表」第 3 条、「输入历史列表」第 1 条、后台面板）：
//! 圆角框，框线和输入框的边一个颜色（暗），左右和输入框的边对齐；标题嵌在上边框，按键提示嵌在下边框，边框上的字
//! 和框线一个颜色。
//! 一条一行，行首两格；选中的写 `❯ `，整行铺强调色的底、上面的字是深色（`picked_bar`）。放不下框（不到 3 行）的只画
//! 那几条。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::theme;

/// 面板排好的一行：是第几条（「没有对得上的」这类是 `None`；点开的一条下面几行都算它），和画出来的样子。
pub type Row = (Option<usize>, Line<'static>);

/// 框要几行才画得下：上下两条边，中间至少一条。
const FRAMED: u16 = 3;

/// 框的样子：上边框的标题（标题、条数这些），下边框的按键提示（没有的只是一条线）。
#[derive(Debug, Clone, Default)]
pub struct Chrome {
    /// 嵌在上边框的字。
    pub title: Vec<Span<'static>>,
    /// 嵌在下边框的按键提示。
    pub hint: Option<String>,
}

impl Chrome {
    /// 标题和框线一个颜色，`meta`（条数、搜的字）跟在后面。
    pub fn new(title: &str, meta: Vec<Span<'static>>) -> Self {
        let mut spans = vec![Span::styled(title.to_string(), theme::dim())];
        spans.extend(meta);
        Self {
            title: spans,
            hint: None,
        }
    }

    /// 下边框写这句按键提示。
    pub fn hint(mut self, hint: &str) -> Self {
        self.hint = Some(hint.to_string());
        self
    }

    /// 按键模式是 Vim（`on`）：按键提示里的「↑/↓」写成「j/k」。只给认 `j`、`k` 的框用，命令列表这些跟着输入框打字的
    /// 照旧。
    #[must_use]
    pub fn vim(mut self, on: bool) -> Self {
        if on && let Some(hint) = &mut self.hint {
            *hint = moves(hint, true);
        }
        self
    }
}

/// 按键提示里的「↑/↓」：按键模式是 Vim（`vim`）写成「j/k」（蓝图「按键」表「按键模式」那一行）。
pub fn moves(hint: &str, vim: bool) -> String {
    if vim {
        hint.replace("↑/↓", "j/k")
    } else {
        hint.to_string()
    }
}

/// 上边框里搜的字（`label` 是「 · 搜索：」）：打了字的写它；按键模式是 Vim 时按了 `/`、正在打的（`typing`）没打字也写，
/// 末尾一根强调色的光标。
pub fn searched(label: &str, query: &str, typing: bool) -> Vec<Span<'static>> {
    if query.is_empty() && !typing {
        return Vec::new();
    }
    let mut spans = vec![
        Span::styled(label.to_string(), theme::dim()),
        Span::styled(query.to_string(), Style::new().fg(Color::Reset)),
    ];
    if typing {
        spans.push(Span::styled("▏", theme::accent()));
    }
    spans
}

/// 给了 `outer` 这么高，框里放得下几条：画得下框的去掉上下两条边。
pub fn room(outer: u16) -> u16 {
    if outer >= FRAMED { outer - 2 } else { outer }
}

/// 放 `rows` 条要多高：连框（放得下的话，照 `outer` 给的最多多高算）。
pub fn height(rows: u16, outer: u16) -> u16 {
    if rows == 0 {
        0
    } else if outer >= FRAMED {
        rows + 2
    } else {
        rows
    }
}

/// 由命令打开的框里上下各空的那两行（2026-10-20 项目主人：先要 `/persona`，又定别的框也加；打字时弹的命令列表、`@` 文件列表、
/// 输入历史试过也加，嫌浪费地方，回滚）。
pub const PAD: u16 = 2;

/// 放字的那一块上下各让一行（[`PAD`]）：放不下的不让。
pub fn padded(text: Rect) -> Rect {
    if text.height <= PAD {
        return text;
    }
    Rect::new(text.x, text.y + 1, text.width, text.height - PAD)
}

/// 框里放字的那一块：画得下框的去掉上下两条边；左右照输入框里的字（`text_x`、`text_width`）。
pub fn inside(outer: Rect, text_x: u16, text_width: u16) -> Rect {
    let rows = room(outer.height);
    let top = outer.y + (outer.height - rows) / 2;
    Rect::new(text_x, top, text_width, rows)
}

/// 放进 `max` 条：从离选中那一条（`picked`）最远的起少露几条（「窗口小的时候」第 1 条）。
pub fn fit(mut rows: Vec<Row>, picked: Option<usize>, max: usize) -> Vec<Row> {
    while rows.len() > max {
        let anchor = rows
            .iter()
            .position(|(item, _)| item.is_some() && *item == picked)
            .unwrap_or(rows.len());
        let far = rows
            .iter()
            .enumerate()
            .filter(|(_, (item, _))| item.is_some() && *item != picked)
            .max_by_key(|(k, _)| k.abs_diff(anchor))
            .map(|(k, _)| k);
        let Some(far) = far else { break };
        rows.remove(far);
    }
    rows.truncate(max);
    rows
}

/// 画一样：先清掉底下的，画得下框的画框（`outer`），再把几条画进框里（`text`）。
pub fn draw(frame: &mut Frame, outer: Rect, text: Rect, chrome: Chrome, lines: Vec<Line<'static>>) {
    if lines.is_empty() || outer.height == 0 {
        return;
    }
    frame.render_widget(Clear, outer);
    if outer.height >= FRAMED {
        frame.render_widget(block(chrome), outer);
    }
    frame.render_widget(Paragraph::new(lines), text);
}

/// 圆角框：标题嵌在上边框 `╭─ 标题 ─…╮`，按键提示嵌在下边框 `╰─ 提示 ─…╯`。
fn block(chrome: Chrome) -> Block<'static> {
    let line = theme::dim();
    let edge = |mut spans: Vec<Span<'static>>| {
        spans.insert(0, Span::styled("─ ", line));
        spans.push(Span::styled(" ", line));
        Line::from(spans)
    };
    let mut block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(line);
    // 没有标题的（问要不要压缩那个框）只是一条线。
    if chrome.title.iter().any(|s| !s.content.is_empty()) {
        block = block.title_top(edge(chrome.title));
    }
    if let Some(hint) = chrome.hint {
        block = block.title_bottom(edge(vec![Span::styled(hint, line)]));
    }
    block
}

/// 一条：行首两格（选中的写 `❯ `），`content` 放不下截掉加 `…`，`right` 贴着右边；选中的铺强调色的底、字换深色。
pub fn item(
    picked: bool,
    content: Vec<Span<'static>>,
    right: Option<Span<'static>>,
    width: u16,
) -> Line<'static> {
    item_with(picked, None, content, right, width)
}

/// 同 [`item`]，没选中的那两格写 `lead`（两格宽，例如会话列表正在用的 `● `、在跑的转圈）：不另占一列，字和别的列表对齐
/// （2026-10-16 项目主人：会话列表左边空出一大块，和模型、历史列表不一样）。`lead` 写了 `force` 的选中了也照它写。
pub fn item_with(
    picked: bool,
    lead: Option<(Span<'static>, bool)>,
    content: Vec<Span<'static>>,
    right: Option<Span<'static>>,
    width: u16,
) -> Line<'static> {
    let mark = match lead {
        Some((lead, force)) if force || !picked => lead,
        _ => Span::raw(if picked { "❯ " } else { "  " }),
    };
    let width = usize::from(width);
    let right_width = right.as_ref().map_or(0, |r| r.width() + 1);
    let room = width.saturating_sub(2 + right_width);
    let mut spans = vec![mark];
    spans.extend(clip_spans(content, room));
    let used: usize = spans.iter().map(Span::width).sum();
    let tail = right.as_ref().map_or(0, Span::width);
    spans.push(Span::raw(" ".repeat(width.saturating_sub(used + tail))));
    spans.extend(right);
    bar(Line::from(spans), picked)
}

/// 现在用的那一项行首写的：强调色的 `mark`（`layout.json` 的 `current_mark`）接一个空格，选中了也照旧写它、不让给 `❯`
/// （2026-10-19 项目主人定各个框统一：右边不再写「当前」）。交给 [`item_with`] 的 `lead`。
pub fn current_lead(mark: &str, on: bool) -> Option<(Span<'static>, bool)> {
    on.then(|| (Span::styled(format!("{mark} "), theme::accent()), true))
}

/// 放进 `width` 列：放不下的从前面截掉、打头写 `…`（路径的末段最有用，首页那一行、工作区框用）。
pub fn tail(text: &str, width: usize) -> String {
    crate::cells::tail_marked(text, width)
}

/// 接着上一条往下写的一行（展开的全文）：行首空两格，和上一条的字对齐；选中的照 [`item`] 铺底。
pub fn more(picked: bool, content: Vec<Span<'static>>, width: u16) -> Line<'static> {
    let width = usize::from(width);
    let mut spans = vec![Span::raw("  ")];
    spans.extend(clip_spans(content, width.saturating_sub(2)));
    let used: usize = spans.iter().map(Span::width).sum();
    spans.push(Span::raw(" ".repeat(width.saturating_sub(used))));
    bar(Line::from(spans), picked)
}

/// 选中的：整行铺强调色的底，每一段的字换成深色、不要自己的底，加粗这类修饰留着（2026-09-30 项目主人：照 Cline）。
fn bar(line: Line<'static>, picked: bool) -> Line<'static> {
    if !picked {
        return line;
    }
    let dark = theme::picked_bar();
    let spans = line
        .spans
        .into_iter()
        .map(|span| {
            let keep = span.style.add_modifier & (Modifier::BOLD | Modifier::UNDERLINED);
            Span::styled(span.content, dark.add_modifier(keep))
        })
        .collect::<Vec<_>>();
    Line::from(spans).style(dark)
}

/// 这几段排到 `room` 列：放不下的截掉，最后一格写 `…`（样子照被截的那一段）。
pub fn clip_spans(spans: Vec<Span<'static>>, room: usize) -> Vec<Span<'static>> {
    let total: usize = spans.iter().map(Span::width).sum();
    if total <= room {
        return spans;
    }
    let mut out = Vec::new();
    let mut used = 0;
    for span in spans {
        let mut kept = String::new();
        for g in span.content.graphemes(true) {
            let w = g.width();
            if used + w + 1 > room {
                break;
            }
            kept.push_str(g);
            used += w;
        }
        let cut = kept.len() < span.content.len();
        if !kept.is_empty() {
            out.push(Span::styled(kept, span.style));
        }
        if cut {
            if room > 0 {
                out.push(Span::styled("…", span.style));
            }
            return out;
        }
    }
    out
}

#[cfg(test)]
mod tests;
