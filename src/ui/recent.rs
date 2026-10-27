//! 侧边栏「最近会话」（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 7 条，2026-10-26 项目主人）：照会话列表的先后列，当前这个
//! 在原位、行首实心圆点（2026-10-27 项目主人：切过去就顶到最上面，难找）；在跑的转圈，等人确认、等回答的写黄色 `?`。一共几个
//! 照 `tui.sidebar_sessions`。

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

/// 「最近会话」那一段：标题加一行行，一行是两格记号接名字（记号就挂在别的几段缩进的那两格里，名字和它们的内容对齐），
/// 一并交回每一行是哪个会话（标题那一行是 `None`）。`count` 是一共列几个，0 和没有可列的是空的。`current` 是正在看的会话和它的名字（没有标题的照列表里的写）。
pub fn recent_lines(
    list: &[SessionInfo],
    current: Option<(&str, Option<&str>)>,
    count: usize,
    (width, frame): (u16, usize),
    config: &Config,
) -> (Vec<Line<'static>>, Vec<Option<String>>) {
    let me = current.map(|c| c.0);
    let mut rows: Vec<Row> = list
        .iter()
        .map(|s| Row {
            id: &s.session,
            name: name(s).unwrap_or(&config.text.untitled),
            current: Some(s.session.as_str()) == me,
            busy: s.busy,
            waiting: s.waiting,
        })
        .collect();
    if let Some((id, title)) = current {
        match rows.iter().position(|r| r.current) {
            // 正文里有标题的照它（列表里的可能还是旧的）。
            Some(at) => rows[at].name = title.unwrap_or(rows[at].name),
            // 还没进列表（刚开的）：它最新，排第一行。
            None => rows.insert(
                0,
                Row {
                    id,
                    name: title.unwrap_or(&config.text.untitled),
                    current: true,
                    busy: false,
                    waiting: false,
                },
            ),
        }
    }
    // 当前这个排在名额以外的：前面的少列一个，它接在最后（先后不乱，自己也看得见）。
    if let Some(at) = rows.iter().position(|r| r.current)
        && count > 0
        && at >= count
    {
        let mine = rows.remove(at);
        rows.truncate(count - 1);
        rows.push(mine);
    }
    rows.truncate(count);
    if rows.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let mut lines = vec![Line::raw(config.text.side_recent.clone())];
    let mut ids = vec![None];
    // 记号两格（挂在缩进里）。
    let room = width.saturating_sub(2);
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
