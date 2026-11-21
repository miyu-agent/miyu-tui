//! 小读法本身：每种格子、每条 serde 规矩各一个小类型，和派生的比（资源里没用到的写法也在这里守着）。

use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;
use serde_json::{Value, json};

use super::{resource, same_text, same_value};
use crate::from_json::{Kind, Node, Read, choice, record};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Level {
    Full,
    ReadOnly,
}

choice! { Level { Full = "full", ReadOnly = "read_only" } }

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Body {
    None,
    Arg(String),
    Diff([String; 2]),
}

choice! { Body { None = "none", Arg(String) = "arg", Diff([String; 2]) = "diff" } }

/// 查写法的字（同颜色）。
#[derive(Debug, PartialEq)]
struct Even(String);

fn even(text: &str) -> Result<(), String> {
    if text.len().is_multiple_of(2) {
        Ok(())
    } else {
        Err(format!("单数个字：{text}"))
    }
}

impl<'de> Deserialize<'de> for Even {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        even(&text).map_err(serde::de::Error::custom)?;
        Ok(Even(text))
    }
}

impl Read for Even {
    const KIND: &'static Kind = &Kind::Checked(even);

    fn build(node: Node) -> Self {
        Even(String::build(node))
    }
}

fn seven() -> u8 {
    7
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inner {
    a: String,
    #[serde(default)]
    b: Vec<u16>,
}

record! { Inner, deny { a, b(default) } }

#[derive(Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Small {
    name: String,
    #[serde(default)]
    count: u16,
    #[serde(default = "seven")]
    seven: u8,
    #[serde(rename = "as")]
    as_: Option<String>,
    #[serde(rename = "loop", default = "seven")]
    loop_: u8,
    list: Vec<String>,
    pair: [String; 2],
    nested: Vec<[[f64; 2]; 2]>,
    levels: HashMap<Level, String>,
    sorted: BTreeMap<String, i32>,
    chars: HashMap<char, char>,
    inner: Option<Inner>,
    inners: Vec<Inner>,
    bodies: HashMap<String, Body>,
    even: Even,
    ratio: f32,
    size: usize,
    big: u64,
    flag: bool,
    mark: char,
    raw: Option<Value>,
    #[serde(skip)]
    skipped: Vec<String>,
}

record! {
    Small, deny {
        name, count(default), seven(default = seven), as_("as"), loop_("loop", default = seven), list, pair,
        nested, levels, sorted, chars, inner, inners, bodies, even, ratio, size, big, flag, mark, raw,
    }
    skip { skipped }
}

#[derive(Debug, PartialEq, Deserialize)]
struct Loose {
    a: String,
    #[serde(default)]
    b: Option<Level>,
}

record! { Loose { a, b(default) } }

#[derive(Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Heading {
    title: String,
    sub: String,
}

record! { Heading, deny { title, sub } }

#[derive(Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Flat {
    #[serde(flatten)]
    head: Heading,
    auto: String,
    #[serde(default)]
    more: Vec<String>,
}

record! { Flat, deny { auto, more(default) } flatten head }

#[derive(Debug, PartialEq, Deserialize)]
struct FlatLoose {
    #[serde(flatten)]
    head: Heading,
    names: HashMap<String, String>,
}

record! { FlatLoose { names } flatten head }

fn small() -> Value {
    json!({
        "name": "n", "count": 3, "as": "a", "list": ["x", "y"], "pair": ["p", "q"],
        "nested": [[[1.0, 2], [3, 4.5]]], "levels": {"full": "F", "read_only": "R"},
        "sorted": {"b": -2, "a": 1}, "chars": {"─": "-", "│": "|"},
        "inner": {"a": "i", "b": [1, 2]}, "inners": [{"a": "j"}],
        "bodies": {"x": "none", "y": {"arg": "command"}, "z": {"diff": ["old", "new"]}},
        "even": "ab", "ratio": 0.5, "size": 12, "big": 18_446_744_073_709_551_615_u64, "flag": true,
        "mark": "•", "raw": {"any": [1, "thing"]}
    })
}

#[test]
fn every_kind_reads_the_same_as_serde() {
    let good = small();
    assert!(same_value::<Small>("小的", &good));
    let text = serde_json::to_string_pretty(&good).unwrap();
    assert!(same_text::<Small>("小的原文", &text));
    let (count, broken) = resource::<Small>("小的", &text);
    assert!(broken > count / 2, "写坏的样本大多读不成：{broken}/{count}");
}

#[test]
fn hand_written_bad_ones_say_the_same() {
    let bad = [
        r#"{"name": 1}"#,
        r#"{"name": "n", "name": "m"}"#,
        r#"{"name": "n", "zz": 1, "list": 5}"#,
        r#"{"list": 5, "zz": 1}"#,
        r#"[]"#,
        r#"["n", 1]"#,
        r#""n""#,
        r#"{"name": "n", "levels": {"half": "H"}}"#,
        r#"{"name": "n", "bodies": {"x": "arg"}}"#,
        r#"{"name": "n", "bodies": {"x": {"nope": 1}}}"#,
        r#"{"name": "n", "bodies": {"x": {"none": null}}}"#,
        r#"{"name": "n", "bodies": {"x": {"none": 1}}}"#,
        r#"{"name": "n", "bodies": {"x": 3}}"#,
        r#"{"name": "n", "pair": ["a", "b", "c"]}"#,
        r#"{"name": "n", "pair": ["a"]}"#,
        r#"{"name": "n", "even": "abc"}"#,
        r#"{"name": "n", "chars": {"ab": "c"}}"#,
        r#"{"name": "n", "size": -1}"#,
        r#"{"name": "n", "ratio": "1"}"#,
        r#"{"name": "n", "inner": {"a": "i", "a": "j"}}"#,
        "{\"name\": \"n\",\n  \"count\": 70000\n}",
        r#"{"name": "n",}"#,
        r#"{"name": "n"} x"#,
    ];
    for json in bad {
        same_text::<Small>(json, json);
        if let Ok(value) = serde_json::from_str::<Value>(json) {
            same_value::<Small>(json, &value);
        }
    }
}

#[test]
fn loose_structs_skip_unknown_keys() {
    for json in [
        r#"{"a": "x", "zz": {"deep": [1]}}"#,
        r#"{"a": "x", "b": "full"}"#,
        r#"{"a": "x", "b": null}"#,
        r#"{"a": "x", "b": "none"}"#,
        r#"{"b": "full"}"#,
        r#"["x"]"#,
        r#"["x", "full", 3]"#,
    ] {
        same_text::<Loose>(json, json);
        same_value::<Loose>(json, &serde_json::from_str(json).unwrap());
    }
}

#[test]
fn flattened_fields_follow_serde() {
    for json in [
        r#"{"title": "t", "sub": "s", "auto": "a"}"#,
        r#"{"auto": "a", "sub": "s", "title": "t", "more": ["m"]}"#,
        r#"{"title": "t", "auto": "a"}"#,
        r#"{"title": 5, "sub": "s", "auto": "a"}"#,
        r#"{"title": "t", "sub": "s", "auto": "a", "zz": 1}"#,
        r#"{"title": "t", "sub": "s", "zz": 1}"#,
        r#"{"title": "t", "title": "u", "sub": "s", "auto": "a"}"#,
        r#"{"title": "t", "sub": "s", "auto": 1}"#,
        r#"["t", "s", "a"]"#,
        r#"{}"#,
    ] {
        same_text::<Flat>(json, json);
        same_value::<Flat>(json, &serde_json::from_str(json).unwrap());
    }
    for json in [
        r#"{"title": "t", "sub": "s", "names": {"a": "b"}, "zz": 1}"#,
        r#"{"sub": "s", "names": {}}"#,
    ] {
        same_text::<FlatLoose>(json, json);
        same_value::<FlatLoose>(json, &serde_json::from_str(json).unwrap());
    }
}
