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
