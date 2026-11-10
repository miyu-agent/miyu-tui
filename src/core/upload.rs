//! 附件（蓝图 `tui.md`「输入框」第 12 条，`protocol.md` 的 `blob.put`）：发话以前把每个文件交给核心，交回的回应放进
//! `session.send` 的 `attachments`，从本机文件来的另带原来的路径 `path`（核心 3-9 五补：模型看不了时占位那一句带上它；剪贴板贴的
//! 图存在临时目录里，不带）。传不上的（太大、读不了、在数据根里）整句不发，交回核心的原话。

use std::path::{Path, PathBuf};

use base64::Engine;
use serde_json::{Value, json};

use super::Update;
use super::rpc::{Failure, Rpc};

/// 一句话太长、转成的文字附件（「输入框」第 11 条，施工 YC-11）：名字（`pasted-年月日-时分秒.txt`，同网页）和字。不落盘，
/// 分块交给核心（`blob.open`、`blob.write`、`blob.close`，核心施工 W-5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pasted {
    /// 附件叫什么。
    pub name: String,
    /// 字。
    pub text: String,
}

/// 转成附件的那段字叫什么：`pasted-年月日-时分秒.txt`，照这台机器的时刻（同网页的 `pastedName`）。
#[must_use]
pub fn pasted_name(at: &jiff::Zoned) -> String {
    at.strftime("pasted-%Y%m%d-%H%M%S.txt").to_string()
}

/// 一块传多少字节：核心收的上限（同网页附件包的 `chunk_bytes`），写成 base64 加信封还在协议的一行以内。
const CHUNK: usize = 512 * 1024;

/// 照先后传每一个文件，再传转成附件的那段字，交回它们的回应；有一个传不上就停，交回为什么。
pub(super) async fn attach(
    rpc: &mut Rpc,
    files: &[PathBuf],
    pasted: Option<&Pasted>,
) -> Result<Vec<Value>, Update> {
    let mut out = Vec::with_capacity(files.len() + 1);
    for file in files {
        let params = json!({"path": file.display().to_string()});
        let got = rpc.call("blob.put", params).await.map_err(refused)?;
        out.push(with_path(got, file));
    }
    if let Some(pasted) = pasted {
        out.push(text(rpc, pasted).await?);
    }
    Ok(out)
}

/// 这个命令要先传哪些：说话带的文件、转成附件的那段字；重做换了附件的照换的（换成没有的是空的）。不用传的是 `None`。
pub(super) fn wanted(command: &super::Command) -> Option<(Vec<PathBuf>, Option<Pasted>)> {
    match command {
        super::Command::Send { files, pasted, .. } if !files.is_empty() || pasted.is_some() => {
            Some((files.clone(), pasted.clone()))
        }
        super::Command::Redo { files, .. } => files.clone().map(|files| (files, None)),
        _ => None,
    }
}

/// 核心拒了、断了。
fn refused(failure: Failure) -> Update {
    match failure {
        Failure::Refused(message) => Update::Refused {
            reason: None,
            message,
        },
        Failure::Io(_) | Failure::Disconnected => Update::Disconnected,
    }
}

/// 一段字分块传成 `text/plain` 的附件，交回 `blob.close` 的回应（同 `blob.put`）。
async fn text(rpc: &mut Rpc, pasted: &Pasted) -> Result<Value, Update> {
    let bytes = pasted.text.as_bytes();
    let open = json!({"name": pasted.name, "size": bytes.len(), "media_type": "text/plain"});
    let opened = rpc.call("blob.open", open).await.map_err(refused)?;
    let upload = opened["upload"].clone();
    for (i, chunk) in bytes.chunks(CHUNK).enumerate() {
        let data = base64::engine::general_purpose::STANDARD.encode(chunk);
        let write = json!({"upload": upload, "offset": i * CHUNK, "data": data});
        rpc.call("blob.write", write).await.map_err(refused)?;
    }
    rpc.call("blob.close", json!({"upload": upload}))
        .await
        .map_err(refused)
}

/// `blob.put` 的回应接上原来的路径（核心 3-9 五补）。
/// 剪贴板贴的图（暂存在临时目录）、写法核心不收的（不是绝对路径、有控制字符、超过 4096 字节）不带。
fn with_path(mut got: Value, file: &Path) -> Value {
    let text = file.display().to_string();
    let fits = file.is_absolute() && text.len() <= 4096 && !text.chars().any(char::is_control);
    if fits && !crate::clipboard::is_staged(file) {
        got["path"] = json!(text);
    }
    got
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;

    use super::with_path;

    #[test]
    fn a_file_from_disk_carries_its_path_and_a_pasted_one_does_not() {
        let got = json!({"blob": "sha256:aa", "name": "晚霞.png"});
        // 照这台机器的写法拼一个绝对路径：`/home/...` 在 Windows 上不算绝对路径。
        let file = std::env::temp_dir().join("图").join("晚霞.png");
        let from_disk = with_path(got.clone(), &file);
        assert_eq!(from_disk["path"], file.display().to_string());
        assert_eq!(from_disk["blob"], "sha256:aa", "回应原样留着");
        let pasted = Path::new("/home/me/.cache/miyu/tui/pasted/4242/0a1b.png");
        assert!(
            with_path(got.clone(), pasted).get("path").is_none(),
            "剪贴板贴的不带"
        );
        let odd = Path::new("/tmp/a\nb.png");
        assert!(
            with_path(got, odd).get("path").is_none(),
            "有控制字符的不带，核心会拒"
        );
    }
}
