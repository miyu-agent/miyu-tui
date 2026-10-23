//! `/context` 上下文框（蓝图 `tui.md`「上下文 `/context`」，2026-10-22 项目主人定，核心 CX-1 的 `session.context` 和
//! `usage.query` 的 `use` 分组）：和 `/status` 一个位置、一个样子。上一段是组成（一根条、一块一行、空余），下一段是这个
//! 会话连子代理按用途的累计（`uses.rs`；还没开会话的预览没有这一段）。
//!
//! 先读成数（[`read_parts`]、[`read_uses`]），再排：排的那一半不碰界面，单元测试直接给数。

use std::collections::BTreeMap;

use ratatui::text::{Line, Span};
use serde::Deserialize;
use serde_json::Value;

use super::panel::{self, Chrome};
use crate::app::Seen;
use crate::config::Config;
use crate::core::UsageRow;
use crate::theme;

mod parts;
mod uses;

pub use parts::{Shape, read_parts};
pub use uses::read_uses;

/// 框里的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Texts {
    /// 框线上写的。
    pub title: String,
    /// 还没读回来。
    pub loading: String,
    /// 条后面的字：`{used}`、`{window}`；没有窗口的照 `used_only`。
    pub used: String,
    pub used_only: String,
    /// 压缩线，`{line}`。
    pub line: String,
    /// 全是本地估的。
    pub estimated: String,
    /// 累计那一段的标题。
    pub totals: String,
    /// 累计表的列名。
    pub columns: Columns,
    /// 累计表最后一行。
    pub sum: String,
    /// 用途的名字（核心的 `use`），认不得的写原样。
    pub uses: BTreeMap<String, String>,
    /// 下边框的按键提示。
    pub hint: String,
}

/// 累计表的列名。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Columns {
    pub requests: String,
    pub uncached: String,
    pub cache_read: String,
    pub cache_write: String,
    pub output: String,
    pub cost: String,
}

/// 读回来的一样：还没回来是 `None`，拒了的是核心的原话。
pub type Loaded<T> = Option<Result<T, String>>;

/// 框要画的：读回来的两样（还没开会话的只有预览，没有累计）。
#[derive(Debug, Default)]
pub struct ContextData {
    /// `session.context`。
    pub parts: Loaded<Shape>,
    /// `usage.query` 按 `use` 分的一行行。
    pub uses: Loaded<Vec<UsageRow>>,
}

/// 框线上的标题：只写「上下文明细」，用了多少在第一行写（2026-10-23 项目主人：标题、第一行、空余三处都写用量不合理）。
fn title(texts: &Texts, _data: Option<&ContextData>) -> String {
    texts.title.clone()
}

/// 框里的正文（滚动前的全部）。
pub fn rows(
    data: Option<&ContextData>,
    config: &Config,
    money: &dyn Fn(&crate::core::Bill) -> Option<String>,
    width: u16,
) -> Vec<Line<'static>> {
    let texts = &config.text.context_box;
    let note = |text: &str| vec![Line::styled(text.to_string(), theme::dim())];
    let Some(data) = data else {
        return note(&texts.loading);
    };
    let room = usize::from(width).saturating_sub(2).max(1);
    let mut out = match &data.parts {
        None => note(&texts.loading),
        Some(Err(said)) => vec![Line::styled(said.clone(), theme::error())],
        Some(Ok(shape)) => parts::rows(shape, texts, &config.layout.bar, room),
    };
    if let Some(Ok(list)) = &data.uses {
        let table = uses::rows(list, texts, money, room);
        if !table.is_empty() {
            out.push(Line::default());
            out.extend(table);
        }
    }
    out
}

/// 照界面排：金额照界面的币种和符号写。
fn seen_rows(seen: &Seen, width: u16) -> Vec<Line<'static>> {
    let money = |bill: &crate::core::Bill| {
        crate::money::amounts(bill, &seen.link.currency, &seen.config.layout.currencies)
    };
    rows(seen.boxes.context.as_ref(), seen.config, &money, width)
}

/// 一共几行（滚到哪为止）。
pub fn count(seen: &Seen, width: u16) -> usize {
    seen_rows(seen, width).len()
}

/// 框：标题、滚过 `scroll` 行以后的 `max` 行。
pub fn lines(
    seen: &Seen,
    scroll: usize,
    width: u16,
    max: usize,
) -> (Chrome, Vec<Line<'static>>, Vec<Option<usize>>) {
    let texts = &seen.config.text.context_box;
    let chrome = Chrome {
        title: vec![Span::raw(title(texts, seen.boxes.context.as_ref()))],
        hint: Some(texts.hint.clone()),
    };
    let lines: Vec<Line<'static>> = seen_rows(seen, width)
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
    let texts = &seen.config.text.context_box;
    let mut out = vec![title(texts, seen.boxes.context.as_ref())];
    out.extend(
        seen_rows(seen, width)
            .iter()
            .map(|l| l.to_string().trim_end().to_string()),
    );
    out.join("\n").trim_end().to_string()
}

/// 名字后面补空格到 `width` 列，数靠右对齐到 `width` 列：两段表都用。
fn pad_right(text: &str, width: usize) -> String {
    let have = unicode_width::UnicodeWidthStr::width(text);
    format!("{text}{}", " ".repeat(width.saturating_sub(have)))
}

fn pad_left(text: &str, width: usize) -> String {
    let have = unicode_width::UnicodeWidthStr::width(text);
    format!("{}{text}", " ".repeat(width.saturating_sub(have)))
}

/// 读不懂的数当没有。
fn number(v: &Value) -> Option<u64> {
    v.as_u64()
}

#[cfg(test)]
mod tests;
