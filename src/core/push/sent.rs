//! 她发出去的图（`tool.result` 的 `sent`，核心 X-19）：每项同图片块（`blob`、`media_type`、`width`、`height`、`name`）
//! 加 `alt`（看不了图时的一句）、`sticker`（表情包）。不发给模型，界面画在正文里，和回答的字一样（蓝图「她发的图」）。
//! 宽高不要：是扩展报的，大小照读回来的图本身（实测测试用的扩展报 2×2，照它画只有一格）。

use serde_json::Value;

/// 发出去的一张图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    /// 图的内容哈希：照 `blob.get` 读回来。
    pub blob: String,
    /// 文件名；可以没有。
    pub name: Option<String>,
    /// 看不了图时写的一句；可以没有。
    pub alt: Option<String>,
    /// 是表情包。
    pub sticker: bool,
}

/// 读 `sent` 那一格：没有 `blob` 的、不是图的不要；空的、没有这一格的（老日志）是空的。
pub(super) fn read(sent: &Value) -> Vec<Sent> {
    let text = |v: &Value| {
        v.as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    sent.as_array()
        .into_iter()
        .flatten()
        .filter(|item| {
            item["media_type"]
                .as_str()
                .is_none_or(|m| m.starts_with("image/"))
        })
        .filter_map(|item| {
            Some(Sent {
                blob: text(&item["blob"])?,
                name: text(&item["name"]),
                alt: text(&item["alt"]),
                sticker: item["sticker"].as_bool().unwrap_or(false),
            })
        })
        .collect()
}
