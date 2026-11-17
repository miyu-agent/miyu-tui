//! 循环（蓝图 `tui.md`「循环」第 2、3 条，2026-10-30 项目主人）：看着的会话挂着循环时，输入框上面那一行右对齐写下一次
//! 几点（长期目标在右、它在左），侧边栏多一段「循环」写多久一次、下一次、那句话。人生那一行（核心 LF-13，2026-11-17 项目主人
//! 本机验收第 23 条）同一个样子，记号用人生的，侧边栏写「推演人生」和人格的名字（那句话是推演方法，不写）。

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

/// 打头的记号：人生那一行照人生的（会话列表里人生会话标题前面那一个），别的照图标那一套的 `loop`。
pub(crate) fn mark(one: &Loop, config: &Config) -> String {
    if one.life.is_some() {
        config.icons.life_mark()
    } else {
        config.icons.loop_mark.clone()
    }
}

/// 输入框上面那一截（第 2 条）：「◷ 13:07」，停着的「◷ 已暂停」，还没定下一次的只写「◷」（记号照图标那一套的 `loop`，
/// 人生的照人生的）。没有循环的是 `None`。
pub(crate) fn tag(
    mine: &[&Loop],
    config: &Config,
    (now, tz): (Timestamp, &TimeZone),
) -> Option<String> {
    let first = mine.first()?;
    let mark = &mark(first, config);
    Some(match (first.paused, first.next) {
        (true, _) => format!("{mark}{}", config.text.loops.paused),
        (false, Some(next)) => format!("{mark}{}", when(next, now, tz)),
        (false, None) => mark.trim_end().to_string(),
    })
}

/// 侧边栏那一段（第 3 条）：标题，一个循环两行（缩两格，同别的几段）：「每 10 分钟 · 下次 13:07」，暗色的那句话截到一行。
/// 人生那一行第一行「推演人生 · 下次 13:07」，第二行暗色写人格的名字（`name` 照人格的编号给名字）。没有循环的是空的。
pub(crate) fn sidebar(
    mine: &[&Loop],
    (config, name): (&Config, &dyn Fn(&str) -> String),
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
        let said = match &one.life {
            Some(life) => life.persona.as_deref().map(name).unwrap_or_default(),
            None => one.prompt.lines().next().unwrap_or_default().to_string(),
        };
        lines.push(Line::styled(
            format!("  {}", crate::cells::clip(&said, room)),
            theme::dim(),
        ));
    }
    lines
}

/// 输入框上面那一截写什么、打头的记号：看着的会话的（看着子会话时不写，同长期目标）。
pub(super) fn shown(app: &crate::app::App) -> Option<(String, String)> {
    let session = app.talk.transcript.session.as_deref();
    let session = session.filter(|_| app.talk.viewing_title().is_none());
    let now = (Timestamp::now(), &TimeZone::system());
    let mine = of(&app.link.loops, session);
    let text = tag(&mine, &app.config, now)?;
    Some((text, mark(mine.first()?, &app.config)))
}

/// 画在框上面那一行，右对齐、暗色；`goal` 是长期目标那一截占的地方，有的话接在它左边、中间空两格。放不下整截的只写记号
/// （`mark` 是它打头的记号），记号也放不下的不写，不往框外伸（2026-11-02 项目主人：窄窗口里截掉了）。交回画在哪（没画的是空的）。
pub(super) fn draw(
    frame: &mut Frame,
    area: Rect,
    goal: Rect,
    (text, mark): &(String, String),
) -> Rect {
    let right = if goal.width > 0 {
        goal.x.saturating_sub(2)
    } else {
        area.right()
    };
    let room = usize::from(right.saturating_sub(area.x));
    let Some(text) = fit(text, mark, room) else {
        return Rect::default();
    };
    let width = u16::try_from(text.width()).unwrap_or(u16::MAX);
    let at = Rect::new(right - width, area.y, width, area.height.min(1));
    frame.render_widget(Paragraph::new(Span::styled(text, theme::dim())), at);
    at
}

/// 放得进 `room` 列的那一截：整截，不然只写记号，都放不下的是 `None`。
fn fit(text: &str, mark: &str, room: usize) -> Option<String> {
    let mark = mark.trim_end();
    [text, mark]
        .into_iter()
        .find(|t| !t.is_empty() && t.width() <= room)
        .map(str::to_string)
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
        LoopEvery::Life => texts.life.title.clone(),
    }
}

/// 时刻：今天的只写时分，不是今天的写「10-31 13:07」，照本机时区。
pub(crate) fn when(at: Timestamp, now: Timestamp, tz: &TimeZone) -> String {
    let at = at.to_zoned(tz.clone());
    if at.date() == now.to_zoned(tz.clone()).date() {
        at.strftime("%H:%M").to_string()
    } else {
        at.strftime("%m-%d %H:%M").to_string()
    }
}

#[cfg(test)]
mod tests;
