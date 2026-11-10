//! 软件包那一页分组（「配置页」第 43d 条）：一组一块，组名后面暗色写「默认」或「已改」；条件不满足的组不画；停在改过的
//! 一组里按键提示多「D 恢复这一组」。

use ratatui::crossterm::event::KeyCode;
use serde_json::{Value, json};

use super::{press, screen};
use crate::config::Config;
use crate::settings::Settings;
use crate::settings::pages::Section;

fn opened(config: &Config) -> Settings {
    let item = |key: &str, name: &str, section: &str| {
        json!({"key": key, "control": "text", "layers": ["system"], "name": name, "page": "packages",
               "group": "say", "section": section})
    };
    let schema = json!({"pages": [{"id": "packages", "name": "软件包"}],
        "groups": [{"id": "say", "name": "语音播报", "page": "packages", "sections": [
            {"id": "mimo", "name": "MiMo", "shown_when": {"key": "say.engine", "is": ["mimo"]}},
            {"id": "minimax", "name": "MiniMax", "shown_when": {"key": "say.engine", "is": ["minimax"]}},
            {"id": "voice", "name": "声音"}]}],
        "items": [item("say.engine", "引擎", ""), item("say.mimo.key", "MiMo 的 key", "mimo"),
                  item("say.minimax.key", "MiniMax 的 key", "minimax"), item("say.pitch", "音调", "voice")]});
    let written = |v: Value| {
        let layer = json!({"origin": {"layer": "system", "line": 1}, "value": v, "used": true});
        json!({"origin": {"layer": "system"}, "value": v, "layers": [layer]})
    };
    let data = json!({"items": {"say.engine": written(json!("mimo")), "say.mimo.key": written(json!("k"))}});
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => schema.clone(),
            "config.get" => data.clone(),
            "package.list" => json!({"packages": [{"package": "say", "name": "语音播报"}]}),
            _ => json!({}),
        };
        page.answer(tag, Ok(got), &config.text.settings);
    }
    page.on_menu = false;
    page.more.section = Some(Section::Page("packages/say".into(), 0));
    page
}

#[test]
fn each_section_is_a_block_named_with_default_or_changed_and_a_failed_condition_hides_it() {
    let config = Config::builtin().unwrap();
    let mut page = opened(&config);
    let (text, _) = screen(&mut page, &config, 100, 30);
    let line = |word: &str| {
        text.lines()
            .find(|l| l.contains(word))
            .map(str::trim)
            .unwrap_or_default()
            .to_string()
    };
    assert_eq!(line("MiMo  "), "MiMo  已改", "{text}");
    assert_eq!(line("声音"), "声音  默认", "{text}");
    assert!(
        !text.contains("MiniMax"),
        "引擎是 mimo，minimax 那一组不画：\n{text}"
    );
    assert!(
        !text.contains("D 恢复这一组"),
        "停在没分组的引擎上：\n{text}"
    );
    press(&mut page, &config, KeyCode::Char('j'));
    let (text, _) = screen(&mut page, &config, 100, 30);
    assert!(
        text.contains("D 恢复这一组"),
        "停在改过的 MiMo 那一组里：\n{text}"
    );
}
