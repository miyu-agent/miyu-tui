//! 会话列表照时间分段（蓝图 `tui.md`「会话列表」第 10 条，2026-10-19 项目主人定「算作标题式」）：正在用的、置顶的在最上面
//! 不带段头，别的照最近动静分「最近」「近 7 天」「近 30 天」「30 天前」，每段前面一行段头，选不中、不能收起。

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

use crate::core::SessionInfo;
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

/// 这一行属于哪一段：正在用的、置顶的不分段（`None`）。
pub fn group(info: &SessionInfo, current: Option<&str>, today: Date, tz: &TimeZone) -> Option<Age> {
    let on_top = info.pinned || current == Some(info.session.as_str());
    (!on_top).then(|| age(info.last_active, today, tz))
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
