//! 一句话里附的图片、文件画成块（蓝图「输入框」第 12 条、「别处来的话」，核心 C-9）：补发来的你说的话、别的会话发来的话
//! 都照人发附件那样写 `[图片 1]` 这类块，点块用系统的程序打开原来那个文件。

use std::collections::HashMap;
use std::path::PathBuf;

use super::{Chip, Kind, Transcript};
use crate::config::AttachTexts;
use crate::core::Attached;

impl Transcript {
    /// 序号是 `seq` 的那一句附了 `attached`：画成块。你在这个界面发的那句字里已经写着块上的字（`[图片 1]`），照先后对上；
    /// 没写的（别的会话发来的）接在字后面，编号照正文里这一种已有的接着数。别的文件写 `[文件名]`。已经画过块的不再画。
    pub fn attach_to(&mut self, seq: u64, attached: &[Attached], words: &AttachTexts) {
        let at = self
            .entries
            .iter()
            .rposition(|e| e.kind == Kind::User && e.seq == Some(seq));
        self.attach_at(at, attached, words);
    }

    /// 子会话开头先画上的那句交代（还没有序号）附了 `attached`：派它那一步的参数里写的（子会话是派出去以后才订阅的，
    /// 核心推来的那一句看不到，「切进子会话」第 1 条）。
    pub fn attach_handed(&mut self, attached: &[Attached], words: &AttachTexts) {
        let at = self
            .entries
            .iter()
            .rposition(|e| e.kind == Kind::User && e.seq.is_none() && e.from.is_some());
        self.attach_at(at, attached, words);
    }

    fn attach_at(&mut self, at: Option<usize>, attached: &[Attached], words: &AttachTexts) {
        let Some(at) = at.filter(|_| !attached.is_empty()) else {
            return;
        };
        if self.entries[at].pasted.iter().any(Chip::attachment) {
            return;
        }
        let mut counts = self.attachment_counts();
        let entry = &mut self.entries[at];
        let mut written: HashMap<&str, Vec<String>> = HashMap::new();
        for one in attached {
            let kind = one.kind();
            let label = match words.labels.get(kind) {
                Some(template) => {
                    let found = written
                        .entry(kind)
                        .or_insert_with(|| labels_in(&entry.text, template));
                    let n = counts.entry(kind.to_string()).or_default();
                    *n += 1;
                    let mine = (!found.is_empty()).then(|| found.remove(0));
                    mine.unwrap_or_else(|| template.replace("{n}", &n.to_string()))
                }
                None => words
                    .file
                    .replace("{name}", one.name.as_deref().unwrap_or(&one.media_type)),
            };
            if !entry.text.contains(&label) {
                if !entry.text.is_empty() && !entry.text.ends_with(char::is_whitespace) {
                    entry.text.push(' ');
                }
                entry.text.push_str(&label);
            }
            entry.pasted.push(Chip {
                full: one.path.clone().unwrap_or_else(|| label.clone()),
                label,
                kind: Some(kind.to_string()),
                file: one.path.as_ref().map(PathBuf::from),
            });
        }
    }
}

/// 字里照 `template`（`[图片 {n}]`）写的块，照先后。
fn labels_in(text: &str, template: &str) -> Vec<String> {
    let Some((head, tail)) = template.split_once("{n}") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(head) {
        let after = &rest[at + head.len()..];
        let digits = after.chars().take_while(char::is_ascii_digit).count();
        if digits > 0 && after[digits..].starts_with(tail) {
            out.push(format!("{head}{}{tail}", &after[..digits]));
            rest = &after[digits + tail.len()..];
        } else {
            rest = after;
        }
    }
    out
}

#[cfg(test)]
mod tests;
