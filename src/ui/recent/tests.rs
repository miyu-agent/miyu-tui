//! 侧边栏「最近会话」（「后台命令、子代理和侧边栏」第 7 条）：照列表的先后、当前这个在原位，个数、记号。

use super::recent_lines;
use crate::config::Config;
use crate::core::SessionInfo;

fn session(id: &str, title: Option<&str>) -> SessionInfo {
    SessionInfo {
        session: id.into(),
        title: title.map(str::to_string),
        ..SessionInfo::default()
    }
}

fn text(lines: &[ratatui::text::Line]) -> Vec<String> {
    lines.iter().map(|l| l.to_string()).collect()
}

#[test]
fn rows_follow_the_list_and_the_current_one_stays_in_place() {
    let config = Config::builtin().unwrap();
    let mut list = vec![
        session("a", Some("重构登录模块")),
        session("me", Some("列表里的旧名字")),
        session("b", None),
        session("c", Some("查 CI 为什么红")),
    ];
    list[0].busy = true;
    list[2].preview = Some("写周报".into());
    list[2].waiting = true;
    let current = Some(("me", Some("当前这个会话")));
    let (lines, ids) = recent_lines(&list, current, 3, (36, 0), &config);
    let spin = &config.timeline.spinner[0];
    assert_eq!(
        text(&lines),
        [
            "最近会话".to_string(),
            format!("{spin} 重构登录模块"),
            format!("{} 当前这个会话", config.layout.current_mark),
            "? 写周报".to_string(),
        ],
        "照列表的先后，当前这个在原位、照正文的标题（2026-10-27 项目主人：切过去就顶到最上面，难找）；在跑的转圈、等人的问号；\
         没标题的照预览；只列 3 个"
    );
    assert_eq!(
        ids,
        [None, Some("a".into()), Some("me".into()), Some("b".into())]
    );
    assert_eq!(
        lines[3].spans[0].style,
        crate::theme::warn(),
        "等人的问号是黄的"
    );
}

#[test]
fn the_current_one_past_the_count_or_not_yet_listed_still_shows() {
    let config = Config::builtin().unwrap();
    let list: Vec<SessionInfo> = ["a", "b", "c", "me"]
        .iter()
        .map(|id| session(id, Some(id)))
        .collect();
    let (_, ids) = recent_lines(&list, Some(("me", None)), 3, (36, 0), &config);
    assert_eq!(
        ids,
        [None, Some("a".into()), Some("b".into()), Some("me".into())],
        "排在名额以外：前面少列一个，它接在最后"
    );
    let (lines, ids) = recent_lines(&list[..2], Some(("new", None)), 3, (36, 0), &config);
    assert_eq!(
        ids,
        [None, Some("new".into()), Some("a".into()), Some("b".into())],
        "还没进列表的（刚开的）最新，排第一行"
    );
    assert!(lines[1].to_string().ends_with(&config.text.untitled));
}

#[test]
fn off_or_nothing_to_list_draws_nothing() {
    let config = Config::builtin().unwrap();
    let list = vec![session("a", Some("别的"))];
    let (lines, ids) = recent_lines(&list, Some(("me", None)), 0, (36, 0), &config);
    assert!(lines.is_empty() && ids.is_empty(), "选了不显示");
    let (lines, _) = recent_lines(&[], None, 5, (36, 0), &config);
    assert!(lines.is_empty(), "没开会话、列表也空");
    // 还没开会话：只列别的。
    let (lines, _) = recent_lines(&list, None, 5, (36, 0), &config);
    assert_eq!(text(&lines), ["最近会话", "  别的"]);
}

#[test]
fn a_long_name_is_cut_to_the_sidebar() {
    let config = Config::builtin().unwrap();
    let list = vec![session(
        "a",
        Some("一个非常非常非常非常非常非常长的会话标题会被截掉"),
    )];
    let (lines, _) = recent_lines(&list, None, 5, (20, 0), &config);
    let row = lines[1].to_string();
    assert!(row.ends_with('…'), "{row}");
    assert!(
        unicode_width::UnicodeWidthStr::width(row.as_str()) <= 20,
        "记号挂在缩进里，整行放得下：{row}"
    );
}
