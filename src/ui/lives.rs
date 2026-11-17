//! 光打 `/life` 开的跳转列表（蓝图「推演人生」第 1 条，`app/lives.rs`）：和界面语言的框一个位置、一个样子。一条人生一行：
//! 人格的名字，后面暗色写在跑（「进行中 · 下次继续 14:05」）、暂停（「已暂停」）、正在跑一轮（「这一轮正在跑」）；正在看的那一条
//! 行首写当前的记号。

use jiff::Timestamp;
use jiff::tz::TimeZone;
use ratatui::text::{Line, Span};

use super::panel::{self, Chrome, Row};
use crate::app::Seen;
use crate::config::Config;
use crate::core::Life;
use crate::theme;

/// 框（标题、提示）、排好的行、每一行是第几条。
pub fn lines(
    seen: &Seen,
    selected: usize,
    width: u16,
    max: usize,
) -> (Chrome, Vec<Line<'static>>, Vec<Option<usize>>) {
    let texts = &seen.config.text.loops.life;
    let chrome = Chrome::new(&texts.jump[0], Vec::new()).hint(&texts.jump[1]);
    let here = seen.talk.transcript.session.as_deref();
    let now = (Timestamp::now(), &TimeZone::system());
    let rows: Vec<Row> = seen
        .boxes
        .lives
        .iter()
        .enumerate()
        .map(|(i, life)| {
            let on = here == Some(life.session.as_str());
            let lead = panel::current_lead(&seen.config.layout.current_mark, on);
            let content = vec![
                Span::raw(life.name.clone()),
                Span::styled(format!("  {}", state(life, seen.config, now)), theme::dim()),
            ];
            (
                Some(i),
                panel::item_with(i == selected, lead, content, None, width),
            )
        })
        .collect();
    let rows = panel::fit(rows, Some(selected), max);
    let (map, lines) = rows.into_iter().unzip();
    (chrome, lines, map)
}

/// 后面暗色写的那一截：正在跑一轮的、暂停着的、在跑的接下次几点。
pub(crate) fn state(life: &Life, config: &Config, (now, tz): (Timestamp, &TimeZone)) -> String {
    let t = &config.text.loops.life;
    if life.busy {
        return t.busy.clone();
    }
    if life.paused {
        return t.states[1].clone();
    }
    match life.next {
        Some(next) => format!(
            "{}{}{}",
            t.states[0],
            crate::sheet::sep(),
            t.next
                .replace("{time}", &super::loop_tag::when(next, now, tz))
        ),
        None => t.states[0].clone(),
    }
}

#[cfg(test)]
mod tests {
    use jiff::tz::TimeZone;

    use super::state;
    use crate::config::Config;
    use crate::core::Life;

    #[test]
    fn each_life_says_whether_it_runs_and_when_it_goes_on() {
        let config = Config::builtin().unwrap();
        let now = ("2026-11-17T08:00:00Z".parse().unwrap(), &TimeZone::UTC);
        let mut life = Life {
            next: Some("2026-11-17T14:05:00Z".parse().unwrap()),
            ..Life::default()
        };
        assert_eq!(state(&life, &config, now), "进行中 · 下次继续 14:05");
        life.busy = true;
        assert_eq!(state(&life, &config, now), "这一轮正在跑");
        life.busy = false;
        life.paused = true;
        assert_eq!(state(&life, &config, now), "已暂停");
    }
}
