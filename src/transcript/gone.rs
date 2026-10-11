//! 会话不存在了（review B3，2026-10-11 项目主人定）：在别处删了，断线重连时才知道；`--resume` 的编号不对。正文末尾红字
//! 写一句，上面的留着看；之后像按过 `/new` 一样，说下一句话时开新会话（蓝图「连核心」第 7 条）。

use super::{Kind, Transcript};

impl Transcript {
    /// 会话不存在了：写一句（`text`），不再对着它（还没开会话时的命令照「还没开」当场答），记着下一句话先换一份空的正文。
    pub fn gone(&mut self, text: String) {
        self.note(Kind::Error, text);
        self.session = None;
        self.gone = true;
    }
}
