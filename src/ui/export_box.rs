//! `/export` 的框（排期第 36 条，核心设计 58、会话导出调研第五节第 5 条；蓝图 `tui.md`「导出 `/export`」）：和帮助框、语言框
//! 一个位置、一个样子。第一步一行一种用途（完整存档、用于给别的 agent 继续、用于网页端聊天对话、仅导出对话为 Markdown 文件），右边暗色写大小和 token 估算；选了
//! Markdown 的第二步选复制还是存成文件，存档直接存。

use ratatui::text::{Line, Span};
use serde::Deserialize;

use super::panel::{self, Chrome, Row};
use crate::app::{ExportBox, Preview, Use};
use crate::config::Config;
use crate::theme;

/// 框里的字（`text/zh.json` 的 `export`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Texts {
    /// 上边框写的；第二步是 `{use}` 后面接的那一截。
    pub title: String,
    /// 第二步上边框：`{use}` 选的用途。
    pub step_title: String,
    /// 下边框的按键提示：第一步、第二步。
    pub hints: [String; 2],
    /// 四种用途，照 [`Use::ALL`] 的先后：完整存档、用于给别的 agent 继续、用于网页端聊天对话、仅导出对话为 Markdown 文件。
    pub uses: [String; 4],
    /// 存档那一行右边写的。
    pub bundle_note: String,
    /// 估好了：`{size}` 大小、`{tokens}` token。
    pub estimate: String,
    /// 还在估。
    pub estimating: String,
    /// 太大了放不进回应（超过 512 KiB）：只能存成文件。
    pub too_big: String,
    /// 第二步的两行：复制、存成文件。
    pub actions: [String; 2],
    /// 复制好了：`{tokens}`。
    pub copied: String,
    /// 存好了：`{path}`、`{size}`。
    pub saved: String,
    /// 还没开会话。
    pub no_session: String,
}

/// 框（标题、提示）、排好的行、每一行是第几项；选中第 `selected` 行，放不下 `max` 行时离它远的少露。
pub fn lines(
    config: &Config,
    open: Option<&ExportBox>,
    selected: usize,
    width: u16,
    max: usize,
) -> (Chrome, Vec<Line<'static>>, Vec<Option<usize>>) {
    let texts = &config.text.export;
    let Some(open) = open else {
        return Default::default();
    };
    let note = |text: String| Some(Span::styled(text, theme::dim()));
    let (chrome, rows): (Chrome, Vec<Row>) = match open.chosen {
        None => {
            let rows = Use::ALL
                .iter()
                .enumerate()
                .map(|(i, &used)| {
                    let right = match used {
                        Use::Bundle => note(texts.bundle_note.clone()),
                        _ => note(estimate(open.preview(used), texts)),
                    };
                    let content = vec![Span::raw(texts.uses[i].clone())];
                    (Some(i), panel::item(i == selected, content, right, width))
                })
                .collect();
            (
                Chrome::new(&texts.title, Vec::new()).hint(&texts.hints[0]),
                rows,
            )
        }
        Some(used) => {
            let title = texts.step_title.replace("{use}", &texts.uses[used.index()]);
            let can_copy = matches!(open.preview(used), Preview::Ready { text: Some(_), .. });
            let rows = texts
                .actions
                .iter()
                .enumerate()
                .map(|(i, action)| {
                    let style = if i == 0 && !can_copy {
                        theme::faint()
                    } else {
                        ratatui::style::Style::new()
                    };
                    let right = match i {
                        0 => note(estimate(open.preview(used), texts)),
                        _ => note(open.file_name(used)),
                    };
                    let content = vec![Span::styled(action.clone(), style)];
                    (Some(i), panel::item(i == selected, content, right, width))
                })
                .collect();
            (Chrome::new(&title, Vec::new()).hint(&texts.hints[1]), rows)
        }
    };
    let rows = panel::fit(rows, Some(selected), max);
    let (map, lines) = rows.into_iter().unzip();
    (chrome, lines, map)
}

/// 一种 Markdown 估出来的：大小、token；还在估的、太大的照那一句。
fn estimate(preview: &Preview, texts: &Texts) -> String {
    match preview {
        Preview::Waiting => texts.estimating.clone(),
        Preview::Ready {
            bytes,
            tokens,
            text: Some(_),
        } => texts
            .estimate
            .replace("{size}", &crate::drawer::size(*bytes))
            .replace("{tokens}", &crate::meter::short(*tokens)),
        Preview::Ready { text: None, .. } => texts.too_big.clone(),
        Preview::Failed(message) => message.clone(),
    }
}
