//! 界面块的画法（蓝图 `tui.md`「界面块」第 3、5、6 条）：一块排成几行——标题一行，接着六种段一段段排；首页、侧边栏有
//! 行数上限，长了照第 5 条收：`list` 照待办收，`text` 截掉加 `…`，别的段整段放不下的连同后面的都不画。能点的记下在哪一行、
//! 哪几列、点了做什么，画的地方照它记下点击区。

mod list;
mod page;
mod place;
mod segment;
#[cfg(test)]
mod tests;
mod wrap;

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use serde::Deserialize;

pub use page::{PageFrame, draw as page};
pub use place::{Kind, Spot, draw_home, home, home_rows, sidebar, status};

use crate::config::{Bar, WidgetMarks};
use crate::theme;
use crate::widgets::{Action, Block, Segment, Tone};

/// 界面块上的字（`text/<语言>.json` 的 `widget`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Texts {
    /// 列表里做完的收成的那一行，`{count}`。
    pub folded: String,
    /// 列表后面放不下的收成的那一行，`{count}`。
    pub more: String,
    /// 侧边栏后面放不下的块收成的那一行，`{count}`。
    pub more_blocks: String,
    /// 整页：程序在跑、还没推来。
    pub loading: String,
    /// 整页：程序没在跑，`{source}` 是软件名。
    pub not_running: String,
    /// 整页最下面的按键提示：一项是键、做什么。
    pub page_keys: Vec<[String; 2]>,
}

/// 画在哪：侧边栏里内容缩进两格、普通的字照暗（自带的内容一律暗），小标题蓝色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Home,
    Sidebar,
    Page,
}

/// 画一块要的。
pub struct Look<'a> {
    /// 能用几列（侧边栏的连缩进）。
    pub width: u16,
    pub mode: Mode,
    pub marks: &'a WidgetMarks,
    pub bar: &'a Bar,
    pub texts: &'a Texts,
}

impl Look<'_> {
    /// 内容缩进几格。
    fn indent(&self) -> u16 {
        if self.mode == Mode::Sidebar { 2 } else { 0 }
    }

    /// 内容那一截有几列。
    fn room(&self) -> u16 {
        self.width.saturating_sub(self.indent()).max(1)
    }

    /// 语气的样子；侧边栏里普通的照暗。
    fn tone(&self, tone: Tone) -> Style {
        match tone {
            Tone::Normal if self.mode == Mode::Sidebar => theme::dim(),
            Tone::Normal => Style::new(),
            Tone::Dim => theme::dim(),
            Tone::Accent => theme::accent(),
            Tone::Good => theme::good(),
            Tone::Warn => theme::warn(),
            Tone::Bad => theme::error(),
        }
    }
}

/// 能点的一处：排好的第几行、从第几列到第几列（不含），点了做什么。
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub row: usize,
    pub from: u16,
    pub to: u16,
    pub action: Action,
}

/// 排好的一块。
#[derive(Debug, Default)]
pub struct Drawn {
    pub lines: Vec<Line<'static>>,
    pub hits: Vec<Hit>,
    /// 有没放下的：点了能展开（第 5 条）。
    pub cut: bool,
}

/// 标题那一行：标题后面暗色接 ` · 软件名`（第 3 条）；侧边栏的标题蓝色，同自带的小标题，首页的暗色。
pub fn title(block: &Block, look: &Look) -> Line<'static> {
    let (title, source) = block.heading();
    let lead = if look.mode == Mode::Sidebar {
        theme::side_title()
    } else {
        theme::dim()
    };
    let spans = match title {
        Some(title) => vec![
            Span::styled(title.to_string(), lead),
            Span::styled(format!(" · {source}"), theme::dim()),
        ],
        None => vec![Span::styled(source.to_string(), lead)],
    };
    clip_line(Line::from(spans), look.width)
}

/// 排一块：标题一行，接着内容；`rows` 是最多几行（连标题），`None` 是不限（点开了全部、整页）。
pub fn block(block: &Block, look: &Look, rows: Option<usize>) -> Drawn {
    let mut drawn = content(
        block.content.as_deref().unwrap_or_default(),
        look,
        rows.map(|r| r.saturating_sub(1)),
    );
    drawn.lines.insert(0, title(block, look));
    for hit in &mut drawn.hits {
        hit.row += 1;
    }
    drawn
}

/// 排内容：一段段往下排，`rows` 用完了照第 5 条收。
pub fn content(segments: &[Segment], look: &Look, rows: Option<usize>) -> Drawn {
    let mut out = Drawn::default();
    let indent = look.indent();
    for segment in segments {
        let left = rows.map(|r| r.saturating_sub(out.lines.len()));
        if left == Some(0) {
            out.cut = true;
            break;
        }
        let mut piece = segment::lay(segment, look, left);
        let fits = left.is_none_or(|l| piece.lines.len() <= l);
        if !fits {
            // 别的段整段放不下：不画，后面的也不画（第 5 条）。
            out.cut = true;
            break;
        }
        let at = out.lines.len();
        for hit in &mut piece.hits {
            hit.row += at;
            hit.from += indent;
            hit.to += indent;
        }
        let pad = " ".repeat(usize::from(indent));
        out.lines.extend(piece.lines.into_iter().map(|line| {
            if pad.is_empty() {
                return line;
            }
            // 整行的样子（`Line::styled` 记在行上）跟着带过去，不然缩进以后颜色丢了。
            let style = line.style;
            let mut spans = vec![Span::raw(pad.clone())];
            spans.extend(line.spans.into_iter().map(|s| s.patch_style(style)));
            Line::from(spans)
        }));
        out.hits.extend(piece.hits);
        if piece.cut {
            out.cut = true;
            break;
        }
    }
    out
}

/// 一行放不下 `width` 列：截掉加 `…`。
fn clip_line(line: Line<'static>, width: u16) -> Line<'static> {
    let width = usize::from(width);
    if line.width() <= width {
        return line;
    }
    let mut out = Vec::new();
    let mut used = 0;
    for span in line.spans {
        let room = width.saturating_sub(used + 1);
        let w = span.width();
        if w <= room {
            used += w;
            out.push(span);
            continue;
        }
        out.push(Span::styled(wrap::take(&span.content, room), span.style));
        break;
    }
    let style = out.last().map_or_else(Style::new, |s| s.style);
    out.push(Span::styled("…", style));
    Line::from(out)
}
