//! 挂载位的配置（蓝图 `tui.md`「挂载位」第 2 条，反馈 1102 第 13 条）：终端软件包清单里每个多件挂载位一项记录列表
//! （`tui.side` 这种，配置页不画），一条一个部件：名字、藏不藏、最多几行。这里只管读和排先后，不知道有哪些部件：出厂的
//! 先后由画那一处交进来。

use serde::Deserialize;
use serde_json::Value;

/// 配置里写的一条。
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Slot {
    /// 部件名（`recent`、`pkg:<包>/<块>` 这种）。
    pub part: String,
    /// 藏起来。
    #[serde(default)]
    pub hidden: bool,
    /// 最多显示几行；0 是不限。
    #[serde(default)]
    pub rows: u16,
}

/// 侧边栏出厂的部件名，照出厂的先后（「挂载位」第 4 条，画在 `ui/sidebar/parts.rs`）：配置里没写到的照它接在后面。
pub const SIDE_PARTS: [&str; 7] = [
    "avatar", "session", "cwd", "context", "usage", "loops", "recent",
];

/// 侧边栏那一项在终端软件包里的名字（`[settings.side]`）。
pub const SIDE_KEY: &str = "tui.side";

/// 底栏左段出厂的部件名（「挂载位」第 4 条，SL-2，画在 `ui/footer.rs`）：权限级别、模型。
pub const FOOTER_LEFT: [&str; 2] = ["level", "model"];

/// 底栏右段出厂的部件名：上下文、用量（累计连花费）。
pub const FOOTER_RIGHT: [&str; 2] = ["context", "usage"];

/// 底栏那一项在终端软件包里的名字（`[settings.footer]`）。
pub const FOOTER_KEY: &str = "tui.footer";

/// 输入框上面那一块出厂的部件名（「挂载位」第 4 条 SL-2）：待办。
pub const ABOVE_PARTS: [&str; 1] = ["todo"];

/// 输入框上面那一项在终端软件包里的名字（`[settings.above]`）。
pub const ABOVE_KEY: &str = "tui.above";

/// 读 `config.get` 交回的 `key` 那一项：认不得的条目跳过，没写、读不成的是空的（照出厂的画）。
pub fn read(got: &Value, key: &str) -> Vec<Slot> {
    got["items"][key]["value"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| serde_json::from_value::<Slot>(item.clone()).ok())
        .filter(|slot| !slot.part.is_empty())
        .collect()
}

/// 照配置排出这一处要画的部件：写了的照写的先后（藏起来的去掉），没写到的出厂部件照出厂的先后接在后面；交回
/// （部件名，最多几行），连同配置里写了、`known` 里没有的名字（不画，提示一次）。同一个名字写了两遍的照第一条。
pub fn arrange<'a>(known: &[&'a str], config: &[Slot]) -> (Vec<(&'a str, u16)>, Vec<String>) {
    let mut out = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    let mut unknown = Vec::new();
    for slot in config {
        if seen.contains(&slot.part.as_str()) {
            continue;
        }
        match known.iter().find(|name| **name == slot.part) {
            Some(name) => {
                seen.push(name);
                if !slot.hidden {
                    out.push((*name, slot.rows));
                }
            }
            None => unknown.push(slot.part.clone()),
        }
    }
    out.extend(
        known
            .iter()
            .filter(|name| !seen.contains(name))
            .map(|name| (*name, 0)),
    );
    (out, unknown)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{SIDE_KEY, Slot, arrange, read};

    fn slot(part: &str, hidden: bool, rows: u16) -> Slot {
        Slot {
            part: part.into(),
            hidden,
            rows,
        }
    }

    #[test]
    fn written_ones_go_first_hidden_ones_drop_and_the_rest_follow_in_factory_order() {
        let known = ["avatar", "session", "cwd", "recent"];
        let config = [
            slot("recent", false, 8),
            slot("avatar", true, 0),
            slot("nope", false, 0),
            slot("recent", false, 3),
        ];
        let (parts, unknown) = arrange(&known, &config);
        assert_eq!(parts, [("recent", 8), ("session", 0), ("cwd", 0)]);
        assert_eq!(unknown, ["nope"], "不认识的不画、交回去提示");
        let (parts, unknown) = arrange(&known, &[]);
        assert_eq!(parts.len(), 4, "没配的照出厂");
        assert!(unknown.is_empty());
    }

    #[test]
    fn the_record_list_is_read_from_config_get_and_bad_items_are_skipped() {
        let got = json!({"items": {SIDE_KEY: {"value": [
            {"part": "recent", "rows": 8},
            {"part": "avatar", "hidden": true},
            {"hidden": true},
            "recent"
        ]}}});
        assert_eq!(
            read(&got, SIDE_KEY),
            [slot("recent", false, 8), slot("avatar", true, 0)]
        );
        assert!(read(&json!({}), SIDE_KEY).is_empty(), "旧核心没这一项");
    }
}
