//! 「软件包」一半一半（发版前 check 13，2026-11-08 项目主人：「列表和选中信息全部是上对齐导致空间利用率存在问题。修改成一半
//! 一半，上半是列表，下半是详情」）。

use super::{page, screen};
use crate::config::Config;
use crate::settings::pages::Section;
use crate::settings::pages::store::{Listing, Remote, Tab};

#[test]
fn the_list_takes_the_top_half_and_the_preview_the_bottom_half() {
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    p.on_menu = false;
    p.more.section = Some(Section::Store);
    p.more.store.tab = Tab::Source;
    let remote = |id: &str, name: &str| Remote {
        id: id.into(),
        name: name.into(),
        version: "1.0.0".into(),
        ..Remote::default()
    };
    p.more.store.source = Listing::Got(vec![remote("weather", "天气"), remote("tarot", "塔罗")]);
    let height = 40;
    let (text, _) = screen(&mut p, &config, 80, height);
    let lines: Vec<&str> = text.lines().collect();
    let row = lines
        .iter()
        .position(|l| l.contains("天气") && !l.contains(':'));
    let preview = lines
        .iter()
        .position(|l| l.contains("天气") && l.contains(':'));
    assert!(row.is_some_and(|r| r < 10), "列表在上面：\n{text}");
    assert!(
        preview.is_some_and(|r| r >= usize::from(height) / 2),
        "预览块从下半开头：\n{text}"
    );
}

/// 发版前 check 57（2026-11-12 项目主人：「这个页面的列表，左侧缩进不合理」）：名字和别的页、预览块一样从第 3 格起，勾选记号写在
/// 选中条和名字之间那一格，不另占两格。
#[test]
fn names_start_where_the_preview_does_and_the_pick_mark_sits_in_the_gutter() {
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    p.on_menu = false;
    p.more.section = Some(Section::Store);
    p.more.store.tab = Tab::Source;
    let remote = |id: &str, name: &str| Remote {
        id: id.into(),
        name: name.into(),
        version: "1.0.0".into(),
        ..Remote::default()
    };
    p.more.store.source = Listing::Got(vec![remote("weather", "天气"), remote("tarot", "塔罗")]);
    p.more.store.picked.insert("tarot".into());
    let (text, _) = screen(&mut p, &config, 80, 40);
    let lines: Vec<&str> = text.lines().collect();
    let row = |name: &str| {
        lines
            .iter()
            .find(|l| l.contains(name) && !l.contains(':'))
            .unwrap_or_else(|| panic!("{name}：\n{text}"))
            .to_string()
    };
    let col = |line: &str, needle: &str| line[..line.find(needle).unwrap()].chars().count();
    let preview = lines.iter().find(|l| l.contains(':')).unwrap();
    let at = col(preview, preview.trim_start());
    assert_eq!(
        col(&row("天气"), "天气"),
        at,
        "名字和预览块的字一格起：\n{text}"
    );
    let tarot = row("塔罗");
    assert_eq!(col(&tarot, "塔罗"), at, "勾了的名字也不挪：\n{text}");
    assert_eq!(
        tarot.chars().nth(at - 1),
        Some('✓'),
        "记号在名字前一格：\n{text}"
    );
}

/// 发版前 check 57：「下载软件」多了 `r 刷新`，80 列里最后一条提示不被右边「第几个/一共几个」压成半截（几条之间的空收窄）。
#[test]
fn at_eighty_columns_every_hint_stays_whole_next_to_the_counter() {
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    p.on_menu = false;
    p.more.section = Some(Section::Store);
    p.more.store.tab = Tab::Source;
    let remote = |id: &str| Remote {
        id: id.into(),
        name: id.into(),
        version: "1.0.0".into(),
        ..Remote::default()
    };
    p.more.store.source = Listing::Got((0..43).map(|i| remote(&format!("p{i}"))).collect());
    let (text, _) = screen(&mut p, &config, 80, 40);
    let last = text.lines().rev().find(|l| !l.trim().is_empty()).unwrap();
    assert!(
        last.contains("r 刷新") && last.contains("Esc 返回"),
        "{last:?}"
    );
    assert!(last.trim_end().ends_with("1/43"), "{last:?}");
}
