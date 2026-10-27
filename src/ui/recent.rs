//! 侧边栏「最近会话」（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 7 条，2026-10-26 项目主人）：当前这个排第一行、行首实心
//! 圆点，下面照会话列表的先后列别的；在跑的转圈，等人确认、等回答的写黄色 `?`。一共几个照 `tui.sidebar_sessions`。

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::config::Config;
use crate::core::SessionInfo;
use crate::theme;

/// 一行：会话编号、名字、记号的样子。
struct Row<'a> {
    id: &'a str,
    name: &'a str,
    current: bool,
    busy: bool,
    waiting: bool,
}

/// 「最近会话」那一段：标题加一行行（还没起 [`super::sidebar::section`] 的缩进），一并交回每一行是哪个会话（标题那一行是
/// `None`）。`count` 是一共列几个，0 和没有可列的是空的。`current` 是正在看的会话和它的名字（没有标题的照列表里的写）。
pub fn recent_lines(
    list: &[SessionInfo],
    current: Option<(&str, Option<&str>)>,
    count: usize,
    (width, frame): (u16, usize),
    config: &Config,
) -> (Vec<Line<'static>>, Vec<Option<String>>) {
    let listed = |id: &str| list.iter().find(|s| s.session == id);
    let mut rows: Vec<Row> = Vec::new();
    if let Some((id, title)) = current {
        let info = listed(id);
        rows.push(Row {
            id,
            name: title
                .or(info.and_then(name))
                .unwrap_or(&config.text.untitled),
            current: true,
            busy: info.is_some_and(|s| s.busy),
            waiting: info.is_some_and(|s| s.waiting),
        });
    }
    let others = list
        .iter()
        .filter(|s| Some(s.session.as_str()) != current.map(|c| c.0));
    rows.extend(others.map(|s| Row {
        id: &s.session,
        name: name(s).unwrap_or(&config.text.untitled),
        current: false,
        busy: s.busy,
        waiting: s.waiting,
    }));
    rows.truncate(count);
    if rows.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let mut lines = vec![Line::raw(config.text.side_recent.clone())];
    let mut ids = vec![None];
    // 缩进两格、记号两格。
    let room = width.saturating_sub(4);
    for row in rows {
        let (mark, mark_style) = mark(&row, frame, config);
        let text = super::rows::clip(row.name, room);
        let style = if row.current {
            Style::new()
        } else {
            theme::dim()
        };
        let pad = 2usize.saturating_sub(mark.width());
        lines.push(Line::from(vec![
            Span::styled(format!("{mark}{}", " ".repeat(pad)), mark_style),
            Span::styled(text, style),
        ]));
        ids.push(Some(row.id.to_string()));
    }
    (lines, ids)
}

/// 列表里怎么叫它：标题，没有的照预览。
fn name(info: &SessionInfo) -> Option<&str> {
    info.title.as_deref().or(info.preview.as_deref())
}

/// 行首的记号：等人的黄色 `?`，在跑的转圈，当前这个强调色的实心圆点，别的空着。
fn mark(row: &Row, frame: usize, config: &Config) -> (String, Style) {
    let spinner = &config.timeline.spinner;
    let lead = if row.current {
        theme::accent()
    } else {
        theme::dim()
    };
    if row.waiting {
        ("?".to_string(), theme::warn())
    } else if row.busy {
        (spinner[frame % spinner.len().max(1)].clone(), lead)
    } else if row.current {
        (config.layout.current_mark.clone(), lead)
    } else {
        (String::new(), lead)
    }
}

#[cfg(test)]
mod tests;
