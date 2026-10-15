//! 界面块放进各个位置（蓝图「界面块」第 2、4 条）：侧边栏接在自带的几段后面，首页接在工作目录下面，底栏一块一格。每个
//! 位置的上限照 `layout.json` 的 `widgets`；放好的能点的地方记成 [`Spot`]，鼠标照它分。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use super::{Drawn, Look, Mode, block};
use crate::app::Seen;
use crate::theme;
use crate::widgets::{Action, Block, Place, Segment};

/// 一处能点的：哪个包的哪一块、在屏上哪儿、点了做什么。
#[derive(Debug, Clone, PartialEq)]
pub struct Spot {
    pub rect: Rect,
    pub package: String,
    pub id: String,
    pub what: Kind,
}

/// 点了做什么。
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// 列表的一条、一个按钮。
    Act(Action),
    /// 块里别的地方：展开全部、再点收起（第 5 条）。
    Toggle,
    /// 底栏的一格：悬停时说是哪个软件放的（第 3 条）。
    Status,
}

impl Spot {
    /// 指针放上去变成手：能做事的、能展开收起的。
    pub fn clickable(&self) -> bool {
        !matches!(self.what, Kind::Status)
    }
}

/// 照位置排一块要的。
fn look<'a>(seen: &'a Seen, mode: Mode, width: u16) -> Look<'a> {
    Look {
        width,
        mode,
        marks: &seen.config.layout.widgets.marks,
        bar: &seen.config.layout.bar,
        texts: &seen.config.text.widget,
    }
}

/// 画排好的一块：从 `at` 起往下，记下能点的；指针在能做事的那一处上时铺悬停的底色。交回画了几行。
fn put(
    frame: &mut Frame,
    at: Rect,
    (b, drawn): (&Block, &Drawn),
    pointer: Option<ratatui::layout::Position>,
    spots: &mut Vec<Spot>,
) -> u16 {
    let tall = u16::try_from(drawn.lines.len())
        .unwrap_or(u16::MAX)
        .min(at.height);
    let area = Rect { height: tall, ..at };
    frame.render_widget(Paragraph::new(drawn.lines.clone()), area);
    let spot = |rect: Rect, what: Kind| Spot {
        rect,
        package: b.package.clone(),
        id: b.id.clone(),
        what,
    };
    for hit in &drawn.hits {
        let Ok(row) = u16::try_from(hit.row) else {
            continue;
        };
        let rect = Rect::new(at.x + hit.from, at.y + row, hit.to - hit.from, 1).intersection(area);
        if rect.is_empty() {
            continue;
        }
        if pointer.is_some_and(|p| rect.contains(p)) {
            frame.buffer_mut().set_style(rect, theme::hover());
        }
        spots.push(spot(rect, Kind::Act(hit.action.clone())));
    }
    // 能展开收起的：整块（能做事的那几处先认，鼠标照先后找）。
    if drawn.cut {
        spots.push(spot(area, Kind::Toggle));
    }
    tall
}

/// 侧边栏：自带的几段后面，一块一段、之间空一行；一块最多 `sidebar_rows` 行，点开了的不限；剩下的连下一块的标题和一行
/// 内容都放不下就不画了，最后一行写 `… 还有 N 块`。交回画了字的每一行（给侧边栏的选字用）。
pub fn sidebar(
    frame: &mut Frame,
    area: Rect,
    seen: &Seen,
    pointer: Option<ratatui::layout::Position>,
    spots: &mut Vec<Spot>,
) -> Vec<(u16, String)> {
    let widgets = &seen.link.widgets;
    let blocks: Vec<&Block> = widgets.shown(Place::Sidebar).collect();
    let look = look(seen, Mode::Sidebar, area.width);
    let cap = seen.config.layout.widgets.sidebar_rows;
    let mut rows = Vec::new();
    let mut y = area.y;
    for (i, b) in blocks.iter().enumerate() {
        let left = usize::from(area.bottom().saturating_sub(y));
        // 后面还有块的，留一行写「还有 N 块」。
        let reserve = usize::from(i + 1 < blocks.len());
        let room = left.saturating_sub(reserve);
        if room < 2 {
            if left > 0 {
                let more = look
                    .texts
                    .more_blocks
                    .replace("{count}", &(blocks.len() - i).to_string());
                let at = Rect::new(area.x, y, area.width, 1);
                frame.render_widget(Paragraph::new(Line::styled(more.clone(), theme::dim())), at);
                rows.push((y, more));
            }
            break;
        }
        let rows_cap = if widgets.is_expanded(b) {
            room
        } else {
            cap.min(room)
        };
        let drawn = block(b, &look, Some(rows_cap));
        let at = Rect::new(area.x, y, area.width, area.bottom().saturating_sub(y));
        let tall = put(frame, at, (b, &drawn), pointer, spots);
        rows.extend(
            drawn
                .lines
                .iter()
                .take(usize::from(tall))
                .enumerate()
                .map(|(n, l)| (y + u16::try_from(n).unwrap_or(0), l.to_string())),
        );
        y = y.saturating_add(tall + 1);
    }
    rows
}

/// 首页要画的几块，照上限排好：最多 `home_blocks` 块，一块最多 `home_rows` 行，合起来连块之间的空行最多 `home_total` 行。
pub fn home(seen: &Seen, width: u16) -> Vec<(Block, Drawn)> {
    let limits = &seen.config.layout.widgets;
    let look = look(seen, Mode::Home, width);
    let mut used = 0;
    let mut out = Vec::new();
    for b in seen
        .link
        .widgets
        .shown(Place::Home)
        .take(limits.home_blocks)
    {
        let gap = usize::from(!out.is_empty());
        let room = limits.home_total.saturating_sub(used + gap);
        if room < 2 {
            break;
        }
        let rows = if seen.link.widgets.is_expanded(b) {
            room
        } else {
            limits.home_rows.min(room)
        };
        let drawn = block(b, &look, Some(rows));
        used += gap + drawn.lines.len();
        out.push((b.clone(), drawn));
    }
    out
}

/// 首页那几块一共几行（连块之间的空行）。
pub fn home_rows(blocks: &[(Block, Drawn)]) -> u16 {
    let lines: usize = blocks.iter().map(|(_, d)| d.lines.len()).sum();
    u16::try_from(lines + blocks.len().saturating_sub(1)).unwrap_or(u16::MAX)
}

/// 画首页的块：给的地方比要的矮时，前面的照画，最后放不下的那一块收到剩下的行（第 4 条：从最后一块往前收）。
pub fn draw_home(
    frame: &mut Frame,
    area: Rect,
    blocks: &[(Block, Drawn)],
    seen: &Seen,
    pointer: Option<ratatui::layout::Position>,
    spots: &mut Vec<Spot>,
) {
    let look = look(seen, Mode::Home, area.width);
    let mut y = area.y;
    for (b, drawn) in blocks {
        let left = usize::from(area.bottom().saturating_sub(y));
        if left < 2 {
            break;
        }
        let at = Rect::new(area.x, y, area.width, area.bottom() - y);
        let tall = if drawn.lines.len() <= left {
            put(frame, at, (b, drawn), pointer, spots)
        } else {
            let shrunk = block(b, &look, Some(left));
            put(frame, at, (b, &shrunk), pointer, spots)
        };
        y = y.saturating_add(tall + 1);
    }
}

/// 底栏的几格：最多 `status_cells` 格，一格是那一段字（一行，最多 `status_width` 列，长的截掉加 `…`），照语气着色。
pub fn status<'a>(seen: &Seen<'a>) -> Vec<(String, Style, &'a Block)> {
    let limits = &seen.config.layout.widgets;
    seen.link
        .widgets
        .shown(Place::Status)
        .filter_map(|b| {
            let Some(Segment::Text { text, tone }) = b.content.as_deref()?.first() else {
                return None;
            };
            let line = text.lines().next().unwrap_or_default();
            let look = look(seen, Mode::Home, 0);
            Some((
                crate::cells::clip(line, limits.status_width),
                look.tone(*tone),
                b,
            ))
        })
        .filter(|(text, _, _)| !text.is_empty())
        .take(limits.status_cells)
        .collect()
}
