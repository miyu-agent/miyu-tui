//! 一组的说明（核心 HZ-1：`config.schema` 的 `groups` 一项写了 `description` 的）：暗色写在组名下面，放不下折行写全；
//! 没写的组照旧只有组名。不认组名，照有没有这一格画。

use serde_json::json;

use crate::config::Config;
use crate::settings::Settings;

#[test]
fn a_groups_description_is_written_dim_under_its_name_and_wraps() {
    let config = Config::builtin().unwrap();
    let texts = &config.text.settings;
    let said = "下面几类命令在完全放开时也会先问你，关掉就不再问。另有几类永远拒绝、不能关。";
    let toggle = |key: &str, name: &str, group: &str| {
        json!({"key": key, "type": "bool", "control": "toggle", "layers": ["system"], "name": name,
               "page": "privacy", "group": group})
    };
    let schema = json!({"pages": [{"id": "privacy", "name": "隐私与安全"}],
        "groups": [{"id": "protected", "name": "受保护的命令", "page": "privacy", "description": said},
                   {"id": "private", "name": "私密目录", "page": "privacy"}],
        "items": [toggle("permission.confirm.git", "破坏性 git 操作", "protected"),
                  toggle("permission.confirm.power", "关机和重启", "protected"),
                  toggle("sandbox.private.enabled", "保护私密目录", "private")]});
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => schema.clone(),
            "config.get" => json!({"items": {}}),
            _ => json!({}),
        };
        page.answer(tag, Ok(got), texts);
    }
    let lines = super::super::pages::page_lines(&page, "privacy", 0, texts, 40);
    let text: Vec<String> = lines.iter().map(|(l, _)| l.to_string()).collect();
    let at = |word: &str| {
        text.iter()
            .position(|l| l.contains(word))
            .unwrap_or_else(|| panic!("没有「{word}」：{text:#?}"))
    };
    // 组名、说明（折成几行，连起来一字不少）、这一组的项；再是下一组。
    let (title, first, next) = (at("受保护的命令"), at("破坏性 git 操作"), at("私密目录"));
    assert!(title < first && first < next, "{text:#?}");
    let note = &lines[title + 1..first];
    assert!(note.len() > 1, "40 列放不下，折行：{text:#?}");
    let joined: String = note.iter().map(|(l, _)| l.to_string()).collect();
    assert_eq!(joined, said);
    for (line, focused) in note {
        assert!(!focused, "说明不是能选的一行");
        assert_eq!(
            line.style.fg.or(line.spans[0].style.fg),
            crate::theme::dim().fg
        );
        assert!(unicode_width::UnicodeWidthStr::width(line.to_string().as_str()) <= 36);
    }
    // 没写说明的组：组名下面直接是项。
    assert!(text[next + 1].contains("保护私密目录"), "{text:#?}");
}
