//! 存档单独一段（排期第 38 条）。

use super::{Config, SessionInfo, SessionList, info, lines};

#[test]
fn archives_sit_last_under_their_own_heading() {
    // 排期第 38 条（核心设计 58 第三节第 4 条）：存档单独一段排在最后，今天动过的、置顶的存档也在这一段。
    let config = Config::builtin().unwrap();
    let today = Some(jiff::Timestamp::now());
    let with = |id: &str, title: &str, archived: bool, pinned: bool| SessionInfo {
        archived,
        pinned,
        last_active: today,
        ..info(id, Some(title), "~/src/miyu")
    };
    let mut list = SessionList::default();
    list.replace(vec![
        with("0000-aaaaaaaa", "旧机器上的", true, true),
        with("0000-bbbbbbbb", "今天", false, false),
        with("0000-cccccccc", "也是旧的", true, false),
    ]);
    let marks = super::super::Marks {
        current: None,
        asking: &[],
        unseen: None,
    };
    let (_, rows, _) = lines(&list, marks, "~/src/miyu", &config, 90, 20, 0);
    let firsts: Vec<String> = rows
        .iter()
        .map(|l| {
            let text = l.to_string();
            text.split_whitespace()
                .find(|w| w.chars().next().is_some_and(char::is_alphanumeric))
                .unwrap_or("")
                .to_string()
        })
        .collect();
    assert_eq!(
        firsts,
        ["最近", "今天", "存档", "旧机器上的", "也是旧的"],
        "{rows:#?}"
    );
}
