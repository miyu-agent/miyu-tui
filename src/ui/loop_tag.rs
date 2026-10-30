//! 循环（蓝图 `tui.md`「循环」第 2、3 条，2026-10-30 项目主人）：看着的会话挂着循环时，输入框上面那一行右对齐写下一次
//! 几点（长期目标在右、它在左），侧边栏多一段「循环」写多久一次、下一次、那句话。

use jiff::Timestamp;
use jiff::tz::TimeZone;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::config::Config;
use crate::core::{Loop, LoopEvery};
use crate::theme;

/// 看着的会话挂着的循环：在跑的、下一次早的在前，没定下一次的、停着的排后面。
pub(crate) fn of<'a>(loops: &'a [Loop], session: Option<&str>) -> Vec<&'a Loop> {
    let Some(session) = session else {
        return Vec::new();
    };
    let mut mine: Vec<&Loop> = loops.iter().filter(|l| l.session == session).collect();
    mine.sort_by_key(|l| (l.paused, l.next.is_none(), l.next));
    mine
}

/// 输入框上面那一截（第 2 条）：「↻ 13:07」，停着的「↻ 已暂停」，还没定下一次的只写「↻」。没有循环的是 `None`。
pub(crate) fn tag(
    mine: &[&Loop],
    config: &Config,
    (now, tz): (Timestamp, &TimeZone),
) -> Option<String> {
    let first = mine.first()?;
    let mark = &config.layout.loop_mark;
    Some(match (first.paused, first.next) {
        (true, _) => format!("{mark}{}", config.text.loops.paused),
        (false, Some(next)) => format!("{mark}{}", when(next, now, tz)),
        (false, None) => mark.trim_end().to_string(),
    })
}

/// 侧边栏那一段（第 3 条）：标题，一个循环两行（缩两格，同别的几段）：「每 10 分钟 · 下次 13:07」，暗色的那句话截到一行。没有循环的是空的。
pub(crate) fn sidebar(
    mine: &[&Loop],
    config: &Config,
    (now, tz): (Timestamp, &TimeZone),
    width: u16,
) -> Vec<Line<'static>> {
    if mine.is_empty() {
        return Vec::new();
    }
    let texts = &config.text.loops;
    let room = usize::from(width).saturating_sub(2);
    let mut lines = vec![Line::styled(texts.title.clone(), theme::side_title())];
    for one in mine {
        let mut head = vec![every(&one.every, config)];
        match (one.paused, one.next) {
            (true, _) => head.push(texts.paused.clone()),
            (false, Some(next)) => head.push(texts.next.replace("{time}", &when(next, now, tz))),
            (false, None) => {}
        }
        let head = head.join(&crate::sheet::sep());
        lines.push(Line::raw(format!("  {}", crate::cells::clip(&head, room))));
        let said = one.prompt.lines().next().unwrap_or_default();
        lines.push(Line::styled(
            format!("  {}", crate::cells::clip(said, room)),
            theme::dim(),
        ));
    }
    lines
}

/// 输入框上面那一截写什么：看着的会话的（看着子会话时不写，同长期目标）。
pub(super) fn shown(app: &crate::app::App) -> Option<String> {
    let session = app.talk.transcript.session.as_deref();
    let session = session.filter(|_| app.talk.viewing_title().is_none());
    let now = (Timestamp::now(), &TimeZone::system());
    tag(&of(&app.link.loops, session), &app.config, now)
}

/// 画在框上面那一行，右对齐、暗色；`goal` 是长期目标那一截占的地方，有的话接在它左边、中间空两格。
pub(super) fn draw(frame: &mut Frame, area: Rect, goal: Rect, text: &str) {
    let width = u16::try_from(text.width()).unwrap_or(u16::MAX);
    let right = if goal.width > 0 {
        goal.x.saturating_sub(2)
    } else {
        area.right()
    };
    let x = right.saturating_sub(width).max(area.x);
    let at = Rect::new(x, area.y, right.saturating_sub(x), area.height.min(1));
    frame.render_widget(
        Paragraph::new(Span::styled(text.to_string(), theme::dim())),
        at,
    );
}

/// 多久一次：固定间隔照秒数挑最大的整单位写（600 秒写「每 10 分钟」），cron 原样，动态的写「动态间隔」。
fn every(every: &LoopEvery, config: &Config) -> String {
    let texts = &config.text.loops;
    match every {
        LoopEvery::Seconds(seconds) => {
            let units = [86_400, 3_600, 60, 1];
            let at = units
                .iter()
                .position(|unit| seconds % unit == 0 && seconds >= unit)
                .unwrap_or(3);
            texts.every[3 - at].replace("{n}", &(seconds / units[at]).to_string())
        }
        LoopEvery::Cron(cron) => cron.clone(),
        LoopEvery::Dynamic => texts.dynamic.clone(),
    }
}

/// 时刻：今天的只写时分，不是今天的写「10-31 13:07」，照本机时区。
fn when(at: Timestamp, now: Timestamp, tz: &TimeZone) -> String {
    let at = at.to_zoned(tz.clone());
    if at.date() == now.to_zoned(tz.clone()).date() {
        at.strftime("%H:%M").to_string()
    } else {
        at.strftime("%m-%d %H:%M").to_string()
    }
}

#[cfg(test)]
mod tests;
