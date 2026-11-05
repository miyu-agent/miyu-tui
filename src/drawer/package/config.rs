//! 卸的时候配置那一项（`package.remove` 预览的 `config`，核心 PK-5、PK-5补）：`{path?, kept, settings?}`，`path` 是 `etc/<编号>/`
//! （目录里有东西才写），`settings` 是系统配置里它的设置项有几项（0 的不写），`kept` 是两样都留下还是都删掉。确认抽屉和软件包页
//! 卸载的弹窗同一个写法：留着的「留在 <路径>」「N 项设置留在配置里」，删的「会删掉 <路径>」「会删掉 N 项设置」，两样都有的接成
//! 一句。

use serde_json::Value;

/// 几种写法：目录留着、删掉（`{path}`），设置项留着、删掉（`{n}`），两样之间接的字。空的那一样不写。
#[derive(Debug, Clone, Copy)]
pub struct ConfigWords<'a> {
    /// 目录留着。
    pub kept: &'a str,
    /// 目录删掉。
    pub purge: &'a str,
    /// 设置项留着。
    pub settings_kept: &'a str,
    /// 设置项删掉。
    pub settings_purge: &'a str,
    /// 两样之间接的字。
    pub join: &'a str,
}

/// 配置那一项；`purge` 是要不要删（`None` 照预览的 `kept`）。目录、设置项都没有的（旧核心、什么都没留）是 `None`。
pub fn config_item(config: &Value, purge: Option<bool>, words: ConfigWords) -> Option<String> {
    let purge = purge.unwrap_or(config["kept"] == false);
    let (dir, settings) = if purge {
        (words.purge, words.settings_purge)
    } else {
        (words.kept, words.settings_kept)
    };
    let path = config["path"]
        .as_str()
        .map(|p| dir.replace("{path}", &crate::local::home_short(p)));
    let count = config["settings"]
        .as_u64()
        .filter(|n| *n > 0)
        .map(|n| settings.replace("{n}", &n.to_string()));
    let parts: Vec<String> = [path, count]
        .into_iter()
        .flatten()
        .filter(|p| !p.is_empty())
        .collect();
    (!parts.is_empty()).then(|| parts.join(words.join))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{ConfigWords, config_item};

    const WORDS: ConfigWords = ConfigWords {
        kept: "留在 {path}",
        purge: "会删掉 {path}",
        settings_kept: "{n} 项设置留在配置里",
        settings_purge: "会删掉 {n} 项设置",
        join: "，",
    };

    #[test]
    fn the_directory_and_the_settings_join_into_one_line() {
        let home = std::env::home_dir().unwrap_or_default();
        let path = home.join(".miyu/etc/im").display().to_string();
        let both = json!({"path": path, "kept": true, "settings": 3});
        assert_eq!(
            config_item(&both, None, WORDS).as_deref(),
            Some("留在 ~/.miyu/etc/im，3 项设置留在配置里")
        );
        assert_eq!(
            config_item(&both, Some(true), WORDS).as_deref(),
            Some("会删掉 ~/.miyu/etc/im，会删掉 3 项设置")
        );
        let only = json!({"kept": true, "settings": 1});
        assert_eq!(
            config_item(&only, None, WORDS).as_deref(),
            Some("1 项设置留在配置里"),
            "只有设置、没有目录"
        );
        let purged = json!({"kept": false, "settings": 2});
        assert_eq!(
            config_item(&purged, None, WORDS).as_deref(),
            Some("会删掉 2 项设置"),
            "照 kept 判"
        );
        assert_eq!(
            config_item(&json!({"kept": true, "settings": 0}), None, WORDS),
            None
        );
        assert_eq!(config_item(&serde_json::Value::Null, None, WORDS), None);
    }
}
