//! 对照测试（施工 LW-9）：出厂的每份资源照程序里读它的样子（原文、纯 ASCII 主题换过字的原文、盖上主题的、挑了语言的
//! JSON 值），新读法和 serde 派生的旧读法逐份比，再加照它造的写坏的样本（`from_json/tests`）。

use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::from_json::Read;
use crate::from_json::tests::{resource, same_value};
use crate::language::LanguageTable;

/// 比一份，印比了多少样本（`--nocapture` 看得到）。
fn check<T: Read + DeserializeOwned + PartialEq + Debug>(label: &str, json: &str) {
    let (count, broken) = resource::<T>(label, json);
    assert!(broken > 0, "{label}：写坏的样本总有读不成的");
    eprintln!("{label}：比了 {count} 个写坏的样本，{broken} 个两边都读不成");
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap()
}

fn texts() -> [(&'static str, &'static str); 3] {
    [
        ("zh", include_str!("../../resources/text/zh.json")),
        ("en", include_str!("../../resources/text/en.json")),
        ("ja", include_str!("../../resources/text/ja.json")),
    ]
}

#[test]
fn texts_read_the_same() {
    for (code, json) in texts() {
        check::<super::Texts>(&format!("text/{code}.json"), json);
    }
}

#[test]
fn ascii_texts_read_the_same() {
    for (code, json) in texts() {
        check::<super::Texts>(
            &format!("纯 ASCII 的 text/{code}.json"),
            &crate::ascii::plain(json),
        );
    }
}

#[test]
fn summaries_read_the_same() {
    for (code, json) in texts() {
        let value: Value = serde_json::from_str(json).unwrap();
        let summary = value.get("summary").cloned().unwrap_or_default();
        check::<super::Summary>(&format!("text/{code}.json 的 summary"), &pretty(&summary));
        assert!(!same_value::<super::Summary>("没有 summary", &Value::Null));
    }
}

/// 写了几种语言的资源挑了 `code` 那一种（同 `localize::localized`）。
fn picked(json: &str, code: &str, ascii: bool) -> String {
    let table = LanguageTable::builtin().unwrap();
    let text = if ascii {
        crate::ascii::plain(json)
    } else {
        json.into()
    };
    let mut value: Value = serde_json::from_str(&text).unwrap();
    super::localize::pick(&mut value, code, &table);
    pretty(&value)
}

#[test]
fn commands_and_pulse_read_the_same() {
    for code in ["zh", "en", "ja"] {
        for ascii in [false, true] {
            let commands = picked(include_str!("../../resources/commands.json"), code, ascii);
            check::<crate::commands::Commands>(&format!("commands.json {code} {ascii}"), &commands);
            let pulse = picked(include_str!("../../resources/pulse.json"), code, ascii);
            check::<crate::pulse::Words>(&format!("pulse.json {code} {ascii}"), &pulse);
        }
    }
}

/// 出厂的两个主题的整份：默认的、纯 ASCII 的。
fn themes() -> [(&'static str, Value); 2] {
    [
        ("默认主题", crate::sheet::default_value()),
        (
            "纯 ASCII 主题",
            crate::sheet::builtin(crate::sheet::ASCII).unwrap(),
        ),
    ]
}

#[test]
fn themed_layout_and_timeline_read_the_same() {
    for (name, theme) in themes() {
        let layout = crate::sheet::overlay(
            include_str!("../../resources/layout.json"),
            "layout",
            &theme,
        );
        check::<super::Layout>(&format!("{name} 的 layout.json"), &pretty(&layout.unwrap()));
        let timeline = crate::sheet::overlay(
            include_str!("../../resources/timeline.json"),
            "timeline",
            &theme,
        );
        check::<super::Timeline>(
            &format!("{name} 的 timeline.json"),
            &pretty(&timeline.unwrap()),
        );
        check::<crate::sheet::Sheet>(&format!("{name} 的表"), &pretty(&theme));
    }
}

#[test]
fn plain_resources_read_the_same() {
    check::<crate::markdown::Languages>("code.json", include_str!("../../resources/code.json"));
    check::<crate::markdown::Math>("math.json", include_str!("../../resources/math.json"));
    check::<super::FigureLook>("figures.json", include_str!("../../resources/figures.json"));
    check::<super::NotifyLook>("notify.json", include_str!("../../resources/notify.json"));
    check::<super::MentionLook>("mention.json", include_str!("../../resources/mention.json"));
    check::<super::AttachLook>(
        "attachments.json",
        include_str!("../../resources/attachments.json"),
    );
    check::<crate::avatars::shrink::Look>(
        "avatar.json",
        include_str!("../../resources/avatar.json"),
    );
    check::<LanguageTable>(
        "languages.json",
        include_str!("../../resources/languages.json"),
    );
}

#[test]
fn looks_read_the_same() {
    check::<crate::mascot::Look>("mascot.json", include_str!("../../resources/mascot.json"));
    check::<crate::oobe::look::Look>("oobe.json", include_str!("../../resources/oobe.json"));
}

#[test]
fn icons_and_palettes_read_the_same() {
    for (name, json) in [
        ("nerd", include_str!("../../resources/icons/nerd.json")),
        ("plain", include_str!("../../resources/icons/plain.json")),
        ("ascii", include_str!("../../resources/icons/ascii.json")),
    ] {
        check::<super::Icons>(&format!("icons/{name}.json"), json);
    }
    for (name, json) in [
        (
            "tokyonight-night",
            include_str!("../../resources/themes/tokyonight-night.json"),
        ),
        (
            "tokyonight-day",
            include_str!("../../resources/themes/tokyonight-day.json"),
        ),
        (
            "terminal",
            include_str!("../../resources/themes/terminal.json"),
        ),
        (
            "terminal-light",
            include_str!("../../resources/themes/terminal-light.json"),
        ),
    ] {
        check::<crate::theme::Palette>(&format!("themes/{name}.json"), json);
    }
}

#[test]
fn slots_read_the_same() {
    for slot in [
        json!({"part": "recent"}),
        json!({"part": "recent", "shown": false, "rows": 3}),
        json!({"part": "recent", "rows": 70000}),
        json!({"part": "recent", "hidden": true}),
        json!({"shown": true}),
        json!("recent"),
        json!(["recent", true]),
    ] {
        same_value::<super::slots::Slot>(&slot.to_string(), &slot);
    }
}
