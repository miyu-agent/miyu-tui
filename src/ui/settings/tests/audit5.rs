//! 发布前第五批审查（2026-11-02）：「软件包」三页里，问人批不批的那一块和软件源给的字。红测试，只断言缺陷，不改产品代码。

use super::{page, screen};
use crate::config::Config;
use crate::settings::pages::Section;
use crate::settings::pages::store::{Listing, Local, Remote};

/// 进了「软件包」的配置页：装着一个 `memes`。
fn store_page(config: &Config) -> crate::settings::Settings {
    let mut p = page(config);
    p.on_menu = false;
    p.more.section = Some(Section::Store);
    p.more.store.local = Some(vec![Local {
        id: "memes".into(),
        name: "表情包".into(),
        version: "0.1.0".into(),
        on: false,
        ..Local::default()
    }]);
    p
}

/// T5-3：软件源给的名字里的 kitty 图格字 U+10EEEE（T2-4 只在 `tabs::expand` 那一条入口去掉）原样画进格子：
/// 「软件包」列表、预览块不经那一条入口。
#[test]
fn a_kitty_placeholder_in_a_package_name_is_not_drawn() {
    let config = Config::builtin().unwrap();
    let mut p = store_page(&config);
    p.more.store.source = Listing::Got(vec![Remote {
        id: "x".into(),
        name: "天气\u{10EEEE}\u{0305}\u{0305}看不见".into(),
        summary: "说明\u{E0041}\u{E0042}".into(),
        version: "1.0.0".into(),
        ..Remote::default()
    }]);
    let (text, _) = screen(&mut p, &config, 80, 24);
    let bad: Vec<String> = text
        .chars()
        .filter(|c| *c == '\u{10EEEE}' || ('\u{E0020}'..='\u{E007F}').contains(c))
        .map(|c| format!("U+{:X}", u32::from(c)))
        .collect();
    assert!(
        bad.is_empty(),
        "软件源的字里的图格字、标签字符进了画面：{bad:?}"
    );
}
