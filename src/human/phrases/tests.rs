//! 只留原文、用到哪句解哪句，和原来拿到回应就全解成模板的说得一字不差（施工 TM-2）：拿三种语言整份的说法逐句比。

use std::collections::BTreeMap;

use miyu_client::protocol::{Said, Template};
use miyu_client::texts::clean;
use serde_json::{Value, json};

use super::super::Human;
use super::Phrases;

/// 原来的做法：回应一到就把每一句解成模板，解不了的那一句不要（TM-2 以前的 `Human::from_reply`、`Human::say`）。
struct Before(BTreeMap<String, Template>);

impl Before {
    fn from_said(said: &Value) -> Self {
        Self(
            said.as_object()
                .into_iter()
                .flatten()
                .filter_map(|(key, source)| {
                    Some((key.clone(), Template::parse(source.as_str()?).ok()?))
                })
                .collect(),
        )
    }

    fn say(&self, said: &Said) -> Option<String> {
        let template = self.0.get(&said.key)?;
        let fields: BTreeMap<&str, &str> = said
            .fields
            .iter()
            .map(|(field, value)| (field.as_str(), value.as_str()))
            .collect();
        template.fill(&fields, clean).ok()
    }
}

/// 照仓库 `resources/` 读的 `language` 那一种的 `said`，和核心经 `human.get` 交出来的一样（连补上的 `X/one`），再掺几句
/// 坏的、不是字的。
fn said_of(language: &str) -> Value {
    let root = miyu_client::places::ResourceRoot::at(
        std::path::Path::new(miyu_client::WORKSPACE_ROOT).join("resources"),
    );
    let stored = miyu_client::texts::Human::load(&root, language).expect("资源读得懂");
    let mut said: serde_json::Map<String, Value> = stored
        .said_entries()
        .map(|(key, source)| (key.to_string(), json!(source)))
        .collect();
    said.insert("core/bad".into(), json!("{没收尾"));
    said.insert("core/lone".into(), json!("一个 } 单着"));
    said.insert("core/upper".into(), json!("{Count} 个"));
    said.insert("core/number".into(), json!(3));
    said.insert(String::new(), json!("空的键"));
    Value::Object(said)
}

/// 字段的值：带换行、制表、终端控制字符、双向控制字符、尖括号，`clean` 有得清。
const VALUE: &str = "a\nb\tc\u{1b}[31m红\u{202e}反<x>&\"q\"";

#[test]
fn every_phrase_in_every_language_says_the_same_as_before() {
    let mut compared = 0;
    for language in ["zh", "en", "ja"] {
        let said = said_of(language);
        let before = Before::from_said(&said);
        let human = Human::from_reply(&json!({"language": language, "said": said}));
        let keys: Vec<&String> = said.as_object().unwrap().keys().collect();
        assert!(
            keys.len() > 500,
            "{language}：整份说法才几句，读错了：{}",
            keys.len()
        );
        for key in keys {
            let names: Vec<String> = Template::parse(said[key].as_str().unwrap_or("{坏}"))
                .map(|t| t.fields().iter().map(|f| (*f).to_string()).collect())
                .unwrap_or_else(|_| vec!["count".into()]);
            let full: Vec<(String, String)> = names
                .iter()
                .enumerate()
                .map(|(n, name)| (name.clone(), format!("{VALUE}{n}")))
                .collect();
            // 字段全给、少给一个、一个不给、多给一个；键本身、单数那一句、没有的那一句。
            let mut cases = vec![full.clone(), Vec::new()];
            if let Some((_, rest)) = full.split_first() {
                cases.push(rest.to_vec());
            }
            let mut more = full.clone();
            more.push(("extra".into(), "多的".into()));
            cases.push(more);
            for asked in [key.clone(), format!("{key}/one"), format!("{key}/没有")] {
                for fields in &cases {
                    let said = Said {
                        key: asked.clone(),
                        fields: fields.iter().cloned().collect(),
                    };
                    assert_eq!(
                        human.say(&said),
                        before.say(&said),
                        "{language} {asked} {fields:?}"
                    );
                    compared += 1;
                }
            }
        }
    }
    assert!(compared > 10_000, "才比了 {compared} 句");
}

#[test]
fn phrases_are_found_by_key_and_a_singular_like_its_base_is_kept_once() {
    let phrases = Phrases::from_said(&json!({
        "b": "2", "a": "1", "b/one": "2", "c/one": "3", "c": "4", "d/one": "5",
        "e": "6", "e/one": 7, "n": 8
    }));
    assert_eq!(
        phrases.len(),
        6,
        "不是字的不要，和 b 一字不差的 b/one 记在 b 上"
    );
    for (key, source) in [
        ("a", "1"),
        ("b", "2"),
        ("b/one", "2"),
        ("c", "4"),
        ("c/one", "3"),
        ("d/one", "5"),
        ("e", "6"),
    ] {
        assert_eq!(phrases.get(key), Some(source), "{key}");
    }
    for missing in [
        "",
        "0",
        "ab",
        "a/one",
        "c/",
        "d",
        "e/one",
        "n",
        "b/one/one",
        "/one",
    ] {
        assert_eq!(phrases.get(missing), None, "{missing}");
    }
    assert_eq!(Phrases::default().get("a"), None, "没要到的是空的");
}
