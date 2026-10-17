//! 她给人看的文件（核心 D-5 `show`，蓝图 `tui.md`「展示」第 1 条）：`tool.result` 的 `effects` 里一条 `file.shown`。

use serde_json::Value;

use super::Push;

/// 展示的一个文件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    /// 真实位置的绝对路径：同一个路径再展示一次，前面那张缩成一行（「展示」第 5 条）。
    pub path: String,
    /// 内容的哈希：照 `blob.get` 读回来。
    pub blob: String,
    /// 文件名。
    pub name: String,
    /// 核心认的种类：`image/png`、`text/html`、`application/pdf` 这些。
    pub media_type: String,
    /// 字节数。
    pub size: u64,
    /// 她写的标题；没写的是 `None`。
    pub title: Option<String>,
    /// 代码、文本认出的语言（Markdown 代码块的写法）；认不出的是 `None`。
    pub language: Option<String>,
}

impl Shown {
    /// 是图：直接画出来（第 3 条）。
    pub fn image(&self) -> bool {
        self.media_type.starts_with("image/")
    }

    /// 收到新的就当场开一次的：HTML、PDF（第 8 条）。
    pub fn opens_at_once(&self) -> bool {
        matches!(self.media_type.as_str(), "text/html" | "application/pdf")
    }
}

/// 读结果的 `effects` 里的 `file.shown`，照先后交 [`Push::Shown`]；缺 `path`、`blob` 的不要。
pub(super) fn read(body: &Value, out: &mut Vec<Push>) {
    let text = |v: &Value| {
        v.as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    for effect in body["effects"].as_array().into_iter().flatten() {
        if effect["kind"] != "file.shown" {
            continue;
        }
        let (Some(path), Some(blob)) = (text(&effect["path"]), text(&effect["blob"])) else {
            continue;
        };
        let name = text(&effect["name"])
            .unwrap_or_else(|| path.rsplit(['/', '\\']).next().unwrap_or(&path).to_string());
        out.push(Push::Shown(Shown {
            path,
            blob,
            name,
            media_type: text(&effect["media_type"])
                .unwrap_or_else(|| "application/octet-stream".into()),
            size: effect["size"].as_u64().unwrap_or(0),
            title: text(&effect["title"]),
            language: text(&effect["language"]),
        }));
    }
}
