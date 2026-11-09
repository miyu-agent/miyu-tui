//! 侧边栏（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 7 条）：窗口够宽时右边一栏，吉祥物在上，会话那几段在下（待办 2026-10-26
//! 起在输入框上面）；
//! 和主列之间一根暗色竖线。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::layout::slot::{Part, stack};
use crate::app::{MascotState, Screen, Seen};
use crate::config::{Config, Layout};
use crate::core::Usage;
use crate::side_select::TextRow;
use crate::transcript::Transcript;

mod parts;
use crate::{meter, theme};

/// 分出主列和侧边栏（侧边栏不含竖线）；窗口窄时侧边栏宽是 0。
pub fn split(area: Rect, layout: &Layout) -> (Rect, Rect) {
    if area.width < layout.sidebar_from {
        return (area, Rect::new(area.right(), area.y, 0, area.height));
    }
    let side = layout.sidebar_width;
    let main = Rect::new(area.x, area.y, area.width - side - 1, area.height);
    let sidebar = Rect::new(area.right() - side, area.y, side, area.height);
    (main, sidebar)
}

/// 侧边栏那几段的小标题加粗、内容缩进两格（`tui.md`「后台命令、子代理和侧边栏」第 7 条）；一段之间的空行由挂载位排
/// （「挂载位」第 4 条）。
fn item(s: String, style: Style) -> Line<'static> {
    Line::styled(format!("  {s}"), style)
}

/// `session`：会话名当标题（没起名字的也一样蓝），下面短编号，再下面人格、预设各一行（「新会话：人格、工作区」第 3、6 条）。
/// 一并交回短编号在第几行（点它复制完整编号）。
pub(super) fn session_lines(
    t: &Transcript,
    config: &Config,
    width: u16,
    picks: Vec<String>,
) -> (Vec<Line<'static>>, Option<usize>) {
    let text = &config.text;
    let name = t.title.as_deref().unwrap_or(&text.untitled);
    let mut out = vec![Line::styled(
        super::rows::clip(name, width),
        theme::side_title(),
    )];
    let mut id_row = None;
    if let Some(session) = &t.session {
        // 短编号：最后 8 个字符（核心 C-1 定的，头显示的、她看到的都是这个）。
        let short = crate::session_list::short(session);
        id_row = Some(out.len());
        out.push(item(text.side_id.replace("{id}", &short), theme::dim()));
        out.extend(picks.into_iter().map(|label| item(label, theme::dim())));
    }
    (out, id_row)
}

/// `cwd`：工作目录，照斜杠折行。
pub(super) fn cwd_lines(config: &Config, width: u16, cwd: &str) -> Vec<Line<'static>> {
    let room = usize::from(width).saturating_sub(2).max(1);
    let mut out = vec![Line::styled(
        config.text.side_cwd.clone(),
        theme::side_title(),
    )];
    out.extend(
        wrap_path(cwd, room)
            .into_iter()
            .map(|l| item(l, theme::dim())),
    );
    out
}

/// `context`：上下文用了多少、进度条、压缩过几次；还是 0 的不写。
pub(super) fn context_lines(t: &Transcript, config: &Config, width: u16) -> Vec<Line<'static>> {
    if t.context == 0 {
        return Vec::new();
    }
    let text = &config.text;
    let room = usize::from(width).saturating_sub(2).max(1);
    let used = meter::short(t.context);
    let mut out = vec![Line::styled(text.side_context.clone(), theme::side_title())];
    // 窗口照核心在订阅的回应里给的（`tui.md` 第 7 条），头不自己照模型名查。
    match t.limits.window {
        Some(window) => {
            let value = text
                .side_context_value
                .replace("{used}", &used)
                .replace("{window}", &meter::short(window))
                .replace("{percent}", &meter::percent_tenths(t.context, window));
            out.push(item(value, theme::dim()));
            let marks = &config.layout.bar;
            let line = t.limits.compaction_line;
            out.push(bar(t.context, window, line, marks.width.min(room), marks));
        }
        None => out.push(item(used, theme::dim())),
    }
    if t.cache.compactions > 0 {
        let compactions = text
            .side_compactions
            .replace("{n}", &t.cache.compactions.to_string());
        out.push(item(compactions, theme::dim()));
    }
    out
}

/// `usage`：用量连同子代理用的（`App::usage_total`，2026-09-30 项目主人）；还是 0 的不写。
pub(super) fn usage_lines(t: &Transcript, total: &Usage, config: &Config) -> Vec<Line<'static>> {
    let text = &config.text;
    let input = total.input();
    if input + total.output + total.aux == 0 {
        return Vec::new();
    }
    let mut out = vec![Line::styled(text.side_usage.clone(), theme::side_title())];
    // 共多少连同回顾这类辅助请求；输入输出、命中率只照主对话（蓝图「回顾」第 5 条）。
    let sum = meter::short(input + total.output + total.aux);
    out.push(item(
        text.side_total.replace("{tokens}", &sum),
        theme::dim(),
    ));
    let split = text
        .side_split
        .replace("{input}", &meter::short(input))
        .replace("{output}", &meter::short(total.output));
    out.push(item(split, theme::dim()));
    let hit = meter::hit_rate(total.cache_read, input);
    out.push(item(text.side_hit.replace("{percent}", &hit), theme::dim()));
    if t.cache.breaks > 0 {
        let breaks = text.side_breaks.replace("{n}", &t.cache.breaks.to_string());
        out.push(item(breaks, theme::dim()));
    }
    out
}

/// 花了多少：「花费 $0.42 + ¥1.30」；一笔都算不出的不写。没有价格的请求不提（2026-10-10 项目主人：「没有费用记录的请求
/// 不需要出现在侧边栏中」）。
pub(super) fn spent_lines(amounts: Option<String>, config: &Config) -> Vec<Line<'static>> {
    amounts
        .map(|amounts| {
            let line = config.text.side_cost.replace("{amounts}", &amounts);
            Line::styled(format!("  {line}"), theme::dim())
        })
        .into_iter()
        .collect()
}

/// 上下文那根进度条：用了的强调色、没用的暗；压缩线落在的那一格换成警示色（黄），字不变（2026-09-30 项目主人定）。
/// `line` 是核心给的压缩线，没给的不标。
pub(super) fn bar(
    used: u64,
    window: u64,
    line: Option<u64>,
    room: usize,
    marks: &crate::config::Bar,
) -> Line<'static> {
    // 用过就至少亮一格，不然用得少时看不出有这根条。
    let full = if window == 0 || used == 0 {
        0
    } else {
        (((used as f64 / window as f64) * room as f64).round() as usize).max(1)
    }
    .min(room);
    let mark = line
        .filter(|_| window > 0 && room > 0)
        .map(|l| (((l as f64 / window as f64) * room as f64).floor() as usize).min(room - 1));
    let mut spans = vec![Span::raw("  ")];
    for i in 0..room {
        let (glyph, style) = if i < full {
            (&marks.full, theme::accent())
        } else {
            (&marks.empty, theme::dim())
        };
        let style = if mark == Some(i) {
            theme::warn()
        } else {
            style
        };
        spans.push(Span::styled(glyph.clone(), style));
    }
    Line::from(spans)
}

/// 路径折行：一行尽量多放，断在这一行最后一个 `/` 后面；一行里没有 `/`（一级目录本身就放不下）的按字折。
pub(super) fn wrap_path(path: &str, room: usize) -> Vec<String> {
    let mut lines = Vec::new();
    // 照字素簇算（review，2026-10-11：原来逐字符算，`⚠️` 这类字算窄了）。
    let mut rest: Vec<&str> = path.graphemes(true).collect();
    while !rest.is_empty() {
        // 放得进这一行的有几个字。
        let mut used = 0;
        let mut fit = 0;
        for g in &rest {
            let w = g.width();
            if used + w > room {
                break;
            }
            used += w;
            fit += 1;
        }
        if fit == rest.len() {
            lines.push(rest.concat());
            break;
        }
        // 放不完：断在最后一个 `/` 后面（打头的那个不算，不然留下一个孤零零的 `/`）。
        let cut = rest[..fit]
            .iter()
            .rposition(|g| *g == "/")
            .filter(|at| *at > 0)
            .map_or(fit.max(1), |at| at + 1);
        lines.push(rest[..cut].concat());
        rest.drain(..cut);
    }
    lines
}

/// 画侧边栏：竖线；挂着的几件照「挂载位」排（第 4 条，一件之间空一行），包的界面块接在最后（「界面块」第 2 条）。
pub fn draw(
    frame: &mut Frame,
    area: Rect,
    seen: &Seen,
    screen: &mut Screen,
    pet: &mut MascotState,
) {
    screen.side_short = false;
    if area.width == 0 {
        return;
    }
    let line: Vec<Line> = (0..area.height)
        .map(|_| Line::styled(crate::sheet::mark(|m| &m.side), theme::dim()))
        .collect();
    frame.render_widget(
        Paragraph::new(line),
        Rect::new(area.x - 1, area.y, 1, area.height),
    );
    let inner = area.width.saturating_sub(2);
    // 这一帧没画到（藏起来、放不下）的件不留点击区。
    screen.areas.session_id = Rect::default();
    screen.areas.recent = Rect::default();
    screen.recent.clear();
    screen.areas.jumps = Rect::default();
    screen.jumps.clear();
    let body = Rect::new(area.x + 1, area.y + 1, inner, area.height.saturating_sub(1));
    let (
        mut avatar,
        mut session,
        mut cwd,
        mut context,
        mut usage,
        mut loops,
        mut recent,
        mut jumps,
    ) = parts::all();
    let top = screen.side_top;
    let stacked = {
        let mut side = parts::Side {
            seen,
            screen: &mut *screen,
            pet,
        };
        let mut all: [&mut dyn Part<parts::Side>; 8] = [
            &mut avatar,
            &mut session,
            &mut cwd,
            &mut context,
            &mut usage,
            &mut loops,
            &mut jumps,
            &mut recent,
        ];
        stack(
            frame,
            body,
            &mut all,
            (&seen.config.layout.side, 1, top),
            &mut side,
        )
    };
    // 滚到哪照排好的限过（排期第 14 条：像正文那样滚，滚轮在 `app/mouse.rs`）。
    screen.side_top = stacked.top;
    // 跳转消息列不满（老会话只读了最新一页）：接着要更早的一页（发版前 check 第 33 条，`app/pages.rs`）；藏起来、没画的不要。
    screen.side_short = jumps.short;
    // 放不下时指针停在侧边栏上，最右边那一列画滚动条（同正文的，「正文」第 1a 条）：看得出还有、滚到了哪。
    if screen.pointer.is_some_and(|at| area.contains(at)) {
        let x = area.right().saturating_sub(1);
        super::body::bar_at(
            frame,
            (x, body.y, body.height),
            crate::scrollbar::Bar::new(stacked.total, body.height, usize::from(stacked.top)),
        );
    }
    // 写了字的行记下来，给侧边栏的选字用（第 7 条）。
    let mut text_rows: Vec<TextRow> = stacked
        .written
        .into_iter()
        .map(|(y, x, text)| TextRow { y, x, text })
        .collect();
    // 扩展包放的界面块：接在后面（蓝图「界面块」第 2 条）。
    let after = stacked.next;
    let rest = Rect::new(
        area.x + 1,
        after,
        inner,
        area.bottom().saturating_sub(after),
    );
    let pointer = screen.pointer;
    let rows = super::widget::sidebar(frame, rest, seen, pointer, &mut screen.widget_spots);
    text_rows.extend(
        rows.into_iter()
            .map(|(y, text)| TextRow { y, x: rest.x, text }),
    );
    // 选中的反色。
    for row in &text_rows {
        if let Some((from, to)) = screen.side_select.cols(row.y) {
            let cells = Rect::new(from, row.y, to - from, 1).intersection(area);
            frame
                .buffer_mut()
                .set_style(cells, Style::new().add_modifier(Modifier::REVERSED));
        }
    }
    screen.side_select.set_rows(text_rows);
}

#[cfg(test)]
mod tests;
