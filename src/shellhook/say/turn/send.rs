//! 那句话发出去（`turn.rs` 的一轮）：说话、`/edit` 重来、`/redo`、`/compact`、交给核心的斜杠命令；太长的转成文字附件
//! （蓝图 `tui.md`「输入框」第 11 条，施工 YC-11）。

use super::Turn;
use crate::core::{Command, Core, Pasted};

impl Turn {
    /// 这句话放不进协议的一行：整段转成叫 `name` 的文字附件，话里不带字；认这一句照附件的名字（核心推回来的那句是空的，
    /// 这条连接自己发的照序号认）。
    pub fn as_attachment(&mut self, name: String) {
        if let Some(text) = self.pending.take() {
            self.pasted = Some(Pasted {
                name: name.clone(),
                text,
            });
            self.pending = Some(String::new());
            self.said = name;
        }
    }

    /// 那句话发出去：接上了终端会话的说进它，没问到的（核心旧）照新会话开。
    pub(super) fn send(&mut self, core: &Core) {
        if let Some(text) = self.pending.take() {
            self.sent = true;
            if self.bare_redo {
                core.send(Command::Redo {
                    text: None,
                    files: None,
                });
                return;
            }
            if let Some(words) = self.compact.take() {
                // 压缩那几行不带回合号：读回来的那一页清掉，之后来的都是这一轮的，照全部印（`scope`）。
                self.transcript.entries.clear();
                self.transcript.manual_compaction = true;
                self.manual = true;
                core.send(Command::Compact(words));
                return;
            }
            if self.slash {
                super::super::dreaming::start(self, &text, core);
                core.send(Command::Slash(text));
                return;
            }
            // `/edit` 改过的：替换上一轮重来（附件没贴新的照原来的）。
            if self.redo {
                core.send(Command::Redo {
                    text: Some(text),
                    files: (!self.files.is_empty()).then(|| self.files.clone()),
                });
                return;
            }
            core.send(Command::Send {
                text,
                files: self.files.clone(),
                cwd: self.cwd.clone(),
                shell: false,
                skills: Vec::new(),
                pasted: self.pasted.take(),
            });
        }
    }
}
