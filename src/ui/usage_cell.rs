//! 框下面那一行用量那一格（蓝图 `tui.md`「框下面那一行」用量那一格，反馈 1102 第 22 条，照网页 W-1053 的样板）：
//! 平常 `◔ 128.4k · ¥0.42`（环、累计、花费，都连子代理）；会话卡住的两种整格换成红字、后面跟一个能点的「压缩」；
//! 正在压缩的写转圈和「正在压缩上下文」。
//!
//! 只排成几截（[`Piece`]），挑哪几截放得下、画在哪由 `footer.rs` 办。

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use crate::config::UsageCellTexts;
use crate::core::Usage;
use crate::meter;
use crate::theme;
use crate::transcript::{Stuck, Transcript};

/// 这一截点了做什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    /// 开 `/context`。
    Usage,
    /// 同 `/compact`。
    Compact,
}

/// 一截：放不下时先丢 `keep` 小的；`tight` 的和前一截只隔一个空格（不隔 ` · `）；`hit` 是点了做什么。
#[derive(Debug, Clone)]
pub struct Piece {
    pub keep: u8,
    pub tight: bool,
    pub hit: Option<Hit>,
    pub spans: Vec<Span<'static>>,
}

impl Piece {
    fn new(keep: u8, tight: bool, hit: Option<Hit>, spans: Vec<Span<'static>>) -> Self {
        Self {
            keep,
            tight,
            hit,
            spans,
        }
    }
}

/// 排那一格要的几样（`footer.rs` 从 `Seen` 取好）。
pub struct Facts<'a> {
    /// 正在看的会话的正文：上下文、窗口、压缩线、卡没卡住、在不在压。
    pub t: &'a Transcript,
    /// 还没开会话时核心预览的上下文：用了多少、窗口（核心 CX-1 下）。
    pub preview: Option<(u64, Option<u64>)>,
    /// 累计，连子代理（`Talk::usage_total`）。
    pub total: Usage,
    /// 花了多少，连子代理，写成字（`Seen::spent`）；算不出的是 `None`。
    pub spent: Option<String>,
    pub texts: &'a UsageCellTexts,
    /// 转圈的几帧（同运行状态行）。
    pub spinner: &'a [String],
}

/// 用量那一格排成的几截；什么都还没有（环、累计都是 0）的是空的。`frame` 是转圈转到第几帧，`hovered` 是指针停在哪一截
/// 上（加粗，「压缩」另加下划线）。
pub fn pieces(facts: &Facts, frame: usize, hovered: Option<Hit>) -> Vec<Piece> {
    let (t, texts) = (facts.t, facts.texts);
    let bold = |hit: Hit, style: Style| {
        if hovered == Some(hit) {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        }
    };
    if t.compacting_shown() {
        let spinner = facts.spinner;
        let spin = spinner
            .get(frame % spinner.len().max(1))
            .cloned()
            .unwrap_or_default();
        let text = format!("{spin} {}", texts.compacting);
        return vec![Piece::new(
            5,
            false,
            None,
            vec![Span::styled(text, theme::dim())],
        )];
    }
    if let Some(stuck) = t.stuck() {
        let (words, used, limit) = match stuck {
            Stuck::Failed { used, line } => (&texts.failed, used, line),
            Stuck::Over { used, window } => (&texts.over, used, window),
        };
        let red = bold(Hit::Usage, theme::error());
        let head = format!("{} {words}", ring(used, t.limits.window));
        let ratio = format!("{}/{}", meter::short(used), meter::short(limit));
        let button = bold(Hit::Compact, theme::error().add_modifier(Modifier::BOLD));
        let button = if hovered == Some(Hit::Compact) {
            button.add_modifier(Modifier::UNDERLINED)
        } else {
            button
        };
        return vec![
            Piece::new(5, false, Some(Hit::Usage), vec![Span::styled(head, red)]),
            Piece::new(3, true, Some(Hit::Usage), vec![Span::styled(ratio, red)]),
            Piece::new(
                5,
                true,
                Some(Hit::Compact),
                vec![Span::styled(texts.compact.clone(), button)],
            ),
        ];
    }
    // 还没开会话：照选了的人格、预设预览的上下文（核心 CX-1 下），累计、花费还没有。
    let preview = facts.preview.filter(|_| t.session.is_none());
    let (used, window) = preview.unwrap_or((t.context, t.limits.window));
    let total = if preview.is_some() {
        Usage::default()
    } else {
        facts.total
    };
    let tokens = total.input() + total.output + total.aux;
    if used == 0 && tokens == 0 {
        return Vec::new();
    }
    let style = bold(Hit::Usage, theme::dim());
    let mut head = ring(used, window);
    if tokens > 0 {
        head = format!("{head} {}", meter::short(tokens));
    }
    let mut out = vec![Piece::new(
        4,
        false,
        Some(Hit::Usage),
        vec![Span::styled(head, style)],
    )];
    // 设了会话的上下文上限（核心 CTX-1，设计 68）：环照的窗口是被它压小的，写出来让人知道是哪一格在起作用；比花费后丢。
    if let Some(cap) = t.limits.context.filter(|_| preview.is_none()) {
        let cap = texts.cap.replace("{n}", &meter::short(cap));
        out.push(Piece::new(
            3,
            false,
            Some(Hit::Usage),
            vec![Span::styled(cap, style)],
        ));
    }
    // 花费：算不出钱的不写；放不下先收它（比权限级别的字先丢，`footer.rs` 的 `EARLY`）。
    if let Some(spent) = facts.spent.clone().filter(|_| preview.is_none()) {
        out.push(Piece::new(
            2,
            false,
            Some(Hit::Usage),
            vec![Span::styled(spent, style)],
        ));
    }
    out
}

/// 环：上下文占窗口不到 1/8、到 3/8、5/8、7/8、再往上（超出的画满）；没窗口的画空环（主题的 `marks.ring`）。
pub fn ring(used: u64, window: Option<u64>) -> String {
    let rings = crate::sheet::read(|s| s.marks.ring.clone());
    let level = match window.filter(|w| *w > 0) {
        None => 0,
        Some(window) => {
            let eighths = used.saturating_mul(8) / window;
            match eighths {
                0 => 0,
                1 | 2 => 1,
                3 | 4 => 2,
                5 | 6 => 3,
                _ => 4,
            }
        }
    };
    rings[level].clone()
}

#[cfg(test)]
mod tests;
