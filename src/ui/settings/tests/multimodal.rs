//! 默认模型里替对话模型看图、听音频的那一项叫「多模态」（2026-11-20 主人本机验收第 6 条，同网页、核心的字表）。

use super::{page, screen};
use crate::config::Config;

#[test]
fn the_second_default_model_is_called_multimodal() {
    let config = Config::builtin().unwrap();
    let mut page = page(&config);
    page.on_menu = false;
    page.nav.page_step(true);
    page.nav.vertical(1, &page.view);
    let (text, _) = screen(&mut page, &config, 120, 30);
    assert!(text.contains("默认多模态模型"), "{text}");
    assert!(
        text.contains("在对话模型不支持多模态时提供多模态能力"),
        "{text}"
    );
    assert!(!text.contains("视觉"), "{text}");
}

#[test]
fn usage_names_the_multimodal_purpose_the_same_way() {
    let table = crate::language::LanguageTable::builtin().unwrap();
    let words = |lang: &str| {
        let config = Config::load(&table.find(lang).unwrap(), false).unwrap();
        config.text.usage.purposes["vision"].clone()
    };
    assert_eq!(words("zh"), "多模态");
    assert_eq!(words("en"), "multimodal");
    assert_eq!(words("ja"), "マルチモーダル");
}
