//! 管理员模式（蓝图 `tui.md`「管理员模式」，2026-10-24 项目主人定）：`/admin`、配置页 `Shift+A` 切同一个状态，开着时连核心
//! 的那一头给改系统那一层的请求带 `scope`（`core/scope.rs`），配置页写系统那一层（`settings/admin.rs`）。每次启动都关着。

use super::App;

impl App {
    /// 进、退管理员模式；不是管理员的提示一句（第 1、2 条）。
    pub(super) fn toggle_admin(&mut self) {
        let words = &self.config.text.admin;
        if !self.link.account.admin {
            let text = words.denied.clone();
            self.hint(text, false);
            return;
        }
        let on = !self.link.core.admin();
        let text = if on { &words.on } else { &words.off }.clone();
        self.set_admin(on);
        self.hint(text, true);
    }

    /// 换成开、关：连核心的那一头、开着的配置页一起换。
    pub(super) fn set_admin(&mut self, on: bool) {
        self.link.core.set_admin(on);
        if let Some(page) = self.boxes.settings.as_mut() {
            page.admin = self.link.account.admin.then_some(on);
        }
    }
}
