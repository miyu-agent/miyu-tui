//! 资源文件盖上主题（蓝图 `tui.md`「主题」第 2 条）：`layout.json`、`timeline.json` 里的记号照主题那一节盖；换了主题照新的
//! 重读这两份。

use serde::Deserialize;

use super::Config;

impl Config {
    /// 换了主题（`theme` 是整份）：`layout`、`timeline` 照它重读。终端自己的配置项盖过的那几格（吉祥物、时间线的开关）
    /// 由调用的再盖一遍。
    ///
    /// # Errors
    ///
    /// 盖上以后读不懂时什么都不换，交回那一句。
    pub fn retheme(&mut self, theme: &serde_json::Value) -> Result<(), String> {
        let layout = themed("layout", include_str!("../../resources/layout.json"), theme)?;
        let timeline = themed(
            "timeline",
            include_str!("../../resources/timeline.json"),
            theme,
        )?;
        self.layout = layout;
        self.timeline = timeline;
        Ok(())
    }
}

/// `resources/<name>.json` 盖上主题里同名的那一节再读。
pub(super) fn themed<T: for<'de> Deserialize<'de>>(
    name: &str,
    json: &str,
    theme: &serde_json::Value,
) -> Result<T, String> {
    let value = crate::sheet::overlay(json, name, theme)
        .map_err(|e| format!("resources/{name}.json 读不懂：{e}"))?;
    serde_json::from_value(value).map_err(|e| format!("resources/{name}.json 盖上主题读不懂：{e}"))
}
