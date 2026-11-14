//! 画「文件修改」那一页（蓝图 `tui.md`「文件修改」第 2 条，发版前 check 9，`app/files_page.rs`）：够宽时左边差异、右边一栏完整的
//! 文件列表（宽同侧边栏，中间一根暗色竖线），窄的列表在上面、最多露几行；最下面一行按键提示。差异照这个会话里改它的每一步
//! 照先后接起来，中间一道暗色细线（同网页），每一步先照参数比的、核心的完整差异回来了换上（带行号）。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{FilesPage, Seen};
use crate::changes::Changed;
use crate::theme;

/// 窄的时候列表最多露几行（连标题）。
const NARROW_LIST: u16 = 6;

/// 上一帧画成什么样：鼠标、键盘照它点、滚。
#[derive(Debug, Clone, Default)]
pub struct FilesFrame {
    /// 列表一个文件一行的那几行（不含标题）。
    pub list: Rect,
    /// 列表每一行是第几个文件。
    pub rows: Vec<usize>,
    /// 差异那一块。
    pub diff: Rect,
    /// 差异最多滚到第几行。
    pub most: usize,
}

/// 画一帧。
pub fn draw(frame: &mut Frame, page: &mut FilesPage, seen: &Seen, out: &mut FilesFrame) {
    let area = frame.area();
    let files = crate::changes::of(&seen.talk.transcript, &seen.config.timeline);
    *out = FilesFrame::default();
    if area.height < 3 || files.is_empty() {
        return;
    }
    let words = &seen.config.text.files;
    let hint = Rect::new(area.x, area.bottom() - 1, area.width, 1);
    frame.render_widget(
        Paragraph::new(super::floating::hint_line(&words.hints)),
        Rect::new(hint.x + 1, hint.y, hint.width.saturating_sub(2), 1),
    );
    let body = Rect::new(area.x, area.y, area.width, area.height - 1);
    let layout = &seen.config.layout;
    let (diff, list) = if body.width >= layout.sidebar_from {
        let side = layout.sidebar_width;
        let line: Vec<Line> = (0..body.height)
            .map(|_| Line::styled(crate::sheet::mark(|m| &m.side), theme::dim()))
            .collect();
        let x = body.right() - side - 1;
        frame.render_widget(Paragraph::new(line), Rect::new(x, body.y, 1, body.height));
        (
            Rect::new(
                body.x + 1,
                body.y,
                x.saturating_sub(body.x + 2),
                body.height,
            ),
            Rect::new(x + 2, body.y, side.saturating_sub(2), body.height),
        )
    } else {
        let h = NARROW_LIST.min(body.height / 2);
        (
            Rect::new(
                body.x + 1,
                body.y + h,
                body.width.saturating_sub(2),
                body.height - h,
            ),
            Rect::new(body.x + 1, body.y, body.width.saturating_sub(2), h),
        )
    };
    page.selected = page.selected.min(files.len() - 1);
    draw_list(frame, list, (&files, page.selected), seen, out);
    let lines = diff_lines(&files[page.selected], page, diff.width, seen);
    let height = usize::from(diff.height);
    out.most = lines.len().saturating_sub(height);
    page.top = page.top.min(out.most);
    let shown: Vec<Line> = lines.into_iter().skip(page.top).take(height).collect();
    frame.render_widget(Paragraph::new(shown), diff);
    out.diff = diff;
}

/// 右边（窄的时候上面）的列表：标题一行（「文件修改 · N 个文件」），下面一个文件一行，选中的那一行铺底、左边竖条；放不下的
/// 跟着选中的滚（同侧边栏别的列表）。
fn draw_list(
    frame: &mut Frame,
    area: Rect,
    (files, selected): (&[Changed], usize),
    seen: &Seen,
    out: &mut FilesFrame,
) {
    if area.height == 0 {
        return;
    }
    let words = &seen.config.text.files;
    let count = words.count.replace("{n}", &files.len().to_string());
    let title = Line::from(vec![
        Span::styled(words.title.clone(), theme::side_title()),
        Span::styled(format!("{}{count}", crate::sheet::sep()), theme::dim()),
    ]);
    frame.render_widget(
        Paragraph::new(title),
        Rect::new(area.x, area.y, area.width, 1),
    );
    let room = usize::from(area.height - 1);
    let top = crate::menu::edge(selected, None, 0, files.len(), room);
    let rows: Vec<usize> = (top..files.len()).take(room).collect();
    for (n, &i) in rows.iter().enumerate() {
        let y = area.y + 1 + u16::try_from(n).unwrap_or(0);
        let row = Rect::new(area.x, y, area.width, 1);
        if i == selected {
            frame.buffer_mut().set_style(row, theme::row_focus());
            super::floating::bar(frame, row);
        }
        frame.render_widget(
            Paragraph::new(super::sidebar::files::row(&files[i], area.width)),
            row,
        );
        // 行首两格是空白，箭头（没有颜色可靠的，check 13 ②）在字后面再写一遍
        if i == selected && crate::theme::arrows() {
            super::floating::bar(frame, row);
        }
    }
    out.list = Rect::new(
        area.x,
        area.y + 1,
        area.width,
        u16::try_from(rows.len()).unwrap_or(0),
    );
    out.rows = rows;
}

/// 左边的差异：头一行路径（加粗）和 `+a -r`，空一行，再是每一步的差异，两步之间一道暗色细线。
fn diff_lines(file: &Changed, page: &FilesPage, width: u16, seen: &Seen) -> Vec<Line<'static>> {
    let path = crate::local::home_short(&file.path);
    let mut out = vec![
        Line::from(vec![
            Span::styled(path, theme::accent().add_modifier(Modifier::BOLD)),
            Span::styled(format!("  +{} -{}", file.added, file.removed), theme::dim()),
        ]),
        Line::raw(""),
    ];
    let tab = seen.config.layout.output.tab;
    let rule = crate::sheet::mark(|m| &m.rule).repeat(usize::from(width));
    for (i, change) in file.changes.iter().enumerate() {
        if i > 0 {
            out.push(Line::styled(rule.clone(), theme::faint()));
        }
        let diff = change
            .call
            .as_ref()
            .and_then(|c| page.details.get(c))
            .unwrap_or(&change.diff);
        out.extend(
            super::diff_rows::lines(diff, width, tab)
                .into_iter()
                .map(|(gutter, text)| Line::from(vec![gutter, text])),
        );
    }
    out
}
