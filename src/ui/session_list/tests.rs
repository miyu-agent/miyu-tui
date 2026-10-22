//! 会话列表的框画出来的样子：对齐、记号、总 token 一列、窄了标题先让。

use unicode_width::UnicodeWidthStr;

use super::{lines, short_path};
use crate::config::Config;
use crate::core::SessionInfo;
use crate::session_list::SessionList;

/// 只留会话那几行：段头（第 10 条）的另有测试。
fn sessions_only(
    rows: Vec<ratatui::text::Line<'static>>,
    map: &[Option<usize>],
) -> Vec<ratatui::text::Line<'static>> {
    rows.into_iter()
        .zip(map)
        .filter(|(_, item)| item.is_some())
        .map(|(row, _)| row)
        .collect()
}

fn info(session: &str, title: Option<&str>, cwd: &str) -> SessionInfo {
    SessionInfo {
        session: session.into(),
        title: title.map(str::to_string),
        named: false,
        pinned: false,
        cwd: Some(cwd.into()),
        busy: false,
        last_active: None,
        preview: None,
        persona: None,
        preset: None,
        tokens: 0,
        context: None,
        terminal: false,
    }
}

#[test]
fn rows_line_up_in_columns_and_only_a_different_directory_is_written() {
    // 2026-10-01 项目主人：原来编号、目录跟在标题后面各行位置不一样，看着乱；目录一样的不写；未命名的不用别的颜色。
    let config = Config::builtin().unwrap();
    let mut list = SessionList::default();
    list.replace(vec![
        info("0000-aaaaaaaa", Some("Miyu 各仓库代码量统计"), "~/src/miyu"),
        info("0000-bbbbbbbb", None, "~/src/miyu"),
        info("0000-cccccccc", Some("短"), "~/src/app"),
    ]);
    let (_, rows, map) = lines(
        &list,
        super::Marks {
            current: Some("0000-aaaaaaaa"),
            asking: &[],
        },
        "~/src/miyu",
        &config,
        90,
        20,
        0,
    );
    let rows = sessions_only(rows, &map);
    let text: Vec<String> = rows.iter().map(ToString::to_string).collect();
    let column = |l: &str| l.find(" #").map(|at| l[..at].width());
    let at: Vec<_> = text.iter().map(|l| column(l)).collect();
    assert!(
        at.iter().all(|c| c.is_some() && *c == at[0]),
        "编号一列对齐：{text:#?}"
    );
    assert!(text[2].contains("~/src/app"), "目录不一样的写：{text:#?}");
    assert!(
        !text[0].contains("~/src/miyu") && !text[1].contains("~/src/miyu"),
        "一样的不写"
    );
    let style_of = |row: usize, word: &str| {
        rows[row]
            .spans
            .iter()
            .find(|s| s.content.contains(word))
            .map(|s| s.style.fg)
    };
    assert_eq!(
        style_of(1, "未命名会话"),
        style_of(2, "短"),
        "未命名的和起了名的一个颜色"
    );
}

#[test]
fn the_current_one_gets_a_dot_and_a_busy_one_a_spinner_in_the_mark_column() {
    // 2026-10-02 项目主人：在跑的会话要看得出来，右边的「当前」不够显眼。标题前面加记号：正在用的 ●，在跑的转盲文。
    // 2026-10-16 项目主人：会话列表左边空出一大块，和模型、历史列表不一样：记号写在选中的 ❯ 那一列，不另占一列。
    let config = Config::builtin().unwrap();
    let mut list = SessionList::default();
    let mut busy = info("0000-bbbbbbbb", Some("在跑的"), "~/src/miyu");
    busy.busy = true;
    list.replace(vec![
        info("0000-aaaaaaaa", Some("正在用的"), "~/src/miyu"),
        busy,
        info("0000-cccccccc", Some("别的"), "~/src/miyu"),
    ]);
    let shown = |list: &SessionList| -> Vec<String> {
        let (_, rows, map) = lines(
            list,
            super::Marks {
                current: Some("0000-aaaaaaaa"),
                asking: &[],
            },
            "~/src/miyu",
            &config,
            90,
            20,
            0,
        );
        let rows = sessions_only(rows, &map);
        rows.iter().map(ToString::to_string).collect()
    };
    let spin = &config.timeline.spinner[0];
    list.selected = 2;
    let text = shown(&list);
    assert!(text[0].starts_with("● 正在用的"), "{text:#?}");
    assert!(text[1].starts_with(&format!("{spin} 在跑的")), "{text:#?}");
    assert!(text[2].starts_with("❯ 别的"), "{text:#?}");
    assert!(
        !text
            .iter()
            .any(|l| l.contains("当前") || l.contains("在忙")),
        "右边不再写：{text:#?}"
    );
    list.selected = 0;
    let text = shown(&list);
    // 2026-10-19 项目主人：正在用的选中了也照旧写 `●`、不让给 `❯`。
    assert!(
        text[0].starts_with("● 正在用的"),
        "正在用的选中了也写 ●：{text:#?}"
    );
    list.selected = 1;
    let text = shown(&list);
    assert!(
        text[1].starts_with(&format!("{spin} 在跑的")),
        "在跑的选中了也转：{text:#?}"
    );
}

#[test]
fn the_total_tokens_sit_between_the_id_and_the_right_column_right_aligned() {
    // 2026-10-19 项目主人：会话列表显示总 token，放在编号列和最右列中间。
    let config = Config::builtin().unwrap();
    let mut list = SessionList::default();
    let with = |id: &str, tokens: u64| SessionInfo {
        tokens,
        ..info(id, Some("标题"), "~/src/app")
    };
    list.replace(vec![
        with("0000-aaaaaaaa", 27_500_000),
        with("0000-bbbbbbbb", 1_234),
        with("0000-cccccccc", 0),
    ]);
    let marks = super::Marks {
        current: None,
        asking: &[],
    };
    let (_, rows, map) = lines(&list, marks, "~/src/miyu", &config, 90, 20, 0);
    let rows = sessions_only(rows, &map);
    let text: Vec<String> = rows.iter().map(ToString::to_string).collect();
    let after_id = |l: &str| {
        let at = l.find(" #").expect("有编号") + " #".len() + 8;
        l[at..].to_string()
    };
    let cells: Vec<String> = text.iter().map(|l| after_id(l)).collect();
    assert!(cells[0].starts_with("  Σ27.5M"), "{cells:#?}");
    assert!(cells[1].starts_with("   Σ1.2k"), "靠右对齐：{cells:#?}");
    assert!(cells[2].starts_with("      Σ0"), "没有的当 0：{cells:#?}");
    assert!(
        text.iter().all(|l| l.find("~/src/app") > l.find('Σ')),
        "在最右那一格的左边：{text:#?}"
    );
}

#[test]
fn a_narrow_box_shortens_the_title_before_the_tokens() {
    // 2026-10-19 真数据：窄的框里右边的目录把总 token 挤成「Σ4…」。标题先让，编号、总 token 照全写。
    let config = Config::builtin().unwrap();
    let mut list = SessionList::default();
    list.replace(vec![SessionInfo {
        tokens: 27_500_000,
        ..info(
            "0000-aaaaaaaa",
            Some("一个很长很长很长很长的会话标题"),
            "/somewhere/else/acceptance/tui-work",
        )
    }]);
    let marks = super::Marks {
        current: None,
        asking: &[],
    };
    let (_, rows, map) = lines(&list, marks, "~/src/miyu", &config, 64, 20, 0);
    let rows = sessions_only(rows, &map);
    let row = rows[0].to_string();
    assert!(row.contains("#aaaaaaaa  Σ27.5M"), "{row}");
    assert!(row.contains("…/acceptance/tui-work"), "{row}");
    assert!(
        row.contains('…') && !row.contains("会话标题"),
        "标题截短：{row}"
    );
}

#[test]
fn a_long_directory_keeps_its_last_two_levels() {
    assert_eq!(short_path("~/src/miyu", 24), "~/src/miyu");
    assert_eq!(
        short_path("/tmp/claude-1000/-home-shorin/scratchpad/work", 24),
        "…/scratchpad/work"
    );
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() {
        assert_eq!(short_path(&format!("{home}/src"), 24), "~/src");
        assert_eq!(
            short_path(&format!("{home}x/src"), 99),
            format!("{home}x/src"),
            "只认整层"
        );
    }
}

#[test]
fn sessions_are_grouped_by_age_under_plain_headings_unless_searching() {
    // 2026-10-19 项目主人：终端不做项目，只照时间分段，段头「算作标题式」：选不中、不收起，置顶、正在用的不带段头。
    let config = Config::builtin().unwrap();
    let ago = |days: i64| {
        let at = jiff::Timestamp::now() - jiff::SignedDuration::from_hours(days * 24);
        Some(at)
    };
    let with = |id: &str, title: &str, days: Option<i64>, pinned: bool| SessionInfo {
        pinned,
        last_active: days.and_then(ago),
        ..info(id, Some(title), "~/src/miyu")
    };
    let mut list = SessionList::default();
    list.replace(vec![
        with("0000-aaaaaaaa", "置顶的", Some(40), true),
        with("0000-bbbbbbbb", "今天", Some(0), false),
        with("0000-cccccccc", "三天前", Some(3), false),
        with("0000-dddddddd", "很久以前", Some(90), false),
        with("0000-eeeeeeee", "没动静", None, false),
    ]);
    let marks = super::Marks {
        current: None,
        asking: &[],
    };
    let (_, rows, map) = lines(&list, marks, "~/src/miyu", &config, 90, 20, 0);
    let text: Vec<String> = rows
        .iter()
        .map(|l| l.to_string().trim_end().to_string())
        .collect();
    let firsts: Vec<&str> = text
        .iter()
        .map(|l| l.split_whitespace().next().unwrap_or(""))
        .collect();
    assert_eq!(
        firsts,
        [
            "❯",
            "最近",
            "今天",
            "近",
            "三天前",
            "30",
            "很久以前",
            "没动静"
        ],
        "{text:#?}"
    );
    assert_eq!(text[3].trim(), "近 7 天", "空的「近 30 天」不画");
    assert_eq!(
        map,
        [
            Some(0),
            None,
            Some(1),
            None,
            Some(2),
            None,
            Some(3),
            Some(4)
        ],
        "段头选不中"
    );
    // 打字筛的时候不分段。
    list.query = "天".into();
    let (_, _, map) = lines(&list, marks, "~/src/miyu", &config, 90, 20, 0);
    assert!(map.iter().all(Option::is_some), "{map:?}");
}

#[test]
fn the_context_share_sits_left_of_the_total_as_a_percent() {
    // 2026-10-19 项目主人：会话的上下文用量只写百分比，放在总计左边；没窗口的写用量，没说过话的空着。
    let config = Config::builtin().unwrap();
    let mut list = SessionList::default();
    let with = |id: &str, context: Option<(u64, Option<u64>)>| SessionInfo {
        context,
        tokens: 4_600,
        ..info(id, Some("标题"), "~/src/miyu")
    };
    list.replace(vec![
        with("0000-aaaaaaaa", Some((4_400, Some(1_000_000)))),
        with("0000-bbbbbbbb", Some((318_000, Some(1_000_000)))),
        with("0000-cccccccc", Some((4_400, None))),
        with("0000-dddddddd", None),
    ]);
    let marks = super::Marks {
        current: None,
        asking: &[],
    };
    let (_, rows, map) = lines(&list, marks, "~/src/miyu", &config, 90, 20, 0);
    let rows: Vec<String> = sessions_only(rows, &map)
        .iter()
        .map(ToString::to_string)
        .collect();
    let after_id = |l: &str| {
        let at = l.find(" #").expect("有编号") + " #".len() + 8;
        l[at..].to_string()
    };
    let cells: Vec<String> = rows.iter().map(|l| after_id(l)).collect();
    assert!(cells[0].starts_with("   0.4%  Σ4.6k"), "{cells:#?}");
    assert!(cells[1].starts_with("  31.8%  Σ4.6k"), "{cells:#?}");
    assert!(
        cells[2].starts_with("   4.4k  Σ4.6k"),
        "没窗口的写用量：{cells:#?}"
    );
    assert!(
        cells[3].starts_with("         Σ4.6k"),
        "没说过话的空着：{cells:#?}"
    );
}

#[test]
fn terminal_sessions_say_so_when_there_is_room() {
    // 2026-10-21 项目主人：「终端集成会话在会话列表里可以有个区分例如加`终端集成`几个字。宽度不够的话就不显示了。」
    let config = Config::builtin().unwrap();
    let mut list = SessionList::default();
    let mut shell = info("0000-aaaaaaaa", Some("看日志"), "~/src/miyu");
    shell.terminal = true;
    list.replace(vec![
        shell,
        info("0000-bbbbbbbb", Some("写代码"), "~/src/miyu"),
    ]);
    let text = |width: u16| {
        let marks = super::Marks {
            current: None,
            asking: &[],
        };
        let (_, rows, map) = lines(&list, marks, "~/src/miyu", &config, width, 20, 0);
        let rows: Vec<String> = sessions_only(rows, &map)
            .iter()
            .map(ToString::to_string)
            .collect();
        rows
    };
    let wide = text(90);
    assert!(wide[0].contains("终端集成"), "{wide:#?}");
    assert!(!wide[1].contains("终端集成"), "{wide:#?}");
    let column = |l: &str| l.find(" #").map(|at| l[..at].width());
    assert_eq!(
        column(&wide[0]),
        column(&wide[1]),
        "编号一列照旧对齐：{wide:#?}"
    );
    // 编号、总计这些占 18 列，标题 6 列，「终端集成」那一格要 10 列：30 列放不下它。
    let narrow = text(30);
    assert!(
        narrow.iter().all(|l| !l.contains("终端集成")),
        "窄了不写：{narrow:#?}"
    );
    assert!(narrow[0].contains("看日志"), "标题不为它截短：{narrow:#?}");
}

#[test]
fn columns_stay_put_when_the_list_scrolls() {
    // 2026-10-22 项目主人：「列的位置不稳定，会移动」——终端集成会话、总 token 宽的那个滚出去以后，编号、总计一列跟着挪。
    let config = Config::builtin().unwrap();
    let mut list = SessionList::default();
    let mut all: Vec<SessionInfo> = (0..20)
        .map(|i| info(&format!("0000-{i:08}"), Some("写代码"), "~/src/miyu"))
        .collect();
    all[0].terminal = true;
    all[0].tokens = 692_600;
    all[0].context = Some((6_000, Some(100_000)));
    list.replace(all);
    let columns = |list: &SessionList| {
        let marks = super::Marks {
            current: None,
            asking: &[],
        };
        let (_, rows, map) = lines(list, marks, "~/src/miyu", &config, 120, 30, 0);
        let rows: Vec<String> = sessions_only(rows, &map)
            .iter()
            .map(ToString::to_string)
            .collect();
        let at = |l: &str, mark: &str| l.find(mark).map(|at| l[..at].width());
        (at(&rows[1], " #"), at(&rows[1], "Σ"), rows)
    };
    let (id_top, sum_top, top) = columns(&list);
    assert!(top[0].contains("终端集成"), "{top:#?}");
    list.selected = 19;
    let (id_end, sum_end, end) = columns(&list);
    assert!(
        end.iter().all(|l| !l.contains("终端集成")),
        "滚到底它不在露出来的里：{end:#?}"
    );
    assert_eq!(id_top, id_end, "编号一列不挪：{top:#?}\n{end:#?}");
    assert_eq!(sum_top, sum_end, "总计一列不挪：{top:#?}\n{end:#?}");
}
