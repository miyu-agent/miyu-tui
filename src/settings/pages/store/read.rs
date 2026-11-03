//! 读核心的回应（蓝图「配置页」第 43c 条）：`package.search` 的一项（软件源里的包）、`package.list` 的一项（装着的包）、
//! `package.upgrades`、`package.info`。只取三页、预览块用得到的几格；没有的当没有，不报错。

use std::collections::HashMap;

use serde_json::Value;

/// 软件源里的一个包（`package.search` 的一项）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Remote {
    /// 编号。
    pub id: String,
    /// 名字（照连接的语言）；没有的照编号。
    pub name: String,
    /// 软件源里的版本。
    pub version: String,
    /// 一句说明。
    pub summary: String,
    /// 装着的那一份的版本（装着、没写版本的是空的字）；没装的是 `None`。
    pub installed: Option<String>,
    /// 拉黑的理由；没拉黑的是 `None`。
    pub blocked: Option<String>,
    /// 依赖的包的编号。
    pub depends: Vec<String>,
    /// 带了哪几样：种类、名字（索引的原样，照种类排）。
    pub provides: Vec<(String, Vec<String>)>,
    /// 哪个软件源：官方的是 `official`。
    pub source: String,
    /// 这一版第一次进索引的时刻（核心设计 54）。
    pub published: Option<String>,
    /// 一共下载过几次（核心设计 54）。
    pub downloads: Option<u64>,
    /// 近 7 天下载了几次（核心设计 54）。
    pub week: Option<u64>,
}

/// 装着的一个包（`package.list` 的一项）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Local {
    /// 编号。
    pub id: String,
    /// 名字；没有的照编号。
    pub name: String,
    /// 版本；没写的是空的。
    pub version: String,
    /// 一句说明。
    pub summary: String,
    /// 开着：有开关、开关关着的（`enabled: false`），卸掉的出厂包（`removed`）是假的。
    pub on: bool,
    /// 有后台页（`page: true`）。
    pub page: bool,
}

/// 照光标要来的一个包的详情（`package.info`）：预览块照它写。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Info {
    /// 作者。
    pub author: Option<String>,
    /// 上游。
    pub upstream: Option<String>,
    /// 从哪来：`shipped`、`official`、`local`、`source`。
    pub from: Option<String>,
    /// 装的时刻。
    pub installed: Option<String>,
    /// 装着的一共多少字节；没装的是安装后多少字节（`installed_size`）。
    pub size: Option<u64>,
    /// 要下多少字节（没装的）。
    pub download: Option<u64>,
    /// 依赖的包。
    pub depends: Vec<String>,
    /// 装着的包里依赖它的。
    pub required_by: Vec<String>,
    /// 带了哪几样：种类、名字。
    pub provides: Vec<(String, Vec<String>)>,
    /// 扩展声明的能力的名字。
    pub needs: Vec<String>,
}

fn text(v: &Value) -> Option<String> {
    v.as_str().filter(|s| !s.is_empty()).map(str::to_string)
}

fn ids(v: &Value) -> Vec<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|x| x.as_str().map(str::to_string))
        .collect()
}

/// `provides` 是种类到名字：名字是数组、一个字、数、真假的都认（真假、数只写种类）。
fn provides(v: &Value) -> Vec<(String, Vec<String>)> {
    let Some(map) = v.as_object() else {
        return Vec::new();
    };
    let mut out: Vec<(String, Vec<String>)> = map
        .iter()
        .filter_map(|(kind, names)| {
            let names = match names {
                Value::Array(_) => ids(names),
                Value::String(s) => vec![s.clone()],
                Value::Object(o) => o.keys().cloned().collect(),
                Value::Bool(false) | Value::Null => return None,
                _ => Vec::new(),
            };
            Some((kind.clone(), names))
        })
        .collect();
    out.sort();
    out
}

/// 读 `package.search` 的回应。
pub fn remotes(got: &Value) -> Vec<Remote> {
    got["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let id = text(&p["package"])?;
            Some(Remote {
                name: text(&p["name"]).unwrap_or_else(|| id.clone()),
                version: text(&p["version"]).unwrap_or_default(),
                summary: text(&p["summary"]).unwrap_or_default(),
                installed: p["installed"].as_str().map(str::to_string),
                blocked: text(&p["blocked"]),
                depends: ids(&p["depends"]),
                provides: provides(&p["provides"]),
                source: text(&p["source"]).unwrap_or_else(|| "official".into()),
                published: text(&p["published"]),
                downloads: p["downloads"].as_u64(),
                week: p["downloads_week"].as_u64(),
                id,
            })
        })
        .collect()
}

/// 读 `package.list` 的回应：照名字排（2026-10-31 项目主人选的一张平铺的表），一样的照编号。
pub fn locals(got: &Value) -> Vec<Local> {
    let mut out: Vec<Local> = got["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let id = text(&p["package"])?;
            Some(Local {
                name: text(&p["name"]).unwrap_or_else(|| id.clone()),
                version: text(&p["version"]).unwrap_or_default(),
                summary: text(&p["summary"]).unwrap_or_default(),
                on: p["enabled"] != Value::Bool(false) && p["removed"] != Value::Bool(true),
                page: p["page"] == Value::Bool(true),
                id,
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    out
}

/// 读 `package.upgrades` 的回应：能升的，编号到新版本。
pub fn upgrades(got: &Value) -> HashMap<String, String> {
    got["upgrades"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|u| Some((text(&u["package"])?, text(&u["to"]).unwrap_or_default())))
        .collect()
}

/// 读 `package.info` 的回应。
pub fn info(got: &Value) -> Info {
    let needs = got["capabilities"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            text(&c["name"])
                .or_else(|| text(&c["id"]))
                .or_else(|| text(c))
        })
        .collect();
    Info {
        author: text(&got["author"]),
        upstream: text(&got["upstream"]),
        from: text(&got["from"]),
        installed: text(&got["installed"]),
        size: got["size"]
            .as_u64()
            .or_else(|| got["installed_size"].as_u64()),
        download: got["download"].as_u64(),
        depends: ids(&got["depends"]),
        required_by: ids(&got["required_by"]),
        provides: provides(&got["provides"]),
        needs,
    }
}

#[cfg(test)]
mod tests;
