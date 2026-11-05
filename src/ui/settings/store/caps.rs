//! 装、升级问的时候说清来源和能力（核心 PK-3 上，设计 61 第三节，同核心命令行的确认屏）：来源照每个包的 `source`、`feed`
//! 写，都一样的写一个；能力一项一行「名字：说明」，批过的（`held`）接「（已批准）」、暗着，要批的警示色；几个包都要能力的前面
//! 接包名；社区的、本机文件的接核心给的 `notice`（「注意：」打头，照原样）。

use ratatui::style::Style;
use ratatui::text::Line;
use serde_json::Value;

use super::preview::name_of;
use crate::settings::pages::store::{Store, StoreTexts};
use crate::theme;

/// 一个包从哪来：官方软件源、社区（别人加的源写源名）、本机文件、出厂自带、系统安装；旧核心（`source` 是源的名字）的照
/// 那个名字写。
fn from(one: &Value, texts: &StoreTexts) -> Option<String> {
    let a = |key: &str| StoreTexts::say(&texts.ask, key);
    let source = one["source"].as_str()?;
    let feed = one["feed"].as_str();
    Some(match (source, feed) {
        ("community", Some(feed)) if feed != "official" => feed.to_string(),
        ("official" | "community" | "file" | "shipped" | "distro", _) => {
            a(&format!("from_{source}"))
        }
        (other, _) => other.to_string(),
    })
}

/// 「来源」那一格：几个包从哪来，一样的只写一个。
pub fn sources(items: &[&Value], texts: &StoreTexts) -> String {
    let mut all: Vec<String> = Vec::new();
    for label in items.iter().filter_map(|one| from(one, texts)) {
        if !all.contains(&label) {
            all.push(label);
        }
    }
    all.join(&StoreTexts::say(&texts.values, "sep"))
}

/// 能力一项一行；`fresh` 的只列没批过的（升级只说新要的）。
pub fn capabilities(
    store: &Store,
    items: &[&Value],
    fresh: bool,
    texts: &StoreTexts,
) -> Vec<(String, Style)> {
    let a = |key: &str| StoreTexts::say(&texts.ask, key);
    let wanted = |c: &&Value| !fresh || c["held"] != true;
    let with_caps = items
        .iter()
        .filter(|one| list(&one["capabilities"]).any(|c| wanted(&c)))
        .count();
    let mut out = Vec::new();
    for one in items {
        for c in list(&one["capabilities"]).filter(wanted) {
            let name = c["name"].as_str().or(c["id"].as_str()).unwrap_or_default();
            let mut said = match c["summary"].as_str().filter(|s| !s.is_empty()) {
                Some(summary) => a("cap")
                    .replace("{name}", name)
                    .replace("{summary}", summary),
                None => name.to_string(),
            };
            if with_caps > 1 {
                let package = name_of(store, one["package"].as_str().unwrap_or_default());
                said = format!("{package}{}{said}", crate::sheet::sep());
            }
            let held = c["held"] == true;
            if held {
                said.push_str(&a("held"));
            }
            out.push((said, if held { theme::dim() } else { theme::warn() }));
        }
    }
    out
}

/// 核心给的「注意」：几份看一眼里的句子连成一行，前面接「注意：」；没有的不写。
pub fn notice(previews: &[Value], texts: &StoreTexts) -> Option<Line<'static>> {
    let a = |key: &str| StoreTexts::say(&texts.ask, key);
    let mut sentences: Vec<&str> = Vec::new();
    for s in previews.iter().flat_map(|p| list(&p["notice"])) {
        if let Some(s) = s.as_str().filter(|s| !sentences.contains(s)) {
            sentences.push(s);
        }
    }
    (!sentences.is_empty()).then(|| {
        let text = a("notice").replace("{text}", &sentences.join(&a("notice_join")));
        Line::styled(text, theme::warn())
    })
}

fn list(v: &Value) -> impl Iterator<Item = &Value> {
    v.as_array().into_iter().flatten()
}
