//! `/tool-list` 工具框（蓝图 `tui.md`「工具 `/tool-list`」，2026-10-22 项目主人定，核心 S-T 的 `session.tools`）：和
//! `/status` 一个位置、一个样子。只写名字，按来源分组：组名一行（来源的名字，暗色跟几件），下面这一组的工具名空两格排开，
//! 放不下的折到下一行。组照内置、软件包、MCP、脚本排，组里照核心给的先后（名字的字节序）。
//!
//! 先读成 [`Tool`]（[`read`]），再排（[`rows`]）：排的那一半不碰界面，单元测试直接给数。

use ratatui::text::{Line, Span};
use serde::Deserialize;
use serde_json::Value;
use unicode_width::UnicodeWidthStr;

use super::panel::{self, Chrome};
use crate::app::Seen;
use crate::theme;

/// 框里的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Texts {
    /// 框线上写的，`{n}` 是一共几件。
    pub title: String,
    /// 还没读回来。
    pub loading: String,
    /// 一件都没有。
    pub empty: String,
    /// 组名后面暗色写的，`{n}`。
    pub count: String,
    /// 下边框的按键提示。
    pub hint: String,
}

/// 一件工具：名字和它从哪来。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// 名字（`shell`、`mcp__notes__search`）。
    pub name: String,
    /// 来源的种类：`builtin`、`package`、`mcp`、`script`。
    pub kind: String,
    /// 来源的编号：同种类里分组照它。
    pub source: String,
    /// 来源的名字（照连接的语言），组名写它；没有的写编号。
    pub source_name: String,
}

/// 读 `session.tools` 的回应；读不懂的那一件不要。
pub fn read(got: &Value) -> Vec<Tool> {
    let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
    got["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| {
            let name = text(&t["name"]);
            let source = &t["source"];
            let id = text(&source["id"]);
            (!name.is_empty()).then(|| Tool {
                name,
                kind: text(&source["kind"]),
                source_name: Some(text(&source["name"]))
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| id.clone()),
                source: id,
            })
        })
        .collect()
}

/// 种类的先后：内置、软件包、MCP、脚本，认不得的排最后。
fn kind_order(kind: &str) -> usize {
    ["builtin", "package", "mcp", "script"]
        .iter()
        .position(|k| *k == kind)
        .unwrap_or(4)
}

/// 分好的组：组名、这一组的工具名。
fn groups(tools: &[Tool]) -> Vec<(String, Vec<String>)> {
    let mut sorted: Vec<&Tool> = tools.iter().collect();
    // 稳定排序：同一组里照核心给的先后。
    sorted.sort_by_key(|t| (kind_order(&t.kind), t.kind.clone(), t.source.clone()));
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let mut last: Option<(&str, &str)> = None;
    for t in sorted {
        let key = (t.kind.as_str(), t.source.as_str());
        if last != Some(key) {
            out.push((t.source_name.clone(), Vec::new()));
            last = Some(key);
        }
        if let Some((_, names)) = out.last_mut() {
            names.push(t.name.clone());
        }
    }
    out
}

/// 排成的行（不带框）：组与组之间空一行；工具名缩进两格、空两格排开，一行放不下的折下去，一个名字比一行还长的独占一行。
pub fn rows(tools: &[Tool], texts: &Texts, width: u16) -> Vec<Line<'static>> {
    let room = usize::from(width).saturating_sub(4).max(1);
    let mut out = Vec::new();
    for (i, (name, names)) in groups(tools).into_iter().enumerate() {
        if i > 0 {
            out.push(Line::default());
        }
        let count = texts.count.replace("{n}", &names.len().to_string());
        out.push(Line::from(vec![
            Span::styled(name, theme::md_heading()),
            Span::styled(format!("  {count}"), theme::dim()),
        ]));
        let mut line = String::new();
        for tool in names {
            if !line.is_empty() && line.width() + 2 + tool.width() > room {
                out.push(Line::raw(format!("  {}", std::mem::take(&mut line))));
            }
            if !line.is_empty() {
                line.push_str("  ");
            }
            line.push_str(&tool);
        }
        if !line.is_empty() {
            out.push(Line::raw(format!("  {line}")));
        }
    }
    out
}

/// 框线上的标题：读回来了写一共几件，没回来、拒了的只写「工具」那一截（`{n}` 前面的字）。
fn title(texts: &Texts, loaded: Option<&Result<Vec<Tool>, String>>) -> String {
    match loaded {
        Some(Ok(tools)) => texts.title.replace("{n}", &tools.len().to_string()),
        _ => texts
            .title
            .split("{n}")
            .next()
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

/// 框里的正文：没回来写「正在读…」，拒了的写核心的原话，一件都没有的写「没有」。
fn body(
    texts: &Texts,
    loaded: Option<&Result<Vec<Tool>, String>>,
    width: u16,
) -> Vec<Line<'static>> {
    let note = |text: &str| vec![Line::styled(text.to_string(), theme::dim())];
    match loaded {
        None => note(&texts.loading),
        Some(Err(said)) => vec![Line::styled(said.clone(), theme::error())],
        Some(Ok(tools)) if tools.is_empty() => note(&texts.empty),
        Some(Ok(tools)) => rows(tools, texts, width),
    }
}

/// 一共几行（滚到哪为止）。
pub fn count(seen: &Seen, width: u16) -> usize {
    body(
        &seen.config.text.tool_list,
        seen.boxes.tools.as_ref(),
        width,
    )
    .len()
}

/// 框：标题、滚过 `scroll` 行以后的 `max` 行。
pub fn lines(
    seen: &Seen,
    scroll: usize,
    width: u16,
    max: usize,
) -> (Chrome, Vec<Line<'static>>, Vec<Option<usize>>) {
    let texts = &seen.config.text.tool_list;
    let loaded = seen.boxes.tools.as_ref();
    let chrome = Chrome {
        title: vec![Span::raw(title(texts, loaded))],
        hint: Some(texts.hint.clone()),
    };
    let lines: Vec<Line<'static>> = body(texts, loaded, width)
        .into_iter()
        .skip(scroll)
        .take(max)
        .map(|line| panel::item(false, line.spans, None, width))
        .collect();
    let map = vec![None; lines.len()];
    (chrome, lines, map)
}

/// 照纯文字写整个框（`c` 复制）：第一行标题，下面照框里的样子。
pub fn plain(seen: &Seen, width: u16) -> String {
    let texts = &seen.config.text.tool_list;
    let loaded = seen.boxes.tools.as_ref();
    let mut out = vec![title(texts, loaded)];
    out.extend(
        body(texts, loaded, width)
            .iter()
            .map(|l| l.to_string().trim_end().to_string()),
    );
    out.join("\n").trim_end().to_string()
}

#[cfg(test)]
mod tests;
