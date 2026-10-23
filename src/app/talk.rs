//! 会话（蓝图 `app.md` 的 `talk` 块）：正在看的会话和它的任务表，停放着的会话、切进子会话前的主会话、`/new` 以后
//! 还在跑的旧会话、切过去还在补发的那个，最近发出去的一句、打断以后要退回的，确认和提问的抽屉，新会话要选的人格
//! 和预设。

use super::{Unsent, new_session, sessions, takeback};
use crate::core::{Bill, Usage};
use crate::drawer::Drawers;
use crate::jobs::{Board, Job};
use crate::transcript::Transcript;

/// 会话。
pub struct Talk {
    /// 还没开会话：照选了的人格、预设预览的上下文，用了多少、窗口（核心 CX-1 下，底栏写，`context_box.rs`）。
    pub preview: Option<(u64, Option<u64>)>,
    /// 会话：正文、在不在跑、用量。
    pub transcript: Transcript,
    /// 后台任务表和待办。
    pub board: Board,
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
    /// 发了删除、核心还没推来「删掉了」的会话：这之间推来的它们的变化不要（会把删了的又加回列表，「会话列表」第 2 条）。
    pub(super) deleting: crate::core::DeletingSessions,
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
    /// 新会话照配置开局的那一级（`permission.level`、`permission.start_read_only`，`head_settings.rs` 读）：首页照它画。
    pub default_level: crate::core::Level,
    /// 这一回的首页上按过 Tab：照按的画，配置再读到也不盖（按的那一级记在核心那头，开会话时补发）。
    pub level_picked: bool,
    /// 人打的 `/dream` 正在整理（`dream.rs`，蓝图「整理记忆」）。
    pub dream: Option<crate::dream::Dream>,
}

impl Talk {
    /// 子代理状态行照的那份任务表：主会话的（切进了子会话的是停放着的主会话的）。
    pub fn tree(&self) -> &Board {
        match &self.home {
            Some(main) => self.parked.get(main).map_or(&self.board, |p| &p.board),
            None => &self.board,
        }
    }

    /// 子代理状态行列的子代理：主会话任务表里在跑的，加上正在看的那个（报完了也留着，「切进子会话」第 5 条）。
    pub fn listed_agents(&self) -> Vec<&Job> {
        self.tree().listed(self.viewing())
    }

    /// 正在看的子会话的子代理叫什么（输入框右上角写，「切进子会话」第 2 条）；看着主会话是 `None`。
    pub fn viewing_title(&self) -> Option<String> {
        self.viewing().and_then(|child| self.agent_title(child))
    }

    /// 子会话 `child` 的子代理叫什么（在哪个会话的任务表里都找）。
    pub(super) fn agent_title(&self, child: &str) -> Option<String> {
        self.boards().find_map(|b| {
            b.jobs
                .iter()
                .find(|j| j.session.as_deref() == Some(child))
                .map(|j| j.title.clone())
        })
    }

    /// 正在看的这棵会话树：主会话和它派出去的子代理（一层层往下，照任务表）。别的会话问的不弹（「确认和提问的抽屉」第 10 条）。
    pub(super) fn family(&self) -> Vec<String> {
        let Some(main) = self.main_session() else {
            return Vec::new();
        };
        let board_of = |s: &str| {
            if self.transcript.session.as_deref() == Some(s) {
                Some(&self.board)
            } else {
                self.parked.get(s).map(|p| &p.board)
            }
        };
        let mut family = vec![main];
        let mut at = 0;
        while at < family.len() {
            if let Some(board) = board_of(&family[at].clone()) {
                for child in board.jobs.iter().filter_map(|j| j.session.clone()) {
                    if !family.contains(&child) {
                        family.push(child);
                    }
                }
            }
            at += 1;
        }
        family
    }

    /// 每一份任务表：正在看的、停放着的。
    pub(super) fn boards(&self) -> impl Iterator<Item = &Board> {
        std::iter::once(&self.board).chain(self.parked.values().map(|p| &p.board))
    }

    /// 子会话 `child` 自己又派了几个在跑的子代理（状态行里写「（+N）」）。
    pub fn nested_agents(&self, child: &str) -> usize {
        if self.transcript.session.as_deref() == Some(child) {
            return self.board.agents().len();
        }
        self.parked.get(child).map_or(0, |p| p.board.agents().len())
    }

    /// 正在看的会话连同它派出去的子代理（一层层往下）一共用了多少：框下面那一行、侧边栏写（2026-09-30 项目主人：
    /// 子代理用的也算进主会话）。子代理的照它停放着的正文，订阅上以前用的看不到。
    pub fn usage_total(&self) -> Usage {
        super::sessions::notes::tree_usage(self.transcript.total, &self.board, &self.parked)
    }

    /// 花了多少：连同子代理的，和 [`Talk::usage_total`] 一个算法（蓝图「配置与模型」第 5 条）。
    pub fn bill_total(&self) -> Bill {
        super::sessions::notes::tree_bill(&self.transcript.bill, &self.board, &self.parked)
    }

    /// 正在看的子会话；看着主会话是 `None`。
    pub fn viewing(&self) -> Option<&str> {
        self.home.as_ref().and(self.transcript.session.as_deref())
    }

    /// 主会话：切进了子会话的是停放着的那个。
    pub fn main_session(&self) -> Option<String> {
        self.home
            .clone()
            .or_else(|| self.transcript.session.clone())
    }

    /// 这个会话还忙着：有一轮在跑，或者还有任务没报完（第 4 条）。
    pub(super) fn busy_here(&self) -> bool {
        self.board.busy() || self.transcript.running.is_some()
    }

    /// 在补发的是不是 `session`。
    pub(super) fn opening(&self, session: &str) -> bool {
        self.opening.as_ref().is_some_and(|(s, _)| s == session)
    }

    /// 是首页：正文里一条都没有，也没在回答（`tui.md`「空会话的首页」第 1 条）。
    pub fn blank(&self) -> bool {
        // 人打的 `/dream` 在整理的不算：那一块画在正文末尾（「整理记忆」第 2 条），首页打的也要换到正文。
        self.transcript.entries.is_empty()
            && self.transcript.running.is_none()
            && self.dream.is_none()
    }
}
