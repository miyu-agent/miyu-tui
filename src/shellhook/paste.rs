//! `shellhook paste`：shell 里按 `Ctrl+V`（图纸 `shellhook.md`「粘贴」，同终端界面「输入框」第 11、12 条）：程序自己读
//! 剪贴板，复制的文件、图收成块（图是附件），大段的字收成 `[已粘贴 14 行]`，别的照字。块上的字印到标准输出，脚本插进
//! 命令行；块的原文、附件记在这个 shell 的一份里（`<缓存目录>/shellhook/<shell>-pasted.json`），说这句话时（`say`）照
//! 命令行里还在的块换回原文、带上附件。那一份留到 shell 退出：从历史里调出来再发照样认得出块，附件的编号在这个 shell 里
//! 一直往下排、不从 1 重来（同终端界面整个会话往下排）。收块的规矩、写法和全屏同一份（`input::InputBox`）。

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::input::{Attachment, Block, Draft, InputBox, PasteRule};

/// 剪贴板里的截图算哪一种附件（`attachments.json` 里的名字，同全屏）。
const IMAGE: &str = "image";

/// 记着的一块。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Stored {
    /// 块上的字。
    label: String,
    /// 原文（附件的就是块上的字）。
    text: String,
    /// 附件：文件、哪一种。
    file: Option<PathBuf>,
    kind: Option<String>,
    /// 文件块：本机的路径。
    path: Option<PathBuf>,
}

/// 这个 shell 记着的那一份。
fn store(shell: u32) -> Option<PathBuf> {
    Some(super::say::spool::dir()?.join(format!("{shell}-pasted.json")))
}

fn load(shell: u32) -> Vec<Stored> {
    store(shell)
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// 读剪贴板、收成块，块上的字印到标准输出；读不到、是空的，提示一句到标准错误，退出码 1（脚本照 shell 自己的粘贴办）。
///
/// # Errors
///
/// 读配置、写不进记着的那一份、写标准输出失败。
pub fn run(shell: u32) -> io::Result<ExitCode> {
    sweep();
    let config = super::say::config()?;
    let mut stored = load(shell);
    let mut input = InputBox::new(1, std::time::Duration::ZERO);
    input.set_paste_rule(PasteRule {
        lines: config.layout.paste_fold_lines,
        chars: config.layout.paste_fold_chars,
        label: config.text.paste_label.clone(),
    });
    input.set_attach_rule(crate::app::attach_rule(&config));
    // 这一句里已经贴过的附件：接着编号（`[图片 2]`）。
    let mut base = std::collections::HashMap::new();
    for kind in stored.iter().filter_map(|s| s.kind.clone()) {
        *base.entry(kind).or_insert(0) += 1;
    }
    input.renumber(base);
    if !grab(&mut input, shell, &config) {
        return Ok(ExitCode::FAILURE);
    }
    let draft = input.draft();
    stored.extend(draft.blocks.iter().map(|b| Stored {
        label: draft.text[b.start..b.end].to_string(),
        text: b.text.clone(),
        file: b.attachment.as_ref().map(|a| a.file.clone()),
        kind: b.attachment.as_ref().map(|a| a.kind.clone()),
        path: b.path.clone(),
    }));
    if !draft.blocks.is_empty() {
        let path = store(shell).ok_or_else(|| io::Error::other("no cache directory"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_vec(&stored).map_err(io::Error::other)?)?;
    }
    let mut out = io::stdout();
    out.write_all(draft.text.as_bytes())?;
    out.flush()?;
    Ok(ExitCode::SUCCESS)
}

/// 照全屏的先后读：复制的文件，图，字。读到了交回 `true`；读不到、是空的，提示一句交回 `false`。
fn grab(input: &mut InputBox, shell: u32, config: &Config) -> bool {
    if crate::clipboard::read_files().is_some_and(|files| input.paste_files(&files)) {
        return true;
    }
    // 截图暂存在全屏同一处（`tui/pasted/<shell 的编号>/`）：shell 不在了，下次起全屏时清掉。
    let dir = miyu_client::places::cache_root(&miyu_client::places::Env::current())
        .ok()
        .map(|root| crate::clipboard::staging_root(&root).join(shell.to_string()));
    if let Some(file) = dir.as_deref().and_then(crate::clipboard::read_image) {
        input.attach(file, IMAGE);
        return true;
    }
    let hint = match crate::clipboard::read() {
        // 读来的字全是本机现成的文件、目录（复制的路径）的也收成块（同全屏，2026-10-20 项目主人验收：原来只认复制的文件）。
        Ok(text) if !text.is_empty() => {
            if !input.paste_files(&text) {
                input.paste(&text);
            }
            return true;
        }
        Ok(_) => &config.text.clipboard_empty,
        Err(_) => &config.text.clipboard_unreadable,
    };
    drop(writeln!(io::stderr(), "{hint}"));
    false
}

/// 说这一句（`say`）：照字里还在的块换回原文、带上附件。记着的那一份留着（发出去的这一句进了 shell 的历史，从历史里
/// 调出来再发照样认得出块，2026-10-21 项目主人：「占位块历史应当从历史调出来时还是有正常功能的占位块」），shell
/// 退出以后清（[`sweep`]）。没贴过的照原样。
pub fn take(shell: u32, text: &str) -> (String, Vec<PathBuf>) {
    let stored = load(shell);
    if stored.is_empty() {
        return (text.to_string(), Vec::new());
    }
    let draft = found(text, &stored);
    (draft.expand(), draft.attachments())
}

/// 字里认出记着的块：从前往后扫，哪个块上的字在这里起头就是它（同一个样子贴过几次的认最近的那次）。块的先后不论：从历史
/// 调出来的那句，块不一定照贴的先后。
fn found(text: &str, stored: &[Stored]) -> Draft {
    let mut blocks = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let here = &text[at..];
        let hit = stored
            .iter()
            .rev()
            .find(|s| !s.label.is_empty() && here.starts_with(s.label.as_str()));
        let Some(s) = hit else {
            at += here.chars().next().map_or(1, char::len_utf8);
            continue;
        };
        let end = at + s.label.len();
        blocks.push(Block {
            start: at,
            end,
            text: s.text.clone(),
            attachment: s
                .file
                .clone()
                .zip(s.kind.clone())
                .map(|(file, kind)| Attachment { file, kind }),
            path: s.path.clone(),
        });
        at = end;
    }
    Draft {
        text: text.to_string(),
        blocks,
    }
}

/// 清掉已经退出了的 shell 记着的那几份（`<shell>-pasted.json`）。看不了进程在不在的平台不清。
fn sweep() {
    let Some(dir) = super::say::spool::dir() else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(pid) = name.strip_suffix("-pasted.json") else {
            continue;
        };
        if pid.parse::<u32>().is_ok()
            && cfg!(target_os = "linux")
            && !std::path::Path::new(&format!("/proc/{pid}")).exists()
        {
            drop(std::fs::remove_file(entry.path()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Stored, found};

    #[test]
    fn blocks_still_on_the_command_line_come_back_as_their_text_and_files() {
        let stored = vec![
            Stored {
                label: "[已粘贴 14 行]".into(),
                text: "很长的日志".into(),
                file: None,
                kind: None,
                path: None,
            },
            Stored {
                label: "[图片 1]".into(),
                text: "[图片 1]".into(),
                file: Some("/tmp/a.png".into()),
                kind: Some("image".into()),
                path: None,
            },
        ];
        let draft = found("看看 [已粘贴 14 行] 和 [图片 1]", &stored);
        assert_eq!(draft.expand(), "看看 很长的日志 和 [图片 1]");
        assert_eq!(
            draft.attachments(),
            vec![std::path::PathBuf::from("/tmp/a.png")]
        );
        // 命令行里删掉了的块不要。
        let draft = found("只说这个", &stored);
        assert!(draft.attachments().is_empty());
        // 从历史里调出来的：块的先后和贴的不一样也认得出。
        let draft = found("[图片 1] 先说图，再看 [已粘贴 14 行]", &stored);
        assert_eq!(draft.expand(), "[图片 1] 先说图，再看 很长的日志");
        assert_eq!(draft.attachments().len(), 1);
    }
}
