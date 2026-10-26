//! 加供应商先弹的目录窗（蓝图「配置页」第 11a 条）。

use super::{page, screen};
use crate::config::Config;
use crate::settings::pages::catalog::{CatalogPick, Entry};
use crate::settings::popup::Popup;

#[test]
fn the_catalog_window_lists_providers_then_more_and_custom_and_says_when_it_is_still_reading() {
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    let mut pick = CatalogPick {
        providers: None,
        all: false,
        search: String::new(),
        typing: false,
        sel: 0,
    };
    p.popup = Some(Popup::Catalog(pick.clone()));
    let (text, _) = screen(&mut p, &config, 100, 30);
    for want in ["添加供应商", "正在读取…", "更多供应商…", "自定义…"] {
        assert!(text.contains(want), "缺「{want}」：\n{text}");
    }
    let entry = |id: &str, name: &str| Entry {
        id: id.into(),
        name: name.into(),
        supported: true,
    };
    pick.providers = Some(vec![
        entry("deepseek", "DeepSeek"),
        entry("openai", "OpenAI"),
    ]);
    p.popup = Some(Popup::Catalog(pick.clone()));
    let (text, _) = screen(&mut p, &config, 100, 30);
    assert!(!text.contains("正在读取…"), "{text}");
    let at = |name: &str| {
        text.find(name)
            .unwrap_or_else(|| panic!("缺「{name}」：\n{text}"))
    };
    assert!(
        at("DeepSeek") < at("OpenAI")
            && at("OpenAI") < at("更多供应商…")
            && at("更多供应商…") < at("自定义…")
    );
    pick.all = true;
    pick.search = "zzz".into();
    p.popup = Some(Popup::Catalog(pick));
    let (text, _) = screen(&mut p, &config, 100, 30);
    assert!(
        text.contains("无匹配结果") && !text.contains("更多供应商…"),
        "{text}"
    );
}
