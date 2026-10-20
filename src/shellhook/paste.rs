//! `shellhook paste`：shell 里按 `Ctrl+V`（图纸 `shellhook.md`「粘贴」，同终端界面「输入框」第 11、12 条）：程序自己读
//! 剪贴板，复制的文件、图收成块（图是附件），大段的字收成 `[已粘贴 14 行]`，别的照字。块上的字印到标准输出，脚本插进
//! 命令行；块的原文、附件记在这个 shell 的一份里（`<缓存目录>/shellhook/<shell>-pasted.json`），说这句话时（`say`）照
//! 命令行里还在的块换回原文、带上附件，说完清掉。收块的规矩、写法和全屏同一份（`input::InputBox`）。

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

/// 说这一句（`say`）：照命令行里还在的块换回原文，带上附件；记着的那一份清掉。没贴过的照原样。
pub fn take(shell: u32, text: &str) -> (String, Vec<PathBuf>) {
    let stored = load(shell);
    if let Some(path) = store(shell) {
        drop(std::fs::remove_file(path));
    }
    if stored.is_empty() {
        return (text.to_string(), Vec::new());
    }
    let draft = Draft::rebind(text, &before(&stored));
    (draft.expand(), draft.attachments())
}

/// 记着的几块拼成一份（块上的字一个接一个），好照它在命令行的字里认回来（`Draft::rebind`）。
fn before(stored: &[Stored]) -> Draft {
    let mut text = String::new();
    let mut blocks = Vec::new();
    for s in stored {
        let start = text.len();
        text.push_str(&s.label);
        blocks.push(Block {
            start,
            end: text.len(),
            text: s.text.clone(),
            attachment: s
                .file
                .clone()
                .zip(s.kind.clone())
                .map(|(file, kind)| Attachment { file, kind }),
            path: s.path.clone(),
        });
        text.push('\n');
    }
    Draft { text, blocks }
}

#[cfg(test)]
mod tests {
    use super::{Stored, before};
    use crate::input::Draft;

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
        let draft = Draft::rebind("看看 [已粘贴 14 行] 和 [图片 1]", &before(&stored));
        assert_eq!(draft.expand(), "看看 很长的日志 和 [图片 1]");
        assert_eq!(
            draft.attachments(),
            vec![std::path::PathBuf::from("/tmp/a.png")]
        );
        // 命令行里删掉了的块不要。
        let draft = Draft::rebind("只说这个", &before(&stored));
        assert!(draft.attachments().is_empty());
    }
}
