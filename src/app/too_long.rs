//! 太长的话（蓝图 `tui.md`「输入框」第 11 条，施工 YC-11，压测报告 F5）：协议一行最多 1 MiB，放不下的核心整行不认，原来输入框
//! 清空了、话悄悄丢了。回车时先量：放不下的转成一个文字附件发出去（同网页，名字 `pasted-年月日-时分秒.txt`，分块交给核心、
//! 不落盘）；转不了的（编辑上一句、`!` 命令、超过附件上限）字留在框里、弹一句。

use super::App;
use crate::core::{Pasted, line_limit, pasted_name};
use crate::input::Draft;
use crate::transcript::Chip;

/// 转成的文字附件在正文里记成的种类（块上写名字，不编号）。
const TEXT: &str = "text";

impl App {
    /// 这句话放不进一行：`attach` 的转成文字附件交回；转不了的弹一句，交回 `Err`，字由调的那头放回输入框。放得下的是 `Ok(None)`。
    pub(super) fn oversized(&mut self, draft: &Draft, attach: bool) -> Result<Option<Pasted>, ()> {
        let full = draft.expand();
        if line_limit::fits(&full) {
            return Ok(None);
        }
        let most = self.config.attachments.max_mib * 1024 * 1024;
        if attach && full.len() <= most {
            let name = pasted_name(&jiff::Zoned::now());
            let note = self.config.text.too_long_attached.replace("{name}", &name);
            self.hint(note, true);
            return Ok(Some(Pasted { name, text: full }));
        }
        let note = self
            .config
            .text
            .too_long
            .replace("{size}", &line_limit::size(&full));
        self.hint(note, false);
        Err(())
    }

    /// 转成附件的那段话在正文里怎么写：一块，块上是附件的名字（同文件块的写法）。
    pub(super) fn pasted_said(&self, pasted: &Pasted) -> (String, Vec<Chip>) {
        let label = self.config.text.attach.file.replace("{name}", &pasted.name);
        let chip = Chip {
            label: label.clone(),
            full: label.clone(),
            kind: Some(TEXT.to_string()),
            file: None,
            note: None,
        };
        (label, vec![chip])
    }
}
