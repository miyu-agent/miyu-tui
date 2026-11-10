//! 太长的话不发（蓝图 `tui.md`「输入框」第 11 条，施工 YC-11，压测报告 F5）：协议一行最多 1 MiB，放不下的核心整行不认，
//! 原来输入框清空了、话悄悄丢了。回车时先量，放不下的字留在框里、弹一句。

use super::App;
use crate::core::line_limit;

impl App {
    /// 这句话（块换回原文以后的 `full`）放不进一行：弹一句，交回 `true`，字由调的那头放回输入框。
    pub(super) fn too_long(&mut self, full: &str) -> bool {
        if line_limit::fits(full) {
            return false;
        }
        let note = self
            .config
            .text
            .too_long
            .replace("{size}", &line_limit::size(full));
        self.hint(note, false);
        true
    }
}
