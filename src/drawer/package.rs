//! 装、卸软件包，打开扩展的确认（蓝图 `tui.md`「确认和提问的抽屉」第 3 条，核心 F-10 下）：照 `miyu pkg install` 印的那一份写，
//! 问题行是做哪一件，下面几行是包含、读不成、需要、一并删除、大小，照 `detail.preview`（核心看一眼的回应）。

use serde::Deserialize;
use serde_json::Value;

use super::event::Detail;

/// 这几行上的字（`text/zh.json` 的 `drawer.package`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageTexts {
    /// 问题行：安装、装回卸掉的出厂包、卸载、打开扩展，`{id}`。
    pub install: String,
    /// 见 `install`。
    pub restore: String,
    /// 见 `install`。
    pub remove: String,
    /// 见 `install`。
    pub enable: String,
    /// 升级的接在问题行后面：换下的版本 `{version}`。
    pub replacing: String,
    /// 换下的那一份没写版本的。
    pub replacing_unknown: String,
    /// 带的程序：`ui`、`process`、`worker`、`builtin`。
    pub programs: std::collections::HashMap<String, String>,
    /// 带的子命令，`{name}`。
    pub command: String,
    /// 带后台页。
    pub page: String,
    /// 带吉祥物。
    pub mascot: String,
    /// 接的平台，`{platform}`。
    pub connection: String,
    /// 几项设置，`{n}`：一项的、几项的。
    pub settings: [String; 2],
    /// MCP 服务、技能、脚本，`{name}`。
    pub mcp: String,
    /// 见 `mcp`。
    pub skill: String,
    /// 见 `mcp`。
    pub script: String,
    /// 包里带的斜杠命令（命令模板、扩展声明的，核心 X-14），`{name}`。
    pub slash: String,
    /// 一行打头的：包含、读不成、需要、一并删除、大小。
    pub includes: String,
    /// 见 `includes`。
    pub skipped: String,
    /// 见 `includes`。
    pub needs: String,
    /// 见 `includes`。
    pub also_deletes: String,
    /// 见 `includes`。
    pub size: String,
    /// 读不成的一个，`{file}`、`{why}`（为什么是核心给的英文一句）。
    pub skipped_item: String,
    /// 一并删除的设置，`{keys}`。
    pub deleted_settings: String,
    /// 一并删除的状态目录。
    pub state_dir: String,
    /// 多大、几个文件，`{size}`、`{n}`：一个文件的、几个的。
    pub files: [String; 2],
    /// 一行里几样之间隔开的。
    pub separator: String,
}

/// 写出来的样子：问题行，下面几行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// 问题行。
    pub title: String,
    /// 下面几行，没有的那一行不在。
    pub rows: Vec<String>,
}

/// 做哪一件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Doing {
    Install,
    Remove,
    Enable,
}

/// 装、卸、打开扩展的确认写成的样子；`detail` 里这三样都没有的不是这一种，是 `None`。编号、版本照 `preview`，没有 `preview`
/// 的照 `detail` 里写的（装的写路径最后一截）。
pub fn plan(detail: &Detail, texts: &PackageTexts) -> Option<Plan> {
    let (doing, named) = match (&detail.install, &detail.remove, &detail.enable) {
        (Some(path), _, _) => (Doing::Install, folder(path)),
        (None, Some(id), _) => (Doing::Remove, id.clone()),
        (None, None, Some(id)) => (Doing::Enable, id.clone()),
        (None, None, None) => return None,
    };
    let none = Value::Null;
    let preview = detail.preview.as_ref().unwrap_or(&none);
    let mut id = preview["package"].as_str().map_or(named, str::to_string);
    if let Some(version) = preview["version"].as_str() {
        id = format!("{id} {version}");
    }
    let template = match doing {
        Doing::Install if preview["restores"] == true => &texts.restore,
        Doing::Install => &texts.install,
        Doing::Remove => &texts.remove,
        Doing::Enable => &texts.enable,
    };
    let mut title = template.replace("{id}", &id);
    if let Some(replaces) = preview.get("replaces") {
        title.push_str(&match replaces["version"].as_str() {
            Some(version) => texts.replacing.replace("{version}", version),
            None => texts.replacing_unknown.clone(),
        });
    }
    let mut rows = Vec::new();
    let mut row = |label: &str, items: Vec<String>| {
        if !items.is_empty() {
            rows.push(format!("{label}{}", items.join(&texts.separator)));
        }
    };
    row(&texts.includes, included(preview, doing, texts));
    let skipped = list(&preview["skipped"])
        .map(|one| {
            texts
                .skipped_item
                .replace("{file}", one["file"].as_str().unwrap_or_default())
                .replace("{why}", one["why"].as_str().unwrap_or_default())
        })
        .collect();
    row(&texts.skipped, skipped);
    let needs = list(&preview["capabilities"])
        .filter_map(|capability| capability["name"].as_str())
        .map(str::to_string)
        .collect();
    row(&texts.needs, needs);
    if doing == Doing::Remove {
        row(&texts.also_deletes, deleted(preview, texts));
    }
    if let (Some(files), Some(bytes)) = (preview["files"].as_u64(), preview["size"].as_u64()) {
        let line = counted(files, &texts.files).replace("{size}", &size(bytes));
        row(&texts.size, vec![line]);
    }
    Some(Plan { title, rows })
}

/// 「包含」那一行：程序、子命令、后台页、吉祥物、接的平台、MCP 服务、技能、脚本、几项设置（卸的时候设置算在
/// 「一并删除」里）。照命令行的先后。
fn included(preview: &Value, doing: Doing, texts: &PackageTexts) -> Vec<String> {
    let mut items = Vec::new();
    if let Some(kind) = preview["program"].as_str() {
        items.push(
            texts
                .programs
                .get(kind)
                .map_or(kind, String::as_str)
                .to_string(),
        );
    }
    if let Some(name) = preview["command"].as_str() {
        items.push(texts.command.replace("{name}", name));
    }
    if preview["page"] == true {
        items.push(texts.page.clone());
    }
    if preview["mascot"] == true {
        items.push(texts.mascot.clone());
    }
    if let Some(platform) = preview["connection"].as_str() {
        items.push(texts.connection.replace("{platform}", platform));
    }
    for (key, template) in [
        ("mcp", &texts.mcp),
        ("skills", &texts.skill),
        ("scripts", &texts.script),
        ("slash", &texts.slash),
    ] {
        let names = list(&preview[key]).filter_map(Value::as_str);
        items.extend(names.map(|name| template.replace("{name}", name)));
    }
    if let Some(n) = preview["settings"]
        .as_u64()
        .filter(|_| doing != Doing::Remove)
    {
        items.push(counted(n, &texts.settings));
    }
    items
}

/// 卸的时候「一并删除」那一行：写了的设置键、状态目录。
fn deleted(preview: &Value, texts: &PackageTexts) -> Vec<String> {
    let keys: Vec<&str> = list(&preview["settings"])
        .filter_map(Value::as_str)
        .collect();
    let mut items = Vec::new();
    if !keys.is_empty() {
        items.push(
            texts
                .deleted_settings
                .replace("{keys}", &keys.join(&texts.separator)),
        );
    }
    if preview["state"] == true {
        items.push(texts.state_dir.clone());
    }
    items
}

/// 一格数组里的每一项；不是数组的没有。
fn list(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}

/// 一个的、几个的两种写法挑一种，`{n}` 填上。
fn counted(n: u64, forms: &[String; 2]) -> String {
    forms[usize::from(n != 1)].replace("{n}", &n.to_string())
}

/// 路径的最后一截（包的编号照文件夹名）。
fn folder(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

/// 多少字节写成给人看的：不到 1 KiB 的写字节数，再往上一位小数的 KiB、MiB、GiB（照 pacman，同 `miyu pkg`）。
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 3] = ["KiB", "MiB", "GiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests;
