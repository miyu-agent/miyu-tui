//! `/status` 会话状态框（蓝图 `tui.md`「会话状态 `/status`」，2026-10-20 项目主人看了样板定）：左上角框线上写会话标题，一行
//! 一项，左边一列名字对齐、右边写值，放不下的折到下一行、照值那一列对齐。三段之间空一行，上下各空一行：会话 ID、人格、预设、
//! 权限模式、当前工作目录；模型、上下文、压缩、用量、缓存命中、花费；记忆。核心还没给的（工作区、上下文的组成、工具、MCP，
//! 核心排着 `session.status`）不写，不写假的数。还没开会话的标题写「新会话」，只写人格、预设、权限模式、当前工作目录、模型。
//!
//! 先从界面收齐要写的（[`facts`]），再排（[`items`]、[`rows`]）：排的那一半不碰界面，单元测试直接给数。

use ratatui::text::{Line, Span};
use serde::Deserialize;
use unicode_width::UnicodeWidthStr;

use super::panel::{self, Chrome};
use crate::app::Seen;
use crate::config::{Bar, Config};
use crate::core::{ToolStatus, Usage};
use crate::meter;
use crate::theme;
use crate::transcript::{StepKind, ToolState, Transcript};

/// 框里的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Texts {
    /// 还没开会话时框线上写的。
    pub new_title: String,
    /// 左边一列的名字。
    pub keys: Keys,
    /// 思考强度，`{effort}`。
    pub effort: String,
    /// 窗口，`{window}`。
    pub window: String,
    /// 平均出字速度，`{rate}`。
    pub average: String,
    /// 上下文用了多少，`{used}`、`{window}`、`{percent}`。
    pub context: String,
    /// 压缩过几次，`{n}`。
    pub compactions: String,
    /// 压缩线，`{line}`。
    pub compaction_line: String,
    /// 共多少，`{tokens}`。
    pub total: String,
    /// 输入、输出，`{input}`、`{output}`。
    pub split: String,
    /// 辅助请求，`{tokens}`。
    pub aux: String,
    /// 缓存断了几次，`{n}`。
    pub breaks: String,
    /// 回忆、记住，`{recalled}`、`{remembered}`。
    pub memory: String,
    /// 下边框的按键提示。
    pub hint: String,
}

/// 左边一列的名字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keys {
    pub id: String,
    pub persona: String,
    pub preset: String,
    pub level: String,
    pub cwd: String,
    pub model: String,
    pub context: String,
    pub compaction: String,
    pub usage: String,
    pub hit: String,
    pub cost: String,
    pub memory: String,
}

/// 框里要写的，从界面收齐的。
#[derive(Debug, Clone, Default)]
pub struct Facts {
    /// 框线上的标题。
    pub title: String,
    /// 完整的会话编号；还没开会话的是 `None`。
    pub session: Option<String>,
    /// 人格、预设的名字（无人格的写「无人格」）。
    pub persona: Option<String>,
    pub preset: Option<String>,
    /// 权限模式的名字。
    pub level: Option<String>,
    /// 当前工作目录，家目录写成 `~`。
    pub cwd: String,
    /// 模型、端点。
    pub model: Option<(String, String)>,
    pub effort: Option<String>,
    pub window: Option<u64>,
    /// 平均出字速度，每秒几个 token。
    pub speed: Option<f64>,
    /// 上下文用了多少；压缩线。
    pub context: u64,
    pub line: Option<u64>,
    /// 压缩过几次、缓存断了几次。
    pub compactions: u64,
    pub breaks: u64,
    /// 用量（连同子代理的）、花费。
    pub usage: Usage,
    pub spent: Option<String>,
    /// 回忆、记住各几次。
    pub memory: (usize, usize),
}

/// 一行：名字和值（值是几段字，亮的、暗的）；`None` 是空行。
pub type Item = Option<(String, Vec<Span<'static>>)>;

/// 从界面收齐要写的。
pub fn facts(seen: &Seen) -> Facts {
    let text = &seen.config.text;
    let t = &seen.talk.transcript;
    let opened = t.session.is_some();
    // 开着的会话照订阅回应（无人格的编号是空的）；以前的日志没有这一格的不写（同侧边栏）。
    let (persona, preset) = if opened {
        (t.persona.as_deref(), t.preset.as_deref())
    } else {
        (
            seen.talk.persona.chosen.as_deref(),
            seen.talk.preset.chosen.as_deref(),
        )
    };
    let title = match &t.session {
        None => text.status.new_title.clone(),
        Some(_) => t.title.clone().unwrap_or_else(|| text.untitled.clone()),
    };
    Facts {
        title,
        session: t.session.clone(),
        persona: persona.map(|id| match seen.talk.persona.name(id) {
            "" => text.persona_box.none.clone(),
            name => name.to_string(),
        }),
        preset: preset.map(|id| seen.talk.preset.name(id).to_string()),
        level: text.levels.get(&t.level).cloned(),
        cwd: crate::local::home_short(seen.current_workspace()),
        model: t.shown_model().map(|(m, e)| (m.to_string(), e.to_string())),
        effort: t.effort().map(str::to_string),
        window: t.limits.window,
        speed: t.speed.average(),
        context: t.context,
        line: t.limits.compaction_line,
        compactions: t.cache.compactions,
        breaks: t.cache.breaks,
        usage: seen.talk.usage_total(),
        spent: seen.spent(),
        memory: memory_calls(t),
    }
}

/// 一行一项，照样板的先后；没有的那一行不写，空了的那一段连它后面的空行一起不写。
pub fn items(f: &Facts, words: &Texts, bar: &Bar, room: usize) -> Vec<Item> {
    let keys = &words.keys;
    let opened = f.session.is_some();
    let dim = |s: String| Span::styled(s, theme::dim());
    let tail = |s: String| Span::styled(format!(" · {s}"), theme::dim());
    let plain = |s: String| vec![Span::raw(s)];
    let mut groups: [Vec<(String, Vec<Span<'static>>)>; 3] = Default::default();

    // 第一段：会话 ID、人格、预设、权限模式、当前工作目录。
    let first = &mut groups[0];
    if let Some(id) = &f.session {
        first.push((keys.id.clone(), plain(id.clone())));
    }
    if let Some(name) = &f.persona {
        first.push((keys.persona.clone(), plain(name.clone())));
    }
    if let Some(name) = &f.preset {
        first.push((keys.preset.clone(), plain(name.clone())));
    }
    if let Some(level) = &f.level {
        first.push((keys.level.clone(), plain(level.clone())));
    }
    first.push((keys.cwd.clone(), plain(f.cwd.clone())));

    // 第二段：模型、上下文、压缩、用量、缓存命中、花费。
    let second = &mut groups[1];
    if let Some((model, endpoint)) = &f.model {
        let mut value = vec![Span::raw(model.clone()), dim(format!(" · {endpoint}"))];
        if let Some(effort) = &f.effort {
            value.push(tail(words.effort.replace("{effort}", effort)));
        }
        if let Some(window) = f.window {
            value.push(tail(
                words.window.replace("{window}", &meter::short(window)),
            ));
        }
        if let Some(rate) = f.speed {
            value.push(tail(words.average.replace("{rate}", &format!("{rate:.0}"))));
        }
        second.push((keys.model.clone(), value));
    }
    if opened && f.context > 0 {
        second.push((keys.context.clone(), context(f, words, bar, room)));
    }
    if opened && (f.compactions > 0 || f.line.is_some()) {
        let n = f.compactions.to_string();
        let mut value = vec![Span::raw(words.compactions.replace("{n}", &n))];
        if let Some(line) = f.line {
            value.push(tail(
                words.compaction_line.replace("{line}", &meter::short(line)),
            ));
        }
        second.push((keys.compaction.clone(), value));
    }
    let u = &f.usage;
    let input = u.input();
    if opened && input + u.output + u.aux > 0 {
        let sum = meter::short(input + u.output + u.aux);
        let split = words
            .split
            .replace("{input}", &meter::short(input))
            .replace("{output}", &meter::short(u.output));
        let mut value = vec![
            Span::raw(words.total.replace("{tokens}", &sum)),
            tail(split),
        ];
        if u.aux > 0 {
            value.push(tail(words.aux.replace("{tokens}", &meter::short(u.aux))));
        }
        second.push((keys.usage.clone(), value));
        let rate = format!("{}%", meter::hit_rate(u.cache_read, input));
        let mut hit = vec![Span::styled(rate, theme::good())];
        if f.breaks > 0 {
            hit.push(tail(words.breaks.replace("{n}", &f.breaks.to_string())));
        }
        second.push((keys.hit.clone(), hit));
    }
    if let Some(spent) = f.spent.clone().filter(|_| opened) {
        second.push((keys.cost.clone(), plain(spent)));
    }

    // 第三段：记忆（这个会话里做成了的回忆、记住各几次）。
    if opened {
        let value = words
            .memory
            .replace("{recalled}", &f.memory.0.to_string())
            .replace("{remembered}", &f.memory.1.to_string());
        groups[2].push((keys.memory.clone(), plain(value)));
    }

    let mut out = vec![None];
    for group in groups.into_iter().filter(|g| !g.is_empty()) {
        out.extend(group.into_iter().map(Some));
        out.push(None);
    }
    out
}

/// 上下文那一格：一根条（压缩线落的那一格标黄，同侧边栏）接用了多少。
fn context(f: &Facts, words: &Texts, marks: &Bar, room: usize) -> Vec<Span<'static>> {
    let used = meter::short(f.context);
    let Some(window) = f.window else {
        return vec![Span::raw(used)];
    };
    let bar = super::sidebar::bar(f.context, window, f.line, marks.width.min(room / 3), marks);
    let mut spans: Vec<Span<'static>> = bar.spans.into_iter().skip(1).collect();
    let value = words
        .context
        .replace("{used}", &used)
        .replace("{window}", &meter::short(window))
        .replace("{percent}", &meter::percent_tenths(f.context, window));
    spans.push(Span::raw(format!("  {value}")));
    spans
}

/// 这个会话里做成了的回忆（`memory_search`）、记住（`remember`）各几次。
fn memory_calls(t: &Transcript) -> (usize, usize) {
    let mut counts = (0, 0);
    let steps = t
        .entries
        .iter()
        .filter_map(|e| e.segment.as_ref())
        .flat_map(|s| &s.steps);
    for step in steps {
        if let StepKind::Tool {
            name,
            state: ToolState::Done(ToolStatus::Ok),
            ..
        } = &step.kind
        {
            match name.as_str() {
                "memory_search" => counts.0 += 1,
                "remember" => counts.1 += 1,
                _ => {}
            }
        }
    }
    counts
}

/// 排好的一行行（滚动前的全部）：名字一列照最长的那个对齐，值放不下的折到下一行、照值那一列对齐。
pub fn rows(f: &Facts, config: &Config, width: u16) -> Vec<Line<'static>> {
    let room = usize::from(width).saturating_sub(2).max(1);
    let items = items(f, &config.text.status, &config.layout.bar, room);
    let key_width = items
        .iter()
        .flatten()
        .map(|(k, _)| k.width())
        .max()
        .unwrap_or(0)
        + 2;
    let value_room = room.saturating_sub(key_width).max(1);
    let mut out = Vec::new();
    for item in items {
        let Some((key, value)) = item else {
            out.push(Line::raw(""));
            continue;
        };
        for (n, piece) in wrap(value, value_room).into_iter().enumerate() {
            let head = if n == 0 { key.as_str() } else { "" };
            let pad = " ".repeat(key_width.saturating_sub(head.width()));
            let mut spans = vec![Span::styled(format!("{head}{pad}"), theme::dim())];
            spans.extend(piece);
            out.push(Line::from(spans));
        }
    }
    out
}

/// 框、这一屏的行、每一行是第几个（都选不中）。
pub fn lines(
    seen: &Seen,
    scroll: usize,
    width: u16,
    max: usize,
) -> (Chrome, Vec<Line<'static>>, Vec<Option<usize>>) {
    let f = facts(seen);
    let chrome = Chrome {
        title: vec![Span::raw(f.title.clone())],
        hint: Some(seen.config.text.status.hint.clone()),
    };
    let lines: Vec<Line<'static>> = rows(&f, seen.config, width)
        .into_iter()
        .skip(scroll)
        .take(max)
        .map(|line| panel::item(false, line.spans, None, width))
        .collect();
    let map = vec![None; lines.len()];
    (chrome, lines, map)
}

/// 照纯文字写整个框（`c` 复制）：第一行标题，下面照框里的样子。
pub fn plain(f: &Facts, config: &Config, width: u16) -> String {
    let mut out = vec![f.title.clone()];
    let body = rows(f, config, width);
    out.extend(body.iter().map(|l| l.to_string().trim_end().to_string()));
    out.join("\n").trim_end().to_string()
}

/// 几段字照显示宽度折成几行，每行不超过 `room` 列。
fn wrap(spans: Vec<Span<'static>>, room: usize) -> Vec<Vec<Span<'static>>> {
    use unicode_segmentation::UnicodeSegmentation;
    let mut lines: Vec<Vec<Span<'static>>> = vec![Vec::new()];
    let mut used = 0;
    for span in spans {
        let mut piece = String::new();
        for g in span.content.graphemes(true) {
            let w = g.width();
            if used + w > room && used > 0 {
                if !piece.is_empty() {
                    let done = Span::styled(std::mem::take(&mut piece), span.style);
                    lines
                        .last_mut()
                        .into_iter()
                        .for_each(|l| l.push(done.clone()));
                }
                lines.push(Vec::new());
                used = 0;
            }
            piece.push_str(g);
            used += w;
        }
        if !piece.is_empty() {
            let done = Span::styled(piece, span.style);
            lines
                .last_mut()
                .into_iter()
                .for_each(|l| l.push(done.clone()));
        }
    }
    lines
}

#[cfg(test)]
mod tests;
