//! 输入历史列表（蓝图 `tui.md`「输入历史列表」）：和命令列表、后台面板一个框（`panel.rs`），上边框写标题和条数，
//! 下边框写按键提示。框里从上往下三段（2026-11-03 照 Claude Code 改版，排期第 17 条）：对得上的几条（最新的贴着底，
//! 越早越往上；选中的停在正中间，到头才往边上走，照命令列表的 `window`；每条左边写多久以前、右边一行预览），选中那一条的
//! 全文，搜索那一行。
//!
//! 列表排成哪几行、每一行是哪一条，都由 [`lines`] 定：占几行、画什么、鼠标点的是哪一条，照同一份。

use std::time::SystemTime;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::panel::{self, Chrome};
use crate::config::{Config, HistoryTexts};
use crate::history::History;
use crate::input::Sent;
use crate::theme;
use unicode_width::UnicodeWidthStr;

mod preview;
mod shape;
mod side;

pub use shape::list_rows;

/// 排好的一行：是对得上的第几条（「没有对得上的」、空着的、横线、搜索那一行是 `None`；预览那几行算选中的那一条），和画出来的样子。
pub type Row = panel::Row;

/// 列表至少留几条（地方不够时先缩预览，「输入历史列表」第 1 条）。
const KEEP: usize = 3;

/// 框排在多大的地方：框里能写几列（`width`）、最多几行（`max`，输入框上面剩下的，「窗口小的时候」第 1 条）、窗口几行
/// （`screen`）、全部历史几条（`total`，不是对得上的几条：筛少了框不缩，第 52 条）。
#[derive(Debug, Clone, Copy)]
pub struct Room {
    pub width: u16,
    pub max: usize,
    pub screen: u16,
    pub total: usize,
}

/// 预览在右边时（第 52 条）：竖线在框里第几列、列表那一段几行、点右边算哪一条。
#[derive(Debug, Clone, Copy, Default)]
pub struct Side {
    pub col: u16,
    pub rows: usize,
    pub picked: Option<usize>,
}

/// 排好的框：边框上的字、框里从上往下的每一行、预览在不在右边。
#[derive(Debug, Clone, Default)]
pub struct Laid {
    pub chrome: Chrome,
    pub rows: Vec<Row>,
    pub side: Option<Side>,
}

/// 列表的框和框里从上往下的每一行。`matches` 是对得上的几条，最新的在前；`now` 算多久以前。
pub fn lines(
    history: &History,
    matches: &[&Sent],
    room: Room,
    config: &Config,
    now: SystemTime,
) -> Laid {
    let words = words(history, config);
    let count = words.count.replace("{count}", &matches.len().to_string());
    // 按键模式是 Vim 的照会话列表写：平时「/ 搜索 · j/k 选择」，按了 `/` 写打字的那一句（发版前 check 第 31 条）。
    let vim = config.layout.vim;
    let hint = crate::list_search::hint(
        vim,
        history.typing,
        &words.hints,
        &words.hints_vim,
        &words.searching,
    );
    let chrome = Chrome::new(&words.title, vec![Span::styled(count, theme::dim())]).hint(hint);
    let width = room.width;
    let rows = list_rows(&config.layout, room.total, width, room.screen);
    let side = shape::side(width);
    let list_width = if side {
        shape::left_width(width)
    } else {
        width
    };
    let list = items(history, matches, rows, list_width, now, words);
    let caret = !vim || history.typing;
    let search = (None, search(&history.query, caret, words, width));
    let picked = matches.get(history.selected).copied();
    let laid = |rows, side| Laid {
        chrome: chrome.clone(),
        rows,
        side,
    };
    if room.max < KEEP {
        return laid(panel::fit(list, Some(history.selected), room.max), None);
    }
    // 搜索那一行和它上面的横线一直在；剩下的先给列表留够，再给预览。
    let space = room.max - 2;
    let mut out = if side {
        let shown = rows.min(space);
        let left = column(list, history.selected, shown, list_width);
        let cols = width.saturating_sub(list_width + 2).max(1);
        side::join(left, picked, cols, words)
    } else {
        // 上下排：列表先缩到至少三条，再缩预览（连它上下两条圆角框的边），露不出一行字就不画、地方还给列表。
        let boxed = shape::preview_rows(&config.layout) + 2;
        let preview_room = space.saturating_sub(rows.min(KEEP)).min(boxed);
        let (shown, tall) = if preview_room >= 3 {
            (rows.min(space - preview_room), preview_room - 2)
        } else {
            (rows.min(space), 0)
        };
        let mut out = column(list, history.selected, shown, list_width);
        if tall > 0 {
            let tag = picked.map(|_| history.selected);
            out.extend(
                preview::boxed(picked, width, tall, words)
                    .into_iter()
                    .map(|line| (tag, line)),
            );
        }
        out
    };
    let side = side.then(|| Side {
        col: list_width,
        rows: out.len(),
        picked: picked.map(|_| history.selected),
    });
    out.push((None, preview::rule(width)));
    out.push(search);
    laid(out, side)
}

/// 列表那几条（露的是从 `menu::top` 起的 `rows` 条），最新的在后；一条都对不上时一行「没有对得上的」。
fn items(
    history: &History,
    matches: &[&Sent],
    rows: usize,
    width: u16,
    now: SystemTime,
    words: &HistoryTexts,
) -> Vec<Row> {
    let top = crate::menu::top(history.selected, history.pinned, matches.len(), rows);
    let end = (top + rows).min(matches.len());
    let mut list: Vec<Row> = Vec::new();
    if matches.is_empty() {
        let empty = vec![Span::styled(words.empty.clone(), theme::faint())];
        list.push((None, panel::item(false, empty, None, width)));
    }
    let agos: Vec<String> = (top..end).map(|i| ago(matches[i].at, now, words)).collect();
    let column = agos.iter().map(|a| a.width()).max().unwrap_or(0);
    for i in (top..end).rev() {
        let picked = i == history.selected;
        let ago = &agos[i - top];
        let pad = " ".repeat(column - ago.width() + 1);
        let mut spans = vec![Span::styled(format!("{ago}{pad}"), theme::faint())];
        spans.extend(content(matches[i], &history.query, picked, words));
        list.push((Some(i), panel::item(picked, spans, None, width)));
    }
    list
}

/// 列表那一段排成 `shown` 行：放不下的从离选中最远的起少露，少了的在上面垫空行（最新的照旧贴着底，框不缩，第 52 条）。
fn column(list: Vec<Row>, selected: usize, shown: usize, width: u16) -> Vec<Row> {
    let list = panel::fit(list, Some(selected), shown);
    let blank = (None, Line::raw(" ".repeat(usize::from(width))));
    let mut out = vec![blank; shown - list.len()];
    out.extend(list);
    out
}

/// 搜索那一行：打的字原色、后面一根强调色的光标；没打字时光标后面暗色写「筛选历史…」。按键模式是 Vim、没按 `/` 的
/// 不画光标（这时打的字不进搜索）。
fn search(query: &str, caret: bool, words: &HistoryTexts, width: u16) -> Line<'static> {
    let caret = caret
        .then(|| Span::styled(crate::sheet::mark(|m| &m.caret), theme::accent()))
        .into_iter();
    let spans: Vec<Span<'static>> = if query.is_empty() {
        caret
            .chain([Span::styled(words.placeholder.clone(), theme::faint())])
            .collect()
    } else {
        [Span::styled(
            query.to_string(),
            Style::new().fg(ratatui::style::Color::Reset),
        )]
        .into_iter()
        .chain(caret)
        .collect()
    };
    panel::more(false, spans, width)
}

/// 列表上的字：输入历史的，或者 `/jump` 的（「跳到说过的话」第 1 条）。
fn words<'a>(history: &History, config: &'a Config) -> &'a HistoryTexts {
    if history.jump {
        &config.text.jump
    } else {
        &config.text.history
    }
}

/// 多久以前发的：一分钟以内「刚才」，再往后几分钟、几小时、几天（重启以后还在，第 8 条）。
fn ago(at: SystemTime, now: SystemTime, words: &HistoryTexts) -> String {
    let secs = now.duration_since(at).unwrap_or_default().as_secs();
    if secs < 60 {
        words.now.clone()
    } else if secs < 3600 {
        words.minutes.replace("{n}", &(secs / 60).to_string())
    } else if secs < 86_400 {
        words.hours.replace("{n}", &(secs / 3600).to_string())
    } else {
        words.days.replace("{n}", &(secs / 86_400).to_string())
    }
}

/// 一行的字：只写第一行，后面还有的暗色跟「 · +N 行」；命令名强调色，粘贴块照输入框里的样子，搜到的字标出来。
fn content(sent: &Sent, query: &str, picked: bool, words: &HistoryTexts) -> Vec<Span<'static>> {
    let text = sent.draft.text.trim_end_matches('\n');
    let first = text.split('\n').next().unwrap_or_default();
    let extra = text.split('\n').count() - 1;
    // 没选中的字暗、命令名强调色；选中的颜色由选中的那一条的底色定（`panel::item`）。
    let base = if picked {
        Style::new().add_modifier(Modifier::BOLD)
    } else {
        theme::dim()
    };
    let mut styles = vec![base; first.len()];
    if first.starts_with('/') {
        let end = first.find(char::is_whitespace).unwrap_or(first.len());
        paint(&mut styles, 0, end, |s| s.patch(theme::accent()));
    }
    for block in sent.draft.blocks.iter().filter(|b| b.end <= first.len()) {
        paint(&mut styles, block.start, block.end, |_| theme::chip());
    }
    let hit = theme::accent().add_modifier(Modifier::UNDERLINED);
    for (start, end) in hits(first, query) {
        paint(&mut styles, start, end, |s| s.patch(hit));
    }
    let mut spans = group(first, &styles);
    if extra > 0 {
        let more = words.lines.replace("{count}", &extra.to_string());
        spans.push(Span::styled(more, theme::dim()));
    }
    spans
}

/// `[start, end)` 这几个字节的样子照 `f` 改。
fn paint(styles: &mut [Style], start: usize, end: usize, f: impl Fn(Style) -> Style) {
    let end = end.min(styles.len());
    for style in &mut styles[start..end] {
        *style = f(*style);
    }
}

/// 连着一样样子的字并成一段。
fn group(text: &str, styles: &[Style]) -> Vec<Span<'static>> {
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut from = 0;
    for (at, _) in text.char_indices().skip(1) {
        if styles[at] != styles[from] {
            out.push(Span::styled(text[from..at].to_string(), styles[from]));
            from = at;
        }
    }
    if from < text.len() {
        out.push(Span::styled(text[from..].to_string(), styles[from]));
    }
    out
}

/// `query` 在 `text` 里出现的地方（字节范围），不分大小写，一个字一个字比。
fn hits(text: &str, query: &str) -> Vec<(usize, usize)> {
    let lower = |c: char| c.to_lowercase().next().unwrap_or(c);
    let wanted: Vec<char> = query.chars().map(lower).collect();
    if wanted.is_empty() {
        return Vec::new();
    }
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i + wanted.len() <= chars.len() {
        let same = chars[i..i + wanted.len()]
            .iter()
            .zip(&wanted)
            .all(|((_, c), w)| lower(*c) == *w);
        if same {
            let end = chars
                .get(i + wanted.len())
                .map_or(text.len(), |(at, _)| *at);
            out.push((chars[i].0, end));
            i += wanted.len();
        } else {
            i += 1;
        }
    }
    out
}

/// 画列表：`outer` 连框，`text` 是框里放字的那一块。横线接上左右的框线（`├─…─┤`）；预览在右边的，竖线接上上边框（`┬`）
/// 和横线（`┴`）。
pub fn draw(frame: &mut Frame, outer: Rect, text: Rect, laid: Laid) {
    let Laid { chrome, rows, side } = laid;
    let rules: Vec<u16> = rows
        .iter()
        .enumerate()
        .filter(|(_, (_, line))| *line == preview::rule(text.width))
        .filter_map(|(k, _)| u16::try_from(k).ok())
        .collect();
    let lines: Vec<Line> = rows.into_iter().map(|(_, line)| line).collect();
    panel::draw(frame, outer, text, chrome, lines);
    if outer.height < 3 || outer.width < 2 {
        return;
    }
    let set = crate::sheet::read(|s| s.frame.boxes.lines());
    let across = format!(
        "{}{}{}",
        set.vertical_right,
        set.horizontal.repeat(usize::from(outer.width - 2)),
        set.vertical_left
    );
    let bar = side.map(|s| text.x + s.col).filter(|x| *x < outer.right());
    let buffer = frame.buffer_mut();
    for k in rules.into_iter().filter(|k| *k < text.height) {
        buffer.set_string(outer.x, text.y + k, &across, theme::dim());
        if let Some(x) = bar {
            buffer.set_string(x, text.y + k, set.horizontal_up, theme::dim());
        }
    }
    // 上边框那一格：标题写到这儿了的不盖（框线才接）。
    if let Some(x) = bar
        && buffer[(x, outer.y)].symbol() == set.horizontal
    {
        buffer.set_string(x, outer.y, set.horizontal_down, theme::dim());
    }
}

/// 屏幕上第 `x` 列、第 `y` 行是对得上的第几条；`text` 是框里放字的那一块，框的边、空着的地方是 `None`。预览在右边的，
/// 点右边那一半算选中的那一条。
pub fn index_at(text: Rect, laid: &Laid, x: u16, y: u16) -> Option<usize> {
    let k = usize::from(y.checked_sub(text.y)?);
    let row = laid.rows.get(k)?;
    match laid.side {
        Some(side) if k < side.rows && x >= text.x + side.col => side.picked,
        _ => row.0,
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_height;
