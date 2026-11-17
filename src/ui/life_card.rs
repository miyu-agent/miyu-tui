//! 人生卡（蓝图「推演人生」第 7 条，`app/life_card.rs`）：标题「推演人生」，一行一项、名字一列对齐：人格、开始、下次继续
//! （暂停着的不写）、上一轮（还没跑过的不写）、状态；空一行，一排两个按钮，停着的铺选中色（同「前缀要变」的框）。按过一下
//! 「结束人生」的，按钮上面一行用警告色写清后果，按钮换成「确认结束」。不写第几轮（核心的 `rounds` 头不显示）。

use jiff::Timestamp;
use jiff::tz::TimeZone;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::panel::{self, Chrome};
use crate::app::Seen;
use crate::app::life_card::{life_persona, life_row};
use crate::config::Config;
use crate::core::Loop;
use crate::theme;

/// 两个按钮上写的：暂停（暂停着的写继续）、结束人生（按过一下的写确认结束）。
pub(crate) fn labels(config: &Config, paused: bool, arming: bool) -> [String; 2] {
    let b = &config.text.loops.life.buttons;
    [
        b[usize::from(paused)].clone(),
        b[2 + usize::from(arming)].clone(),
    ]
}

/// 卡的标题和一行行（交给 `panels.rs`）；按钮那一行在映射里是 `Some(0)`，点击照横坐标认哪个（`app/life_card.rs`）。
pub fn lines(
    seen: &Seen,
    (button, arming): (usize, bool),
    width: u16,
    max: usize,
) -> (Chrome, Vec<Line<'static>>, Vec<Option<usize>>) {
    let texts = &seen.config.text.loops.life;
    let chrome = Chrome::new(&texts.title, Vec::new()).hint(&texts.card_hint);
    let Some(row) = life_row(seen.talk, seen.link) else {
        return (chrome, Vec::new(), Vec::new());
    };
    let name = life_persona(seen.talk, row).map(|p| seen.talk.persona.name(&p).to_string());
    let now = (Timestamp::now(), &TimeZone::system());
    let plain = |line: Line<'static>| (None, panel::item(false, line.spans, None, width));
    let mut rows: Vec<panel::Row> = fields(row, name.as_deref(), seen.config, now)
        .into_iter()
        .map(plain)
        .collect();
    rows.push(plain(Line::default()));
    if arming {
        rows.push(plain(Line::styled(texts.kill_hint.clone(), theme::warn())));
    }
    let mut spans = Vec::new();
    for (i, label) in labels(seen.config, row.paused, arming).iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let style = if i == button {
            theme::picked_bar()
        } else {
            Style::new()
        };
        spans.push(Span::styled(format!("[ {label} ]"), style));
    }
    rows.push((Some(0), panel::item(false, spans, None, width)));
    // 窗口矮得放不下的，留最下面的（按钮那一行）。
    let rows = rows.split_off(rows.len().saturating_sub(max));
    let (map, lines) = rows.into_iter().unzip();
    (chrome, lines, map)
}

/// 名字、值几行：名字暗色、一列对齐。没有的那一项不写。
pub(crate) fn fields(
    row: &Loop,
    name: Option<&str>,
    config: &Config,
    (now, tz): (Timestamp, &TimeZone),
) -> Vec<Line<'static>> {
    let t = &config.text.loops.life;
    let at = |time: Timestamp| super::loop_tag::when(time, now, tz);
    let life = row.life.as_ref();
    let state = &t.states[usize::from(row.paused)];
    let values = [
        name.map(str::to_string),
        life.and_then(|l| l.created).map(at),
        row.next.filter(|_| !row.paused).map(at),
        life.and_then(|l| l.last).map(at),
        Some(state.clone()),
    ];
    let width = t.keys.iter().map(|k| k.width()).max().unwrap_or(0);
    t.keys
        .iter()
        .zip(values)
        .filter_map(|(key, value)| {
            let pad = " ".repeat(width.saturating_sub(key.width()));
            Some(Line::from(vec![
                Span::styled(format!("{key}{pad}  "), theme::dim()),
                Span::raw(value?),
            ]))
        })
        .collect()
}

#[cfg(test)]
mod tests;
