//! 「软件包」三页的字（蓝图「配置页」第 43c 条），在 `text/*.json` 的 `settings.more.store`。

use std::collections::HashMap;

use serde::Deserialize;

/// 一个按键提示：键、说什么。
pub type Hint = [String; 2];

/// 「软件包」三页的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoreTexts {
    /// 主菜单那一项和它底下那一行。
    pub entry: [String; 2],
    /// 三页的名字：下载软件、已安装、软件后台。
    pub tabs: [String; 3],
    /// 分页条右边「排序」，后面接排序的名字。
    pub sort: String,
    /// 三种排序：最近、下载量、趋势。
    pub sorts: [String; 3],
    /// 搜索那一行打头的字。
    pub search: String,
    /// 列表右边的状态：`installed`、`upgradable`、`blocked`、`off`、`upgrade_to`（`{to}`）。
    pub states: HashMap<String, String>,
    /// 在读软件源。
    pub loading_source: String,
    /// 在读装了的。
    pub loading: String,
    /// 没有对得上的、没有带后台页的。
    pub empty: [String; 2],
    /// 预览块的键：`name`、`version`、`summary` 这类。
    pub keys: HashMap<String, String>,
    /// 预览块里的几种写法：`installed_as`（`{v}`）、`downloads`（`{n}`、`{w}`）、`settings_yes`、`settings_no`、`none`、
    /// `required_by` 这类。
    pub values: HashMap<String, String>,
    /// 「包含」照 `provides` 的种类写：`program`、`skills` 这类。
    pub provides: HashMap<String, String>,
    /// 带的程序照种类写（`ui`、`process`、`worker`、`builtin`，同确认抽屉），不露核心的种类词。
    pub programs: HashMap<String, String>,
    /// 问之前、做的时候写在预览块里的：`install`（`{n}`）、`with`、`recommends`、`remove`（`{name}`，带版本）、`drop`、
    /// `upgrade`（`{n}`）、`approve`（`{name}`）、`go`、`downloading`、`verifying`、`installing`。
    pub ask: HashMap<String, String>,
    /// 状态行：`installed`（`{names}`）、`removed`、`upgraded`、`on`、`off`、`latest`、`all_latest`、`cancelled`、
    /// `no_settings`、`blocked`（`{why}`）、`opened`、`run_web`（`{id}`）、`no_browser`（`{url}`）。
    pub status: HashMap<String, String>,
    /// 按键提示：`source`、`typing`、`installed`、`backstage`、`confirm`、`running`。
    pub hints: HashMap<String, Vec<Hint>>,
}

impl StoreTexts {
    /// 一句字：没有的是空的（字表少写了不至于崩）。
    pub fn say(table: &HashMap<String, String>, key: &str) -> String {
        table.get(key).cloned().unwrap_or_default()
    }

    /// 按键提示：没有的是空的。
    pub fn hint(&self, key: &str) -> &[Hint] {
        self.hints.get(key).map_or(&[], Vec::as_slice)
    }
}
