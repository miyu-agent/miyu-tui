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
use crate::input::{Sent, pieces};
use crate::theme;
use unicode_width::UnicodeWidthStr;

/// 排好的一行：是对得上的第几条（「没有对得上的」、横线、搜索那一行是 `None`；预览那几行算选中的那一条），和画出来的样子。
pub type Row = panel::Row;

/// 列表至少留几条（地方不够时先缩预览，「输入历史列表」第 1 条）。
const KEEP: usize = 3;

/// 列表的框和框里从上往下的每一行。`matches` 是对得上的几条，最新的在前；`width` 是能写几列；`now` 算多久以前；
/// 框里最多 `max` 行（输入框上面剩下的，「窗口小的时候」第 1 条）。
pub fn lines(
    history: &History,
    matches: &[&Sent],
    width: u16,
    config: &Config,
    now: SystemTime,
    max: usize,
) -> (Chrome, Vec<Row>) {
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
    let rows = config.layout.history_rows.max(1);
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
    let caret = !vim || history.typing;
    let search = (None, search(&history.query, caret, words, width));
    if max < KEEP {
        return (chrome, panel::fit(list, Some(history.selected), max));
    }
    // 搜索那一行和它上面的横线一直在；剩下的先给列表留够，再给预览（连它上下两条圆角框的边）。
    let room = max - 2;
    let tall = preview_rows(matches, top..end, width, config);
    let wanted = if tall > 0 { tall + 2 } else { 0 };
    let keep = list.len().min(KEEP).max(room.saturating_sub(wanted));
    // 预览露不出一行字（连上下的边）就不画，地方还给列表。
    let shown = matches
        .get(history.selected)
        .filter(|_| room.saturating_sub(keep.min(list.len())) >= 3);
    let keep = if shown.is_some() { keep } else { room };
    let mut out = panel::fit(list, Some(history.selected), keep.min(room));
    let left = room - out.len();
    if let Some(sent) = shown {
        out.extend(
            preview(sent, width, tall.min(left - 2), words)
                .into_iter()
                .map(|line| (Some(history.selected), line)),
        );
    }
    out.push((None, rule(width)));
    out.push(search);
    (chrome, out)
}

/// 预览那一段要几行：露着的几条里最长的那条折出来几行，不超过 `history_preview_rows`（按上下选时框不跟着一伸一缩）。
fn preview_rows(
    matches: &[&Sent],
    shown: std::ops::Range<usize>,
    width: u16,
    config: &Config,
) -> usize {
    let cap = config.layout.history_preview_rows.max(1);
    shown
        .map(|i| wrapped(matches[i], inner(width)).len().min(cap))
        .max()
        .unwrap_or(0)
}

/// 预览框里能写几列：左右的边各占一格、再各空一格，字和列表的字对齐。
fn inner(width: u16) -> u16 {
    width.saturating_sub(4).max(1)
}

/// 一条的全文折成的几行：原来的换行照留，太长的折行。
fn wrapped(sent: &Sent, cols: u16) -> Vec<String> {
    pieces(sent.draft.text.trim_end(), cols)
        .into_iter()
        .map(|(piece, _)| piece)
        .collect()
}

/// 预览：选中那一条的全文，`height` 行字（短的补空行）装在一个圆角框里（框线照主题的 `frame.boxes`，同外面的框；发版前
/// check 第 31 条，2026-11-09 项目主人：原来上下两条横线隔开「有点丑」）；再长的最后一行写还有几行。
fn preview(sent: &Sent, width: u16, height: usize, words: &HistoryTexts) -> Vec<Line<'static>> {
    let all = wrapped(sent, inner(width));
    let shown = if all.len() > height {
        height.saturating_sub(1)
    } else {
        all.len()
    };
    let mut body: Vec<Vec<Span<'static>>> = all[..shown]
        .iter()
        .map(|piece| vec![Span::raw(piece.clone())])
        .collect();
    if all.len() > shown && height > 0 {
        let note = words
            .more
            .replace("{count}", &(all.len() - shown).to_string());
        body.push(vec![Span::styled(note, theme::faint())]);
    }
    body.resize(height, Vec::new());
    let set = crate::sheet::read(|s| s.frame.boxes.set());
    let across = set
        .horizontal_top
        .repeat(usize::from(width.saturating_sub(2)));
    let edge =
        |left: &str, right: &str| Line::styled(format!("{left}{across}{right}"), theme::dim());
    let cols = usize::from(inner(width));
    let mut out = vec![edge(set.top_left, set.top_right)];
    out.extend(body.into_iter().map(|spans| {
        let spans = panel::clip_spans(spans, cols);
        let used: usize = spans.iter().map(Span::width).sum();
        let mut line = vec![Span::styled(
            format!("{} ", set.vertical_left),
            theme::dim(),
        )];
        line.extend(spans);
        line.push(Span::raw(" ".repeat(cols.saturating_sub(used))));
        line.push(Span::styled(
            format!(" {}", set.vertical_right),
            theme::dim(),
        ));
        Line::from(line)
    }));
    out.push(edge(set.bottom_left, set.bottom_right));
    out
}

/// 隔开三段的暗色横线（照主题的框线）。
fn rule(width: u16) -> Line<'static> {
    let line = crate::sheet::read(|s| s.frame.boxes.set()).horizontal_top;
    Line::styled(line.repeat(usize::from(width)), theme::dim())
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

/// 画列表：`outer` 连框，`text` 是框里放字的那一块。隔开三段的横线接上左右的框线（`├─…─┤`）。
pub fn draw(frame: &mut Frame, outer: Rect, text: Rect, chrome: Chrome, rows: Vec<Row>) {
    let rules: Vec<u16> = rows
        .iter()
        .enumerate()
        .filter(|(_, (_, line))| *line == rule(text.width))
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
    for k in rules.into_iter().filter(|k| *k < text.height) {
        frame
            .buffer_mut()
            .set_string(outer.x, text.y + k, &across, theme::dim());
    }
}

/// 屏幕上第 `y` 行是对得上的第几条；`text` 是框里放字的那一块，框的边、空着的地方是 `None`。
pub fn index_at(text: Rect, rows: &[Row], y: u16) -> Option<usize> {
    rows.get(usize::from(y.checked_sub(text.y)?))?.0
}

#[cfg(test)]
mod tests;
