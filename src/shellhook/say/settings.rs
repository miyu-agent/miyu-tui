//! 说话时照核心的配置项换的几样：界面语言（`ui.language`），时间线和按键模式要读的几项。

use crate::config::Config;
use crate::core::{Command, Core};

/// 界面语言照 `ui.language`（同全屏 `app/language.rs`）：写了一种语言的换成它，`auto` 照系统的；界面上的字、运行状态行的
/// 词、工具的显示名都换。
pub(super) fn language(code: &str, config: &mut Config, core: &Core) {
    let next = match config.language_table.find(code) {
        Some(language) if code != "auto" => language,
        _ => {
            let table = &config.language_table;
            table.detect(|name| std::env::var(name).ok())
        }
    };
    if next == config.language {
        return;
    }
    if let Ok(fresh) = Config::load(&next, code == "auto") {
        config.text = fresh.text;
        config.pulse = fresh.pulse;
        config.language = next.clone();
        core.send(Command::FetchHuman(next.code().to_string()));
    }
}

/// 盖在 `layout`、`timeline` 上要读的几项：时间线的、按键模式（同全屏 `apply_layout_settings`；吉祥物 shell 里不画）。
pub(super) fn layout_keys() -> Vec<&'static str> {
    let mut keys = crate::config::Timeline::KEYS.to_vec();
    keys.push(crate::config::Layout::KEYS_KEY);
    keys
}

/// 读几项配置（`config.get`），回应照 `tag` 认。
pub fn get<K: serde::Serialize>(core: &Core, tag: u64, keys: K) {
    core.send(Command::Ask {
        tag,
        method: "config.get",
        params: serde_json::json!({ "keys": keys }),
        system: false,
    });
}
