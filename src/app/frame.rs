//! 画一帧以前先走的一步（蓝图 `app.md` 第三节）：要改状态的都在这里做完，画面层只照结果画。输入框从有字变空时换一条
//! 提示，同步斜杠命令列表的选中，`@` 文件列表的词变了问核心。

use super::App;
use crate::commands::Spec;
use crate::mention::Found;

/// 这一帧输入框上面的两个列表。
pub struct Lists {
    /// 开着的 `@` 文件列表。
    pub mentions: Option<Found>,
    /// 斜杠命令列表对得上的几条；不开的是 `None`。
    pub matches: Option<Vec<Spec>>,
}

impl App {
    /// 画对话那一帧以前（引导、配置页开着时不走这里）：换提示，算好两个列表（历史列表、后台面板、抽屉、`@` 列表开着时不开斜杠命令列表：几个占
    /// 同一个地方，`tui.md`「输入历史列表」「`@` 文件列表」第 1 条）。
    pub fn before_frame(&mut self) -> Lists {
        self.typing.tips.see(
            self.typing.input.editor.is_empty(),
            self.config.text.tips.len(),
        );
        let mentions = crate::frame_log::section("mention", || self.mention_found());
        let blocked = self.typing.history.open
            || self.boxes.panel.is_some()
            || self.talk.drawers.open()
            || mentions.is_some();
        let matches = if blocked { None } else { self.menu_matches() };
        Lists { mentions, matches }
    }
}
