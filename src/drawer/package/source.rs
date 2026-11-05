//! 装的时候说清来源和能力（核心 PK-3 上，设计 61 第三节，同核心命令行的确认屏）：「来源」照预览的 `source`、`feed`（官方
//! 软件源、社区、别人加的源写源名、本机文件、出厂自带、系统安装），「需要」里批过的（`held`）接「（已批准）」，社区的、本机
//! 文件的接核心给的 `notice`。她调 `package` 工具时人答「允许本次」，核心照这张卡上的预览自己带 `approve`，这里只管写清楚。

use serde_json::Value;

use super::PackageTexts;

/// 从哪来；没有 `source` 的（旧核心的卸、开扩展）不写。旧核心装的 `source` 是源的名字，照那个名字写。
pub fn from(preview: &Value, texts: &PackageTexts) -> Option<String> {
    let source = preview["source"].as_str()?;
    let feed = preview["feed"].as_str();
    Some(match (source, feed) {
        ("community", Some(feed)) if feed != "official" => feed.to_string(),
        _ => texts
            .sources
            .get(source)
            .cloned()
            .unwrap_or_else(|| source.to_string()),
    })
}

/// 要的能力：名字，批过的接「（已批准）」；要装的那个加一起装的依赖。
pub fn needs(preview: &Value, texts: &PackageTexts) -> Vec<String> {
    let deps = list(&preview["with"]).flat_map(|w| list(&w["capabilities"]));
    list(&preview["capabilities"])
        .chain(deps)
        .filter_map(|c| {
            let name = c["name"].as_str()?;
            Some(if c["held"] == true {
                format!("{name}{}", texts.held)
            } else {
                name.to_string()
            })
        })
        .collect()
}

/// 核心给的「注意」：句子连成一行；没有的是空的。
pub fn notice(preview: &Value, texts: &PackageTexts) -> Vec<String> {
    let all: Vec<&str> = list(&preview["notice"]).filter_map(Value::as_str).collect();
    if all.is_empty() {
        Vec::new()
    } else {
        vec![all.join(&texts.notice_join)]
    }
}

fn list(v: &Value) -> impl Iterator<Item = &Value> {
    v.as_array().into_iter().flatten()
}
