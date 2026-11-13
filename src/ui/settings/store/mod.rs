//! 画「软件包」三页（蓝图「配置页」第 43c 条，照 shorin-pac 的版面）：面包屑、分页条（右边写排序）、筛的字、列表（名字、
//! 版本、暗色的接入方式标签、右边一格状态，不写说明）、预览块（浅一档的底），最下面照旧状态行、细线、按键提示。

mod asking;
mod caps;
mod popup;
mod preview;

use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::pages::{Painted, paint};
use super::{crumb, footer};
use crate::caret::Caret;
use crate::settings::pages::store::flow::Doing;
use crate::settings::pages::store::remove::{self, Button};
use crate::settings::pages::store::{AskView, Flow, Item, Listing, Store, StoreTexts, Tab};
use crate::settings::{Hit, Settings, Texts};
use crate::theme;

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
        // 画不出预览块：问的时候算没看完（发布前第五批审查 T5-1）。
        page.more.store.ask = AskView::default();
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
    if store.typing || !store.query.is_empty() {
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
    // 列表、空一行、预览块，下面留给状态行、细线、按键提示（`footer` 的四行）再空一行。装、卸、升级的那几步画在弹窗里
    // （`popup.rs`，排期第 42 条），列表照常。
    let room = stage.height.saturating_sub(4 + 5 + 1);
    let confirming = matches!(store.flow, Some(Flow::Confirm { .. }));
    // 一半一半：上半列表，下半预览块（发版前 check 13，2026-11-08 项目主人：「列表和选中信息全部是上对齐导致空间利用率
    // 存在问题」）；列表放不下的跟着光标滚。
    let list_h = (room / 2).max(1);
    let body = Rect::new(stage.x, stage.y + 4, stage.width, list_h);
    // 软件源给的名字、说明是外来的字：图格字、标签字符不进画面（发布前第五批审查 T5-3），列表、预览块都过一遍。
    let mut lines = rows(store, words, body.width);
    for (line, _) in &mut lines {
        crate::tabs::printable_lines(std::slice::from_mut(line));
    }
    let tab = store.tab.index();
    let mut top = page.more.store.top[tab];
    paint(frame, body, &lines, &mut top);
    page.more.store.top[tab] = top;
    if lines.len() == page.more.store.rows().len() {
        marks(frame, body, &page.more.store, top);
    }
    let store = &page.more.store;
    let pv_y = body.bottom() + 1;
    let room = stage.bottom().saturating_sub(5).saturating_sub(pv_y);
    let mut shown = preview::lines(store, words, &texts.more.package_from);
    crate::tabs::printable_lines(&mut shown);
    // 预览块铺满下半（上下各空一行）；放不下的截掉。
    let wrapped = wrap(&shown, stage.width.saturating_sub(4));
    let pv_h = room;
    let pv = Rect::new(stage.x, pv_y, stage.width, pv_h);
    if pv.height > 2 && !shown.is_empty() {
        frame.buffer_mut().set_style(pv, theme::panel());
        let inner = Rect::new(
            pv.x + 2,
            pv.y + 1,
            pv.width.saturating_sub(4),
            pv.height - 2,
        );
        for (n, line) in wrapped.iter().take(usize::from(inner.height)).enumerate() {
            let y = inner.y + u16::try_from(n).unwrap_or(0);
            frame.buffer_mut().set_line(inner.x, y, line, inner.width);
        }
    }
    let flow = page.more.store.flow.as_ref().map(|flow| {
        let mut lines = preview::flowing(&page.more.store, flow, words);
        crate::tabs::printable_lines(&mut lines);
        lines
    });
    let labels = remove_buttons(&page.more.store, words);
    let selected = (!labels.is_empty()).then_some(page.more.store.button);
    let drawn = flow.and_then(|lines| {
        popup::draw(
            frame,
            stage,
            lines,
            (confirming, &mut page.more.store.ask),
            (&labels, selected),
            words,
        )
    });
    let drawn = drawn.is_some_and(|rects| {
        let buttons = rects.into_iter().enumerate();
        page.hits
            .extend(buttons.map(|(i, rect)| (rect, Hit::Button(i))));
        true
    });
    let store = &mut page.more.store;
    if !confirming || !drawn {
        store.ask = AskView::default();
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

/// 列表一行：名字（照最长的对齐）、暗色版本、暗色的接入方式标签、右边一格状态。
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
    let at = store.cursor();
    let rows: Vec<(Vec<Span<'static>>, String, (String, Style))> = list
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
                        // 被它盖住的那份更新（核心 SD-1）：卸掉这份就换回去。
                        None if p.covers.as_ref().is_some_and(|c| c.newer) => {
                            (say("covered_newer"), theme::accent())
                        }
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
            let name_style = if off { theme::dim() } else { Style::new() };
            let pad = " ".repeat(name_w - item.name().width());
            spans.push(Span::styled(format!("{}{pad}", item.name()), name_style));
            spans.push(Span::styled(version, theme::dim()));
            (spans, tags(item, words), state)
        })
        .collect();
    let state_w = rows
        .iter()
        .map(|(_, _, (t, _))| t.width())
        .max()
        .unwrap_or(0);
    let room = usize::from(width).saturating_sub(2);
    rows.into_iter()
        .enumerate()
        .map(|(i, (mut spans, tags, (state, style)))| {
            // 接入方式的标签接在版本后面空两格；窄得和状态挤不下的先不画标签。
            let used: usize = spans.iter().map(Span::width).sum();
            if !tags.is_empty() && used + 2 + tags.width() + 2 + state_w <= room {
                spans.push(Span::styled(format!("  {tags}"), theme::dim()));
            }
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

/// 「下载软件」勾了的那几行：主题的 `ok` 记号写在选中条和名字之间那一格（发版前 check 57，2026-11-12 项目主人：原来另占两格，
/// 名字比别的页、预览块往右缩了两格）。两套主题的 `ok` 都是一格宽。
fn marks(frame: &mut Frame, body: Rect, store: &Store, top: usize) {
    if store.tab != Tab::Source || store.picked.is_empty() {
        return;
    }
    let ok = crate::sheet::read(|s| s.marks.ok.clone());
    let shown = store.rows().into_iter().enumerate().skip(top);
    for (n, item) in shown.take(usize::from(body.height)) {
        if store.picked.contains(item.id()) {
            let y = body.y + u16::try_from(n - top).unwrap_or(0);
            frame
                .buffer_mut()
                .set_string(body.x + 1, y, &ok, theme::accent());
        }
    }
}

/// 一行的接入方式标签（核心设计 48，2026-11-12 项目主人：编号不带接入方式，交给界面上的标签），中间隔主题的 `sep`；字表少写的那种不画。
fn tags(item: &Item, words: &StoreTexts) -> String {
    item.access()
        .iter()
        .map(|k| StoreTexts::say(&words.access, k))
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(&crate::sheet::sep())
}

/// 问卸载时弹窗最下面那排按钮（PK-5）；不在问卸载的没有。
fn remove_buttons(store: &Store, words: &StoreTexts) -> Vec<(String, bool)> {
    let Some(Flow::Confirm {
        doing: Doing::Remove(_),
        previews,
    }) = &store.flow
    else {
        return Vec::new();
    };
    let say = |key: &str| StoreTexts::say(&words.ask, key);
    remove::buttons(previews.first())
        .iter()
        .map(|b| match b {
            Button::Keep => (say("keep"), false),
            Button::Purge => (say("purge"), true),
            Button::Cancel => (say("cancel"), false),
        })
        .collect()
}

/// 这一页这时候的按键提示。
fn hints<'a>(store: &Store, words: &'a StoreTexts) -> &'a [[String; 2]] {
    let scroll = store.ask.rows > store.ask.room;
    match &store.flow {
        Some(Flow::Confirm {
            doing: Doing::Remove(_),
            ..
        }) => {
            return words.hint(if scroll {
                "confirm_remove_scroll"
            } else {
                "confirm_remove"
            });
        }
        Some(Flow::Confirm { .. }) if scroll => {
            return words.hint("confirm_scroll");
        }
        Some(Flow::Confirm { .. }) => return words.hint("confirm"),
        Some(_) => return words.hint("running"),
        None => {}
    }
    if store.typing {
        return words.hint("typing");
    }
    match store.tab {
        Tab::Source => words.hint("source"),
        Tab::Installed => words.hint("installed"),
        Tab::Backstage => words.hint("backstage"),
    }
}

/// 问的时候的预览块（发布前第五批审查 T5-1）：放得下的全写；放不下的照滚到的地方露一窗，最后一行暗色写下面（滚到底了写
/// 上面）还有几行。滚到哪、露几行记在 `view`，按键照它滚、没看完不让批。
fn asking_window(
    view: &mut AskView,
    lines: Vec<Line<'static>>,
    height: usize,
    words: &StoreTexts,
) -> Vec<Line<'static>> {
    if lines.len() <= height {
        *view = AskView {
            top: 0,
            rows: lines.len(),
            room: height,
        };
        return lines;
    }
    let room = height.saturating_sub(1).max(1);
    view.rows = lines.len();
    view.room = room;
    view.scroll(0);
    let ask = |key: &str, n: usize| StoreTexts::say(&words.ask, key).replace("{n}", &n.to_string());
    let note = match view.left() {
        0 => ask("above", view.top),
        below => ask("below", below),
    };
    let mut out: Vec<Line<'static>> = lines.into_iter().skip(view.top).take(room).collect();
    out.push(Line::styled(note, theme::faint()));
    out
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
