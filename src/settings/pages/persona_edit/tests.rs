//! 人格的窗：改提示词、简单描述、子代理可选（蓝图「配置页」第 36 条）。

use serde_json::json;

use super::PromptEdit;
use crate::settings::Settings;
use crate::settings::pages::choose::PersonaView;

/// 一个交给了编辑器的人设：原文 `你是她。`，你那一层的版本 `v1`。
fn editing() -> Settings {
    Settings {
        prompt: Some(PromptEdit {
            persona: "mine".into(),
            prompt: "persona".into(),
            original: "你是她。".into(),
            version: Some("v1".into()),
            view: PersonaView {
                id: "mine".into(),
                detail: None,
                error: None,
                cursor: 3,
            },
        }),
        ..Settings::default()
    }
}

#[test]
fn an_unchanged_prompt_is_not_saved_and_a_changed_one_carries_its_version() {
    let mut page = editing();
    page.prompt_edited(Some("你是她。\n".into()));
    assert!(page.take_asks().is_empty(), "编辑器补的换行不算改了");
    let mut page = editing();
    page.prompt_edited(None);
    assert!(page.take_asks().is_empty(), "读不回来的不存");
    let mut page = editing();
    page.prompt_edited(Some("你是另一个她。\n".into()));
    let asks = page.take_asks();
    assert_eq!(asks[0].1, "persona.set");
    assert_eq!(
        asks[0].2,
        json!({"persona": "mine", "prompts": {"persona": {"text": "你是另一个她。\n", "expect": "v1"}}})
    );
}

#[test]
fn a_saved_prompt_says_so() {
    // 2026-10-13 验收 T-189：存成了状态行没有一句话，还写着别处的「已获取 5 个模型」。
    let texts = crate::config::Config::builtin().unwrap().text.settings;
    let mut page = editing();
    page.prompt_edited(Some("你是另一个她。".into()));
    let (tag, _, _) = page.take_asks().remove(0);
    page.answer(tag, Ok(json!({"persona": {"id": "mine"}})), &texts);
    let (said, tone) = page.status.clone().unwrap();
    assert_eq!(said, "已保存");
    assert_eq!(tone, crate::settings::Tone::Good);
}

#[test]
fn an_emptied_prompt_is_saved_empty_not_put_back() {
    // 核心 P-3 补：`unset` 是回到出厂的字，清空了要的是空的。
    let mut page = editing();
    page.prompt_edited(Some("\n".into()));
    let asks = page.take_asks();
    assert_eq!(
        asks[0].2["prompts"]["persona"],
        json!({"text": "", "expect": "v1"})
    );
}

#[test]
fn the_short_description_sits_under_the_name_and_is_saved_as_the_summary() {
    // 「配置页」第 36 条（2026-10-17 项目主人定）：简单描述就是 `persona.summary`，名字下面一行。
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{FIELDS, Field};
    use crate::settings::pages::personas::Detail;
    use crate::settings::popup::Popup;
    assert_eq!(FIELDS[1], Field::Summary, "名字下面一行");
    let texts = crate::config::Config::builtin().unwrap().text.settings;
    let detail = Detail {
        id: "mine".into(),
        summary: Some("会写诗".into()),
        ..Detail::default()
    };
    let mut page = Settings {
        popup: Some(Popup::Persona(PersonaView {
            id: "mine".into(),
            detail: Some(detail),
            error: None,
            cursor: 1,
        })),
        ..Settings::default()
    };
    let press = |page: &mut Settings, code| {
        page.key(KeyEvent::new(code, KeyModifiers::NONE), &texts);
    };
    press(&mut page, KeyCode::Enter);
    let Some(Popup::Line(edit)) = &page.popup else {
        panic!("开编辑窗：{:?}", page.popup);
    };
    assert_eq!(
        (edit.key.as_str(), edit.editor.text()),
        ("persona.summary", "会写诗")
    );
    // 光标停在末尾、不选中（2026-10-20 项目主人：整段选中一敲就替换掉全部的字，不对）：先删掉原来的三个字。
    assert_eq!(edit.editor.selection(), None);
    for _ in 0..3 {
        press(&mut page, KeyCode::Backspace);
    }
    for c in "写代码的".chars() {
        press(&mut page, KeyCode::Char(c));
    }
    press(&mut page, KeyCode::Enter);
    let asks = page.take_asks();
    assert_eq!(asks[0].1, "persona.set");
    assert_eq!(
        asks[0].2,
        json!({"persona": "mine", "changes": [{"key": "persona.summary", "value": "写代码的"}]})
    );
}

#[test]
fn subagent_selectable_is_a_switch_saved_at_once() {
    // 核心 P-10 补（2026-10-17 项目主人定）：人格编辑里一个开关「子代理可选」，默认关；回车切、当场存。
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{FIELDS, Field};
    use crate::settings::pages::personas::{Detail, detail};
    use crate::settings::popup::Popup;
    assert!(!detail(&json!({"persona": "mine"})).subagent, "没写的是关");
    assert!(detail(&json!({"persona": "mine", "subagent": true})).subagent);
    let texts = crate::config::Config::builtin().unwrap().text.settings;
    let at = FIELDS.iter().position(|f| *f == Field::Subagent).unwrap();
    assert_eq!(at, 2, "简单描述下面");
    let mut page = Settings {
        popup: Some(Popup::Persona(PersonaView {
            id: "mine".into(),
            detail: Some(Detail {
                id: "mine".into(),
                ..Detail::default()
            }),
            error: None,
            cursor: at,
        })),
        ..Settings::default()
    };
    page.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &texts);
    assert!(matches!(page.popup, Some(Popup::Persona(_))), "不开编辑窗");
    let asks = page.take_asks();
    assert_eq!(asks[0].1, "persona.set");
    assert_eq!(
        asks[0].2,
        json!({"persona": "mine", "changes": [{"key": "persona.subagent", "value": true}]})
    );
}
