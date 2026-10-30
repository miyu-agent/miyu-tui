//! 画「软件包」三页（蓝图「配置页」第 43c 条，照 shorin-pac 的版面）：面包屑、分页条（右边写排序）、筛的字、列表（名字、
//! 版本、右边一格状态，不写说明）、预览块（浅一档的底），最下面照旧状态行、细线、按键提示。

mod asking;
mod preview;

use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::pages::{Painted, paint};
use super::{crumb, footer};
use crate::caret::Caret;
use crate::settings::pages::store::{Flow, Item, Listing, Store, StoreTexts, Tab};
use crate::settings::{Hit, Settings, Texts};
use crate::theme;

/// 列表最多几行（2026-10-31 样板：放不下的跟着光标滚）。
const LIST_ROWS: u16 = 8;

/// 画一帧。
pub(super) fn draw(
    frame: &mut Frame,
    stage: Rect,
    page: &mut Settings,
    texts: &Texts,
    caret: &mut Caret,
) {
    let words = &texts.more.store;
    crumb(frame, stage, texts, Some(&words.entry[0]));
    if stage.height < 12 {
        return;
    }
    tabs(
        frame,
        Rect::new(stage.x, stage.y + 2, stage.width, 1),
        page,
        words,
    );
    let store = &page.more.store;
    let search_y = stage.y + 3;
    if store.tab == Tab::Source && (store.typing || !store.query.is_empty()) {
        let style = if store.typing {
            Style::new()
        } else {
            theme::dim()
        };
        let line = Line::from(vec![
            Span::styled(words.search.clone(), theme::dim()),
            Span::styled(store.query.clone(), style),
        ]);
        frame
            .buffer_mut()
            .set_line(stage.x + 2, search_y, &line, stage.width.saturating_sub(2));
        if store.typing {
            let x = stage.x
                + 2
                + u16::try_from((words.search.clone() + &store.query).width()).unwrap_or(0);
            caret.put(
                Position::new(x.min(stage.right().saturating_sub(1)), search_y),
                true,
            );
        }
    }
    // 列表、空一行、预览块，下面留给状态行、细线、按键提示（`footer` 的四行）再空一行。
    let room = stage.height.saturating_sub(4 + 5 + 1);
    let list_h = LIST_ROWS.min(room / 2).max(1);
    let body = Rect::new(stage.x, stage.y + 4, stage.width, list_h);
    let lines = rows(store, words, body.width);
    let tab = store.tab.index();
    let mut top = page.more.store.top[tab];
    paint(frame, body, &lines, &mut top);
    page.more.store.top[tab] = top;
    let store = &page.more.store;
    let pv_y = body.bottom() + 1;
    let pv_h = stage.bottom().saturating_sub(5).saturating_sub(pv_y);
    let pv = Rect::new(stage.x, pv_y, stage.width, pv_h);
    let shown = preview::lines(store, words, &texts.more.package_from);
    if pv.height > 0 && !shown.is_empty() {
        frame.buffer_mut().set_style(pv, theme::panel());
        let inner = Rect::new(
            pv.x + 2,
            pv.y + 1,
            pv.width.saturating_sub(4),
            pv.height.saturating_sub(2),
        );
        let wrapped = wrap(&shown, inner.width);
        for (n, line) in wrapped.iter().take(usize::from(inner.height)).enumerate() {
            let y = inner.y + u16::try_from(n).unwrap_or(0);
            frame.buffer_mut().set_line(inner.x, y, line, inner.width);
        }
    }
    let hints = hints(store, words);
    let count = store.rows().len();
    let counter = format!(
        "{}/{count}",
        if count == 0 { 0 } else { store.cursor() + 1 }
    );
    footer(frame, stage, page, texts, hints, None, Some(counter));
}

/// 分页条：当前页铺选中色的底、加粗，别的暗着，中间照记号表的间隔（同「供应商和模型」）；「下载软件」右边写排序。
fn tabs(frame: &mut Frame, row: Rect, page: &mut Settings, words: &StoreTexts) {
    let store = &page.more.store;
    let mut x = row.x;
    let sep = crate::sheet::sep();
    for (i, name) in words.tabs.iter().enumerate() {
        let on = store.tab.index() == i;
        let style = if on {
            theme::picked_bar().add_modifier(Modifier::BOLD)
        } else {
            theme::dim()
        };
        let label = format!(" {name} ");
        let width = u16::try_from(label.width()).unwrap_or(0);
        if x + width > row.right() {
            break;
        }
        frame.buffer_mut().set_string(x, row.y, &label, style);
        page.hits.push((Rect::new(x, row.y, width, 1), Hit::Tab(i)));
        x += width;
        let sep_w = u16::try_from(sep.width()).unwrap_or(3);
        if i + 1 < words.tabs.len() && x + sep_w <= row.right() {
            frame
                .buffer_mut()
                .set_string(x, row.y, &sep, theme::faint());
            x += sep_w;
        }
    }
    let store = &page.more.store;
    if let (Tab::Source, Some(sort)) = (store.tab, store.sort) {
        let right = Line::from(vec![
            Span::styled(format!("{} ", words.sort), theme::dim()),
            Span::raw(words.sorts[sort.index()].clone()),
        ]);
        let w = u16::try_from(right.width()).unwrap_or(0);
        if x + w + 2 <= row.right() {
            frame
                .buffer_mut()
                .set_line(row.right() - w, row.y, &right, w);
        }
    }
}

/// 列表一行：勾选记号（只有「下载软件」）、名字（照最长的对齐）、暗色版本、右边一格状态。
pub(super) fn rows(store: &Store, words: &StoreTexts, width: u16) -> Vec<Painted> {
    let say = |key: &str| StoreTexts::say(&words.states, key);
    let waiting = match (store.tab, &store.source) {
        (Tab::Source, Listing::Unasked | Listing::Loading) => Some(words.loading_source.clone()),
        (Tab::Source, Listing::Failed(why)) => Some(why.clone()),
        (_, _) if store.local.is_none() && store.tab != Tab::Source => Some(words.loading.clone()),
        _ => None,
    };
    if let Some(text) = waiting {
        return vec![(Line::styled(text, theme::dim()), false)];
    }
    let list = store.rows();
    if list.is_empty() {
        let empty = if store.tab == Tab::Backstage {
            &words.empty[1]
        } else {
            &words.empty[0]
        };
        return vec![(Line::styled(empty.clone(), theme::dim()), false)];
    }
    let name_w = list.iter().map(|r| r.name().width()).max().unwrap_or(0) + 2;
    let ok = crate::sheet::read(|s| s.marks.ok.clone());
    let at = store.cursor();
    let rows: Vec<(Vec<Span<'static>>, (String, Style))> = list
        .iter()
        .map(|item| {
            let (version, state, off) = match item {
                Item::Remote(p) => {
                    let state = match (&p.blocked, &p.installed) {
                        (Some(_), _) => (say("blocked"), theme::warn()),
                        (None, Some(had)) if *had == p.version => (say("installed"), theme::good()),
                        (None, Some(_)) => (say("upgradable"), theme::accent()),
                        (None, None) => (String::new(), Style::new()),
                    };
                    (p.version.clone(), state, false)
                }
                Item::Local(p) => {
                    let state = match store.upgrades.get(&p.id) {
                        Some(to) => (say("upgrade_to").replace("{to}", to), theme::accent()),
                        None if !p.on => (say("off"), theme::dim()),
                        None => (String::new(), Style::new()),
                    };
                    let version = if store.tab == Tab::Backstage {
                        String::new()
                    } else {
                        p.version.clone()
                    };
                    (version, state, !p.on)
                }
            };
            let mut spans = Vec::new();
            if store.tab == Tab::Source {
                let mark = if store.picked.contains(item.id()) {
                    ok.clone()
                } else {
                    " ".repeat(ok.width())
                };
                spans.push(Span::styled(format!("{mark} "), theme::accent()));
            }
            let name_style = if off { theme::dim() } else { Style::new() };
            let pad = " ".repeat(name_w - item.name().width());
            spans.push(Span::styled(format!("{}{pad}", item.name()), name_style));
            spans.push(Span::styled(version, theme::dim()));
            (spans, state)
        })
        .collect();
    let state_w = rows.iter().map(|(_, (t, _))| t.width()).max().unwrap_or(0);
    let room = usize::from(width).saturating_sub(2);
    rows.into_iter()
        .enumerate()
        .map(|(i, (mut spans, (state, style)))| {
            // 状态一格贴右边、照显示宽度右对齐；窄得放不下的接在版本后面空两格。
            let used: usize = spans.iter().map(Span::width).sum();
            let pad = if used + 2 + state_w <= room {
                room - used - state_w
            } else {
                2
            };
            spans.push(Span::raw(" ".repeat(pad + state_w - state.width())));
            spans.push(Span::styled(state, style));
            (Line::from(spans), i == at)
        })
        .collect()
}

/// 这一页这时候的按键提示。
fn hints<'a>(store: &Store, words: &'a StoreTexts) -> &'a [[String; 2]] {
    match &store.flow {
        Some(Flow::Confirm { .. }) => return words.hint("confirm"),
        Some(_) => return words.hint("running"),
        None => {}
    }
    match store.tab {
        Tab::Source if store.typing => words.hint("typing"),
        Tab::Source => words.hint("source"),
        Tab::Installed => words.hint("installed"),
        Tab::Backstage => words.hint("backstage"),
    }
}

/// 太长的行照宽度折开（照显示宽度，一个字一个字地切）。
fn wrap(lines: &[Line<'static>], width: u16) -> Vec<Line<'static>> {
    let width = usize::from(width.max(1));
    let mut out = Vec::new();
    for line in lines {
        let mut row: Vec<Span<'static>> = Vec::new();
        let mut used = 0;
        for span in &line.spans {
            let mut piece = String::new();
            for c in span.content.chars() {
                let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                if used + w > width {
                    row.push(Span::styled(std::mem::take(&mut piece), span.style));
                    out.push(Line::from(std::mem::take(&mut row)));
                    used = 0;
                }
                piece.push(c);
                used += w;
            }
            row.push(Span::styled(piece, span.style));
        }
        out.push(Line::from(row));
    }
    out
}

#[cfg(test)]
mod tests;
