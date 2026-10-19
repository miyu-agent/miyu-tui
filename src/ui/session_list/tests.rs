//! 会话列表的框画出来的样子：对齐、记号、总 token 一列、窄了标题先让。

use unicode_width::UnicodeWidthStr;

use super::{lines, short_path};
use crate::config::Config;
use crate::core::SessionInfo;
use crate::session_list::SessionList;

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
    let (_, rows, _) = lines(
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
        let (_, rows, _) = lines(
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
    assert!(text[0].starts_with("❯ 正在用的"), "选中的写 ❯：{text:#?}");
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
    let (_, rows, _) = lines(&list, marks, "~/src/miyu", &config, 90, 20, 0);
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
    let (_, rows, _) = lines(&list, marks, "~/src/miyu", &config, 64, 20, 0);
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
