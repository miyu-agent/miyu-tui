//! 一句话里附的图片、文件（`message.user` 的 `image`、`file` 块，核心 `kernel/blocks.md`）：人发的、别的会话发的（核心
//! C-9：`send_message`、`subagent` 带的附件）一个样子，界面照人发附件那样画成块（蓝图「输入框」第 12 条、「别处来的话」）。

use serde_json::Value;

/// 附的一个。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attached {
    /// 媒体类型：照它认是图片、PDF、音频、视频还是别的文件。
    pub media_type: String,
    /// 文件名，只是名字；可以没有。
    pub name: Option<String>,
    /// 原来在本机的哪儿：点块照它打开；可以没有（剪贴板贴的）。
    pub path: Option<String>,
}

impl Attached {
    /// 种类：图片、PDF、音频、视频照 `attach.labels` 的名字（`image`、`pdf`、`audio`、`video`），别的是 `file`。
    pub fn kind(&self) -> &'static str {
        let media = self.media_type.as_str();
        match media.split('/').next() {
            Some("image") => "image",
            Some("audio") => "audio",
            Some("video") => "video",
            _ if media == "application/pdf" => "pdf",
            _ => "file",
        }
    }
}

impl Attached {
    /// 照本机的一个文件：`kind` 是照扩展名认出的种类（`image`、`pdf`、`audio`、`video`，认不出的是 `None`）。派子代理那一步的
    /// 参数里只写路径（核心 C-9），照它画块。
    pub fn from_path(path: &str, kind: Option<&str>) -> Self {
        let media_type = match kind {
            Some("pdf") => "application/pdf".to_string(),
            Some(kind) => format!("{kind}/*"),
            None => "application/octet-stream".to_string(),
        };
        let name = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());
        Self {
            media_type,
            name,
            path: Some(path.to_string()),
        }
    }
}

/// 一句话正文里附的，照先后；没有的是空的。
pub fn read(body: &Value) -> Vec<Attached> {
    let text = |v: &Value| v.as_str().filter(|s| !s.is_empty()).map(str::to_string);
    super::blocks(body)
        .filter(|b| matches!(b["type"].as_str(), Some("image" | "file")))
        .map(|b| Attached {
            media_type: b["media_type"].as_str().unwrap_or_default().to_string(),
            name: text(&b["name"]),
            path: text(&b["path"]),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::read;

    #[test]
    fn image_and_file_blocks_are_read_in_order_and_text_is_skipped() {
        // 核心 C-9：别的会话发来的话是「字 + 图片块、文件块」，和人发的附件一个样子。
        let body = json!({"blocks": [
            {"type": "text", "text": "看看这两个"},
            {"type": "image", "blob": "sha256:aa", "name": "晚霞.png", "media_type": "image/png",
             "width": 8, "height": 6, "path": "/home/me/晚霞.png"},
            {"type": "file", "blob": "sha256:bb", "name": "报告.pdf", "media_type": "application/pdf"},
            {"type": "file", "blob": "sha256:cc", "name": "data.zip", "media_type": "application/zip",
             "path": "/tmp/data.zip"}]});
        let got = read(&body);
        let kinds: Vec<&str> = got.iter().map(super::Attached::kind).collect();
        assert_eq!(kinds, ["image", "pdf", "file"]);
        assert_eq!(got[0].path.as_deref(), Some("/home/me/晚霞.png"));
        assert_eq!(got[1].path, None, "剪贴板贴的、没给位置的");
        assert_eq!(got[2].name.as_deref(), Some("data.zip"));
        assert!(read(&json!({"blocks": [{"type": "text", "text": "只有字"}]})).is_empty());
    }
}
