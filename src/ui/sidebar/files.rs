//! 侧边栏「文件修改」（蓝图 `tui.md`「文件修改」第 1 条，发版前 check 9，2026-11-13 主人 1113 初验）：这个会话改过的文件，一个
//! 一行（文件名、暗色的目录，后面暗色 `+3 -1`），先改的在前，最多列 `sidebar_files` 个，多的写「还有 N 个」；没改过文件的不出这
//! 一块。点一行开「文件修改」那一页、选上这个文件（`app/files_page.rs`）。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::super::layout::slot::{Laid, Part};
use super::parts::Side;
use crate::changes::Changed;
use crate::config::slots::SIDE_PARTS as NAMES;
use crate::theme;

/// `files`：改过的文件，一个一行；点一行开那一页。
#[derive(Default)]
pub struct Files {
    /// 从第 1 行（标题下面）起每一行是第几个文件。
    rows: Vec<usize>,
}

impl Part<Side<'_, '_>> for Files {
    fn name(&self) -> &'static str {
        NAMES[7]
    }

    fn lay(&mut self, side: &Side, width: u16) -> Laid {
        let seen = side.seen;
        let files = crate::changes::of(&seen.talk.transcript, &seen.config.timeline);
        let count = seen.config.layout.sidebar_files;
        self.rows.clear();
        if files.is_empty() || count == 0 {
            return Laid::Lines(Vec::new());
        }
        let words = &seen.config.text.files;
        let mut lines = vec![Line::styled(words.title.clone(), theme::side_title())];
        for (i, file) in files.iter().take(count).enumerate() {
            lines.push(row(file, width));
            self.rows.push(i);
        }
        if files.len() > count {
            let more = words
                .more
                .replace("{n}", &(files.len() - count).to_string());
            lines.push(Line::styled(format!("  {more}"), theme::dim()));
        }
        Laid::Lines(lines)
    }

    fn placed(
        &mut self,
        _: &mut Frame,
        area: Rect,
        (first, shown): (usize, usize),
        side: &mut Side,
    ) {
        // 标题那一行（第 0 行）不算，同跳转消息；「还有 N 个」那一行点了开那一页、选上第一个没露出来的。
        let start = first.max(1);
        let rows = (first + shown).saturating_sub(start);
        side.screen.areas.files = Rect {
            y: area.y + u16::try_from(start - first).unwrap_or(0),
            height: u16::try_from(rows).unwrap_or(0),
            ..area
        };
        side.screen.files = (start - 1..start - 1 + rows).collect();
    }
}

/// 一行：缩进两格、文件名（正文色），后面暗色的目录（家目录写 `~`，放不下先截目录），再空一格暗色 `+a -r`。
pub fn row(file: &Changed, width: u16) -> Line<'static> {
    let counts = format!("+{} -{}", file.added, file.removed);
    let room = usize::from(width).saturating_sub(2 + 1 + counts.width());
    let (dir, name) = split(&crate::local::home_short(&file.path));
    let name = super::super::rows::clip(&name, u16::try_from(room).unwrap_or(u16::MAX));
    let left = room.saturating_sub(name.width() + 1);
    let mut spans = vec![Span::raw("  "), Span::raw(name.clone())];
    if left > 1 && !dir.is_empty() {
        let dir = super::super::rows::clip(&dir, u16::try_from(left).unwrap_or(u16::MAX));
        spans.push(Span::styled(format!(" {dir}"), theme::faint()));
    }
    let used: usize = spans.iter().map(Span::width).sum();
    let pad = usize::from(width)
        .saturating_sub(used + counts.width())
        .max(1);
    spans.push(Span::raw(" ".repeat(pad)));
    spans.push(Span::styled(counts, theme::dim()));
    Line::from(spans)
}

/// 路径拆成目录（带结尾的 `/`，没有的是空的）和文件名。
fn split(path: &str) -> (String, String) {
    match path.rfind('/') {
        Some(at) if at + 1 < path.len() => (path[..=at].to_string(), path[at + 1..].to_string()),
        _ => (String::new(), path.to_string()),
    }
}
