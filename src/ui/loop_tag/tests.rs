//! 循环写在哪、怎么写（「循环」第 2、3 条）。

use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::{of, sidebar, tag};
use crate::config::Config;
use crate::core::{Loop, LoopEvery};

fn one(id: &str, session: &str, every: LoopEvery, next: Option<&str>) -> Loop {
    Loop {
        id: id.into(),
        session: session.into(),
        every,
        next: next.map(|t| t.parse().unwrap()),
        paused: false,
        prompt: "继续查询天气\n第二行不写".into(),
    }
}

fn text(lines: &[ratatui::text::Line]) -> Vec<String> {
    lines.iter().map(ToString::to_string).collect()
}

#[test]
fn the_tag_says_when_the_next_one_runs() {
    let config = Config::builtin().unwrap();
    let now: Timestamp = "2026-10-30T04:00:00Z".parse().unwrap();
    let at = (now, &TimeZone::UTC);
    let loops = vec![
        one(
            "t1",
            "s-1",
            LoopEvery::Seconds(600),
            Some("2026-10-31T04:07:00Z"),
        ),
        one(
            "t2",
            "s-1",
            LoopEvery::Dynamic,
            Some("2026-10-30T13:07:00Z"),
        ),
        one(
            "t3",
            "s-2",
            LoopEvery::Dynamic,
            Some("2026-10-30T05:00:00Z"),
        ),
    ];
    let mine = of(&loops, Some("s-1"));
    assert_eq!(mine.len(), 2, "只看看着的会话");
    assert_eq!(
        tag(&mine, &config, at).as_deref(),
        Some("↻ 13:07"),
        "最早那个，今天的只写时分"
    );
    let tomorrow = of(&loops[..1], Some("s-1"));
    assert_eq!(
        tag(&tomorrow, &config, at).as_deref(),
        Some("↻ 10-31 04:07")
    );
    let mut paused = loops[0].clone();
    paused.paused = true;
    assert_eq!(tag(&[&paused], &config, at).as_deref(), Some("↻ 已暂停"));
    let unset = one("t4", "s-1", LoopEvery::Dynamic, None);
    assert_eq!(
        tag(&[&unset], &config, at).as_deref(),
        Some("↻"),
        "还没定下一次"
    );
    assert_eq!(tag(&of(&loops, Some("s-9")), &config, at), None);
    assert!(of(&loops, None).is_empty(), "首页没有");
}

#[test]
fn the_sidebar_writes_how_often_when_and_what() {
    let config = Config::builtin().unwrap();
    let now: Timestamp = "2026-10-30T04:00:00Z".parse().unwrap();
    let at = (now, &TimeZone::UTC);
    let fixed = one(
        "t1",
        "s",
        LoopEvery::Seconds(600),
        Some("2026-10-30T13:07:00Z"),
    );
    let dynamic = one("t2", "s", LoopEvery::Dynamic, None);
    let cron = one("t3", "s", LoopEvery::Cron("0 8 * * *".into()), None);
    let mut hourly = one("t4", "s", LoopEvery::Seconds(7_200), None);
    hourly.paused = true;
    let lines = sidebar(&[&fixed, &dynamic, &cron, &hourly], &config, at, 40);
    assert_eq!(
        text(&lines),
        [
            "循环",
            "  每 10 分钟 · 下次 13:07",
            "  继续查询天气",
            "  动态间隔",
            "  继续查询天气",
            "  0 8 * * *",
            "  继续查询天气",
            "  每 2 小时 · 已暂停",
            "  继续查询天气",
        ]
    );
    assert_eq!(
        text(&sidebar(
            &[&one("t5", "s", LoopEvery::Seconds(90), None)],
            &config,
            at,
            40
        ))[1],
        "  每 90 秒"
    );
    assert!(sidebar(&[], &config, at, 40).is_empty(), "没有循环整段不画");
}
