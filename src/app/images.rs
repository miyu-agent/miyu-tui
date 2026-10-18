//! `/images on|off`（蓝图「图片、公式和 mermaid 图」第 10 条，2026-10-18 项目主人）：这次运行贴不贴图，不写配置。
//! 同一个界面开在 herdr 里，电脑上的 kitty 看能贴图，手机 Termius 连进同一个 herdr 看就把贴图的指令印成了字；界面隔着
//! herdr 分不出现在是谁在看，交给人当场关。关了的照终端显示不了图写源码，贴上去的擦掉。

use super::App;

impl App {
    /// `/images on`、`/images off`；不带的在开、关之间换。
    pub(super) fn images_command(&mut self, words: Option<&str>) {
        let [off_note, on_note, none_note, usage] = self.config.text.images.clone();
        let mut figures = self.media.figures.borrow_mut();
        let off = match words.map(str::trim).unwrap_or_default() {
            "off" => true,
            "on" => false,
            "" => !figures.off(),
            _ => {
                drop(figures);
                self.hint(usage, false);
                return;
            }
        };
        if !off && !figures.supported() {
            drop(figures);
            self.hint(none_note, false);
            return;
        }
        figures.set_off(off);
        drop(figures);
        // 关了：贴上去的图删掉、整屏重画（`main.rs`）；开了的照常重排，图重新做。
        self.screen.wipe |= off;
        self.hint(if off { off_note } else { on_note }, false);
    }
}
