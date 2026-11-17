//! 会话列表照时间分段（蓝图 `tui.md`「会话列表」第 10 条，2026-10-19 项目主人定「算作标题式」）：正在用的、置顶的在最上面
//! 不带段头，结束了的人生最后照人格一段一段「已结束的人生 · 美羽」（「推演人生」第 8 条），别的照人最后一次发话（没人说过话的照开的时刻，和排的先后同一个时刻，`core/spoken.rs`）分「最近」「近 7 天」「近 30 天」「30 天前」，存档最后单独一段「存档」（排期第 38 条），每段前面
//! 一行段头，选不中、不能收起。

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

use crate::core::{SessionInfo, session_order_key};
use crate::theme;

/// 哪一段：照最近动静离今天几天。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Age {
    /// 今天、昨天。
    Recent,
    /// 近 7 天。
    Week,
    /// 近 30 天。
    Month,
    /// 30 天前，没有最近动静的也在这里。
    Older,
    /// 导进来的只读存档，不管多久以前（排期第 38 条）。
    Archived,
}

impl Age {
    /// 段头写第几个（`texts.groups` 的先后）。
    pub fn index(self) -> usize {
        self as usize
    }
}

/// 照本机的日子算：`today` 是今天，`tz` 是本机的时区。
pub fn age(at: Option<Timestamp>, today: Date, tz: &TimeZone) -> Age {
    let Some(at) = at else {
        return Age::Older;
    };
    let day = at.to_zoned(tz.clone()).date();
    let days = today.since(day).map_or(0, |span| span.get_days());
    match days {
        ..=1 => Age::Recent,
        2..=7 => Age::Week,
        8..=30 => Age::Month,
        _ => Age::Older,
    }
}

/// 分出来的一段：照时间的那几段，或者一个人格结束了的人生（「推演人生」第 8 条）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group<'a> {
    /// 照最近动静、存档。
    Age(Age),
    /// 这个人格结束了的人生；会话那时不带人格的是 `None`。
    Ended(Option<&'a str>),
}

/// 这一行属于哪一段：正在用的不分段（`None`）；结束了的人生照人格分段，存档都在「存档」那一段，置顶了的也是；别的置顶的不分段。
pub fn group<'a>(
    info: &'a SessionInfo,
    current: Option<&str>,
    today: Date,
    tz: &TimeZone,
) -> Option<Group<'a>> {
    if current == Some(info.session.as_str()) {
        return None;
    }
    if info.life_ended {
        return Some(Group::Ended(info.persona.as_deref()));
    }
    if info.archived {
        return Some(Group::Age(Age::Archived));
    }
    (!info.pinned).then(|| Group::Age(age(session_order_key(info), today, tz)))
}

/// 段头那一行：标题那一列起头，暗色加粗。
pub fn heading(text: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(text.to_string(), theme::dim().add_modifier(Modifier::BOLD)),
    ])
}

/// 去掉后面没有会话跟着的段头（放不下时离选中的远的会话先少露，段头要跟着走）。
pub fn drop_orphans(rows: Vec<super::super::panel::Row>) -> Vec<super::super::panel::Row> {
    let mut out: Vec<super::super::panel::Row> = Vec::with_capacity(rows.len());
    for row in rows.into_iter().rev() {
        let heading = row.0.is_none();
        let followed = out.last().is_some_and(|(item, _)| item.is_some());
        if !heading || followed {
            out.push(row);
        }
    }
    out.reverse();
    out
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use jiff::tz::TimeZone;

    use ratatui::text::Line;

    use super::{Age, age, drop_orphans};

    #[test]
    fn a_heading_left_without_sessions_after_it_is_dropped() {
        // 放不下时离选中的远的会话先少露（`panel::fit`），段头要跟着走。
        let row = |item: Option<usize>| (item, Line::raw(format!("{item:?}")));
        let rows = vec![
            row(None),
            row(Some(0)),
            row(None),
            row(None),
            row(Some(1)),
            row(None),
        ];
        let kept: Vec<_> = drop_orphans(rows)
            .into_iter()
            .map(|(item, _)| item)
            .collect();
        assert_eq!(kept, [None, Some(0), None, Some(1)]);
    }

    #[test]
    fn ended_lives_come_last_one_section_per_persona() {
        // 「人生-归档」（2026-11-17）：结束了的人生会话找不到。存档后面照人格一段一段，段头写人格的名字，点进去照常只读。
        use crate::config::Config;
        use crate::core::SessionInfo;
        use crate::session_list::SessionList;
        let one = |session: &str, persona: Option<&str>, ended: bool, archived: bool| SessionInfo {
            session: session.into(),
            title: Some(session.into()),
            persona: persona.map(str::to_string),
            last_active: Some("2026-11-17T08:00:00Z".parse().unwrap()),
            life: ended,
            life_ended: ended,
            archived,
            ..SessionInfo::default()
        };
        let mut list = SessionList {
            names: [("miyu".to_string(), "美羽".to_string())].into(),
            ..SessionList::default()
        };
        list.replace(vec![
            one("0000-life-a01", Some("miyu"), true, false),
            one("0000-life-b01", Some("luna"), true, false),
            one("0000-normal01", None, false, false),
            one("0000-life-a02", Some("miyu"), true, false),
            one("0000-archive1", None, false, true),
        ]);
        let config = Config::builtin().unwrap();
        let marks = super::super::Marks::default();
        let (_, rows, map) = super::super::lines(&list, marks, "~", &config, 90, 20, 0);
        let heads: Vec<String> = rows
            .iter()
            .zip(&map)
            .filter(|(_, item)| item.is_none())
            .map(|(row, _)| row.to_string().trim().to_string())
            .collect();
        assert_eq!(
            heads,
            [
                "30 天前",
                "存档",
                "已结束的人生 · luna",
                "已结束的人生 · 美羽"
            ],
            "没起名字的写编号"
        );
        let order: Vec<&str> = list.matches().iter().map(|s| s.session.as_str()).collect();
        assert_eq!(
            order,
            [
                "0000-normal01",
                "0000-archive1",
                "0000-life-b01",
                "0000-life-a01",
                "0000-life-a02"
            ],
            "同一个人格的挨着"
        );
    }

    #[test]
    fn days_since_the_last_activity_pick_the_group_by_local_date() {
        let tz = TimeZone::UTC;
        let today = date(2026, 10, 19);
        let at = |s: &str| Some(s.parse().unwrap());
        assert_eq!(age(at("2026-10-19T08:00:00Z"), today, &tz), Age::Recent);
        assert_eq!(
            age(at("2026-10-18T00:01:00Z"), today, &tz),
            Age::Recent,
            "昨天"
        );
        assert_eq!(age(at("2026-10-17T23:59:00Z"), today, &tz), Age::Week);
        assert_eq!(age(at("2026-10-12T12:00:00Z"), today, &tz), Age::Week);
        assert_eq!(age(at("2026-10-11T12:00:00Z"), today, &tz), Age::Month);
        assert_eq!(age(at("2026-09-19T12:00:00Z"), today, &tz), Age::Month);
        assert_eq!(age(at("2026-09-18T12:00:00Z"), today, &tz), Age::Older);
        assert_eq!(age(None, today, &tz), Age::Older, "没有最近动静的");
    }
}
