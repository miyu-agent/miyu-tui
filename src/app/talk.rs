//! 会话（蓝图 `app.md` 的 `talk` 块）：正在看的会话和它的任务表，停放着的会话、切进子会话前的主会话、`/new` 以后
//! 还在跑的旧会话、切过去还在补发的那个，最近发出去的一句、打断以后要退回的，确认和提问的抽屉，新会话要选的人格
//! 和预设。

use super::{Unsent, new_session, sessions, takeback};
use crate::drawer::Drawers;
use crate::jobs::{Board, Feed};
use crate::transcript::Transcript;

/// 会话。
pub struct Talk {
    /// 会话：正文、在不在跑、用量。
    pub transcript: Transcript,
    /// 后台任务表和待办（现在由假数据源推）。
    pub board: Board,
    /// 演示用的假数据源。
    pub feed: Feed,
    /// 停放着的会话：切进子会话时的主会话、子代理的会话、`/new` 以后还有任务在跑的旧会话（`sessions.rs`）。
    pub parked: sessions::Lot,
    /// 切进了子会话：主会话的编号（它停放着）；看着主会话是 `None`。
    pub home: Option<String>,
    /// `/new` 以后还有任务在跑、照样订阅着的旧会话：任务都报完了退订。
    pub left: Vec<String>,
    /// 切过去、还在补发的会话，和原来那个要不要接着订阅：补完了再换上来（`switch.rs`）。
    pub opening: Option<(String, bool)>,
    /// 上次交回的会话列表：再开框先照它画（`switch.rs`）。
    pub sessions_seen: Option<Vec<crate::core::SessionInfo>>,
    /// 最近发出去的一句：没发出去时撤回来（`redo.rs`）。
    pub(super) unsent: Option<Unsent>,
    /// Ctrl+C 打断时要退回排着的话，走到哪一步了（`takeback.rs`）。
    pub takeback: Option<takeback::Takeback>,
    /// 核心推来的确认、提问：问过的、了结过的（`asking.rs`）。
    pub asks: crate::drawer::Asks,
    /// 确认和提问的抽屉：现在这一个和排着的（蓝图「确认和提问的抽屉」）。
    pub drawers: Drawers,
    /// 待办点开了，列出全部（`tui.md`「后台命令、子代理和侧边栏」第 4 条）。
    pub todo_full: bool,
    /// 这一次提前压已经弹过「已触发上下文压缩」（`updates.rs`）。
    pub compaction_announced: bool,
    /// 还没开会话时 `/workspace` 换的、验过的目录：新会话开在这里（`workspace.rs`）。
    pub workspace_pending: Option<String>,
    /// 新会话要选的人格（`new_session.rs`）。
    pub persona: new_session::Pick,
    /// 新会话要选的预设。
    pub preset: new_session::Pick,
    /// `/demo-ask`、`/demo-approve` 各出到第几个。
    pub demo_drawers: (usize, usize),
}
