//! 读核心给的一块（`widgets.md`「头」第 3 条）：认不得的段整段不要、认不得的语气当普通、认不得的状态不写记号、认不得的
//! 动作当不能点；以后的核心多了几种也照样画得出认得的那些。

use serde_json::Value;

use super::{Action, Block, Button, Item, Pair, Place, Segment, State, Tone};

/// 订阅回应里的 `widgets`。
pub fn blocks(result: &Value) -> Vec<Block> {
    result["widgets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(block)
        .collect()
}

/// 一块；缺了包、编号，位置认不得的是 `None`。
pub fn block(value: &Value) -> Option<Block> {
    let place = match value["place"].as_str()? {
        "home" => Place::Home,
        "sidebar" => Place::Sidebar,
        "status" => Place::Status,
        "page" => Place::Page,
        _ => return None,
    };
    let package = text(&value["package"]).filter(|p| !p.is_empty())?;
    let id = text(&value["id"]).filter(|i| !i.is_empty())?;
    Some(Block {
        source: text(&value["source"]).unwrap_or_else(|| package.clone()),
        package,
        id,
        place,
        title: text(&value["title"]),
        icon: text(&value["icon"]).filter(|i| !i.is_empty()),
        running: value["running"].as_bool().unwrap_or(false),
        content: value["content"]
            .as_array()
            .map(|segments| segments.iter().filter_map(segment).collect()),
    })
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

fn segment(value: &Value) -> Option<Segment> {
    let items = || value["items"].as_array().into_iter().flatten();
    Some(match value["type"].as_str()? {
        "heading" => Segment::Heading(text(&value["text"])?),
        "text" => Segment::Text {
            text: text(&value["text"])?,
            tone: tone(&value["tone"]),
        },
        "pairs" => Segment::Pairs(
            items()
                .filter_map(|p| {
                    Some(Pair {
                        label: text(&p["label"])?,
                        value: text(&p["value"])?,
                        tone: tone(&p["tone"]),
                    })
                })
                .collect(),
        ),
        "progress" => Segment::Progress {
            value: fraction(&value["value"])?,
            text: text(&value["text"]).filter(|t| !t.is_empty()),
            mark: fraction(&value["mark"]),
        },
        "list" => Segment::List(
            items()
                .filter_map(|i| {
                    Some(Item {
                        title: text(&i["title"])?,
                        detail: text(&i["detail"]).filter(|t| !t.is_empty()),
                        right: text(&i["right"]).filter(|t| !t.is_empty()),
                        state: state(&i["state"]),
                        action: action(&i["action"]),
                    })
                })
                .collect(),
        ),
        "buttons" => Segment::Buttons(
            items()
                .filter_map(|b| {
                    Some(Button {
                        label: text(&b["label"])?,
                        action: action(&b["action"])?,
                    })
                })
                .collect(),
        ),
        _ => return None,
    })
}

/// 0 到 1 之间的数；超出的夹回来。
fn fraction(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .filter(|v| v.is_finite())
        .map(|v| v.clamp(0.0, 1.0))
}

fn tone(value: &Value) -> Tone {
    match value.as_str() {
        Some("dim") => Tone::Dim,
        Some("accent") => Tone::Accent,
        Some("good") => Tone::Good,
        Some("warn") => Tone::Warn,
        Some("bad") => Tone::Bad,
        _ => Tone::Normal,
    }
}

fn state(value: &Value) -> Option<State> {
    Some(match value.as_str()? {
        "todo" => State::Todo,
        "doing" => State::Doing,
        "done" => State::Done,
        "failed" => State::Failed,
        "off" => State::Off,
        _ => return None,
    })
}

fn action(value: &Value) -> Option<Action> {
    if let Some(method) = value["call"].as_str() {
        let params = value
            .get("params")
            .cloned()
            .filter(Value::is_object)
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        return Some(Action::Call {
            method: method.to_string(),
            params,
        });
    }
    if let Some(page) = value["open"].as_str() {
        return Some(Action::Open(page.to_string()));
    }
    value["copy"].as_str().map(|t| Action::Copy(t.to_string()))
}
