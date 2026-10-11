//! 跨重连都记着的连接状态（从 `serve.rs` 拆出来，2026-10-11）：主会话、另外订阅着的、命令对着哪个、每个会话读到哪了。

use std::collections::{BTreeSet, HashMap};

use super::replay::Replay;
use super::{Level, Update, page};

/// 跨重连都记着的：主会话、会话还没开时切的权限级别、另外订阅着的会话、命令对着哪个会话。
#[derive(Debug, Default)]
pub struct Link {
    /// 主会话：界面开的那个。`/new` 以后、说第一句话以前没有。
    pub main: Option<String>,
    /// 会话还没开时切的权限级别：开会话时补发（「权限级别」第 2 条）。
    pub pending: Option<Level>,
    /// 另外订阅着的：子代理的会话、`/new` 以后还有任务在跑的旧会话。
    pub watched: BTreeSet<String>,
    /// 切进了哪个子会话：命令对着它；没切是 `None`，对着主会话（「切进子会话」第 3 条）。
    pub viewing: Option<String>,
    /// 每个会话读到的最后一个序号：同一个序号只办一次，掉了队照它补（「会话列表」第 6、7 条）。
    pub seen: HashMap<String, u64>,
    /// 正在补发的会话：补发来的照 [`Replay`] 读，回应到了算补完。
    pub replays: HashMap<String, Replay>,
    /// 按页读的会话读到哪了：再往前要哪一页（核心 9-6 下）。
    pub pages: HashMap<String, page::Marks>,
    /// 还没开会话时在 `/model` 选的：开会话时带上（「配置与模型」第 1 条）。
    pub pending_model: Option<String>,
    /// 新会话的人格框选的：开会话时带上，造成了才清（人格写错被拒的，下一次照样带，不悄悄换默认）。
    pub pending_persona: Option<String>,
    /// 新会话的预设框选的：和 `pending_persona` 一样。
    pub pending_preset: Option<String>,
    /// 还没开会话时 `/workspace` 换的、验过的目录：开会话时带上（核心 9-7 下）。
    pub pending_cwd: Option<String>,
    /// 连上以后先读主会话的最新一页，再带 `after` 订阅（启动时进最近的那个、`--resume` 的，「会话列表」第 8 条）。
    pub replay_main: bool,
}

impl Link {
    /// 命令对着的会话。
    pub(super) fn target(&self) -> Option<String> {
        self.viewing.clone().or_else(|| self.main.clone())
    }

    /// 不再订阅 `session`：读到哪了、补到哪了都不记（补到一半的也不要：最新一页回来以后不再替它订阅，review B7）。
    pub(super) fn forget(&mut self, session: &str) {
        self.seen.remove(session);
        self.pages.remove(session);
        self.replays.remove(session);
    }

    /// 还要 `session` 推来的：主会话、另外订阅着的。
    pub(super) fn follows(&self, session: &str) -> bool {
        self.main.as_deref() == Some(session) || self.watched.contains(session)
    }

    /// 核心说主会话不存在了（`reason` 是拒绝的原因码；读页、补发、重连订阅时）：不再对着它，交回告诉界面的那一条
    /// （review B3，2026-10-11：原来重连一直订阅它、一直失败，排着的命令全卡住）。别的会话、别的原因交回 `None`。
    pub(super) fn gone(&mut self, session: &str, reason: Option<&str>) -> Option<Update> {
        if reason != Some("session_not_found") || self.main.as_deref() != Some(session) {
            return None;
        }
        self.forget(session);
        self.main = None;
        self.viewing = None;
        Some(Update::SessionGone(session.to_string()))
    }

    /// 重连以后主会话怎么接上：最新一页还没读到的（启动时、切过去、补到一半断了的）重读最新一页（`true`），读到过的带
    /// `after` 接着补（review B1、B7，2026-10-11）。
    pub(super) fn reread_main(&mut self, main: &str) -> bool {
        std::mem::take(&mut self.replay_main)
            || (self.replays.contains_key(main) && !self.seen.contains_key(main))
    }
}
