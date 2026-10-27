//! 界面这一头记着的会话：正文里的一条条、在不在跑、连没连上、用了多少。只收核心推来的，不自己编：正文照推送一块块
//! 接起来，思考和调工具记进时间线的一段（`steps.rs`），用量照 `model.called` 加起来。纯状态，不碰终端，好测。

mod attached;
mod beat;
mod blocks;
mod cache;
mod chips;
mod climb;
mod clock;
mod compaction;
mod entry;
mod failure;
pub use failure::RetryAfter;
mod foreign;
mod fresh;
mod goal;
mod gone;
mod jobs;
mod media;
mod model;
mod pages;
mod progress;
mod queue;
mod recap;
mod redo;
mod reply;
mod results;
mod speed;
mod spent;
mod steps;
mod switched;
pub use switched::set_names;
mod turn;
mod undo;
mod words;
pub mod workspace;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::time::Instant;

use crate::config::Texts;
use crate::core::{Bill, CallError, EndReason, Level, Limits, Push, ToolStatus, Update, Usage};

pub use cache::CacheWatch;
pub use chips::Chip;
pub use climb::Progress;
pub use entry::{Entry, JobMark, JobNote, Kind};

pub use steps::{Segment, Step, StepKind, Tally, ToolState};
pub use words::undo_counts;

/// 连核心的状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    /// 还在连。
    Connecting,
    /// 连上了。
    Ready,
    /// 断开了，正在重新连接（「连核心」第 7 条）。
    Reconnecting,
    /// 连不上：给人看的一句（第 8 条）。
    Down(String),
}

/// 发出去了、核心还没开轮（`turn.started` 没来）的那一小段：两下 `Esc` 记下要打断，轮一开就打断（蓝图「按键」`Esc`，2026-10-14）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Starting {
    /// 没在等开轮。
    #[default]
    No,
    /// 发出去了，在等开轮。
    Waiting,
    /// 在等开轮，开了就打断（按过两下 `Esc`）。
    Stop,
}

/// 一块字接到哪里去：回答那一条，或者时间线某一段的某一步。
#[derive(Debug, Clone, Copy)]
enum Slot {
    Reply(usize),
    Step(usize, usize),
}

/// 界面记着的会话。
#[derive(Debug)]
pub struct Transcript {
    /// 正文，从旧到新。
    pub entries: Vec<Entry>,
    /// 这一次请求里第几块接到哪里。每次请求的块从 0 数起，新块开头时覆盖。
    blocks: HashMap<u64, Slot>,
    /// 调了工具、还不知道调用编号的几步，照先后；`message.assistant` 来了按顺序配上。
    unnamed: Vec<(usize, usize)>,
    /// 调用编号到那一步。
    calls: HashMap<String, (usize, usize)>,
    /// 在跑的调用攒着的半行进度（「时间线」第 8a 条）。
    progress: progress::Lines,
    /// 这一轮从什么时候开始跑；没在跑是 `None`。
    pub running: Option<Instant>,
    /// 发出去了、还没开轮（[`Starting`]）。
    pub starting: Starting,
    /// 正文的钟：补发来的照事件的时刻（`clock.rs`）。
    clock: clock::Clock,
    /// 最近一次说话的模型：端点和模型名。
    pub model: Option<(String, String)>,
    /// 连核心的状态。
    pub link: Link,
    /// 会话编号：连上以后才有。
    pub session: Option<String>,
    /// 会话不存在了（review B3）：下一句话先换一份空的正文，再开新会话（`gone.rs`）。
    pub gone: bool,
    /// 会话名称：核心起了名字（`session.meta_changed`）才有。
    pub title: Option<String>,
    /// 标题是人起的（`/rename` 这类，不是内核自动起的）：输入框上边框右边写它（「输入框」第 14 条）。
    pub named: bool,
    /// 实际的权限级别。新会话从工作区、不只读开始（`protocol.md` `session.create` 第 2 条）。
    pub level: Level,
    /// 这个会话从开到现在，每次请求的用量加起来。
    pub total: Usage,
    /// 这个会话花了多少：每次有用量的请求的金额照币种加起来，算不出的数几次（核心 8-15）。
    pub bill: Bill,
    /// 会话用哪个人格、哪个预设（订阅的回应里的，核心 P-1 下、P-2 上）：侧边栏照它写。
    pub persona: Option<String>,
    /// 见 `persona`。
    pub preset: Option<String>,
    /// 这一轮每次请求的用量加起来：收尾行写它（`tui.md`「正文」第 4 条）。
    turn_usage: Usage,
    /// 这一轮主请求出字的速度：收尾那一行写（2026-10-26 项目主人，同网页、核心 SP-1）。
    turn_speed: speed::Speed,
    /// 这一轮开始时的权限级别：收尾行打头的图标照它，之后换了级别也不变。
    turn_level: Level,
    /// 最近一次请求占了多少上下文：输入加输出。
    pub context: u64,
    /// 压过几次、意外断过几次缓存（侧边栏写）。
    pub cache: CacheWatch,
    /// 会话的限额：核心在订阅的回应里给的窗口、压缩线（`core/limits.rs`）。
    pub limits: Limits,
    /// 长期目标现在的样子（订阅回应里的、推来的 `goal.changed`，核心 X-8 补）：输入框上边框右边照它写（「长期目标」第 5 条）。
    pub goal: Option<crate::core::Goal>,
    /// 正在压缩的那一行是第几条（`compaction.rs`）。
    compacting: Option<usize>,
    /// 这个头发了 `/compact`、还没压完：旧核心的进度不带 `trigger` 时照它认手动（`compaction.rs`）。
    pub manual_compaction: bool,
    /// 这一轮她出过字了（来过一块）：出过就不再算在等第一个字（[`Transcript::waiting`]）。
    spoke: bool,
    /// 这一轮是手动压缩（`turn.started` 没有 `trigger`，施工 6-8）：压好了不另起收尾行。
    manual: bool,
    /// 这一轮是清空（`/clear`）：结束时不另起收尾行、不接用时（「正文」第 9 条）。
    cleared: bool,
    /// 出字的速度：最近一次请求的、这个会话的平均（`speed.rs`）。
    pub speed: speed::Speed,
    /// 正在重试时给人看的一句。
    pub retry: Option<String>,
    /// 这一轮最后一次请求出的错；后来又成了的清掉。
    failure: Option<CallError>,
    /// 换模型、冷却（`model.rs`）。
    models: model::ModelWatch,
    /// 在跑的这一轮的编号：这期间收到的都记在它名下。
    turn: Option<u64>,
    /// 这一轮的请求带上了第几条为止（`queue.rs` 的 `took`、`stamped`）。
    seen: u64,
    /// 被退回的排队消息：字和里面的粘贴块，等输入框拿走（`take_returned`）。
    returned: Vec<(String, Vec<Chip>)>,
    /// 这一轮是排着的话开的：那几句照排着时的样子，一句一条（`queue.rs` 的 `takeback`）。
    opened_by: Vec<(String, Vec<Chip>)>,
    /// 最近一次 `turn.reverted` 撤掉的几轮：撤销的回应来了，照它找你说的那句全文。
    reverted: Vec<u64>,
    /// 下一条正文的编号。
    next_id: u64,
    /// 老会话按页读读到哪了（`pages.rs`）。
    older: pages::Older,
    /// 会话在哪个目录干活（订阅回应、`session.workspace_changed`，核心 9-7 上）；还没开会话、核心旧的是 `None`。
    pub workspace: Option<String>,
}

impl Transcript {
    /// 你说了一句。
    pub fn user(&mut self, text: String, pasted: Vec<Chip>) {
        self.push(Kind::User, text);
        if let Some(entry) = self.entries.last_mut() {
            entry.pasted = pasted;
        }
    }

    /// 在正文末尾写一句旁白，不属于哪一轮。
    ///
    /// 插进正文的信息（后台任务的回报、子代理的回报、回顾、换模型这些旁白）画在收起的段下面：前面在进行的那一段
    /// 先收起，接着的步另起一段（蓝图 `tui.md`「时间线」第 21 条；2026-10-02 项目主人报：回报直接插进展开的
    /// 一段里，连接线断、前面那段一直不收）。
    pub fn note(&mut self, kind: Kind, text: String) {
        self.finish_segment();
        let id = self.fresh_id();
        self.entries.push(Entry::new(id, kind, text));
    }

    /// 正文里有正在压缩的那一行（停下来等、手动压缩）。
    pub fn compacting_shown(&self) -> bool {
        self.compacting.is_some()
    }

    /// 有没有还在进行的步骤、正在压缩：有就要转圈。
    pub fn busy(&self) -> bool {
        self.compacting.is_some()
            || self
                .entries
                .iter()
                .filter_map(|e| e.segment.as_ref())
                .any(|s| s.steps.iter().any(Step::busy))
    }

    /// 收一条核心那边的消息。
    pub fn update(&mut self, update: Update, texts: &Texts) {
        match update {
            Update::Limits(limits) => self.limits = limits,
            Update::Ready(session) => {
                self.link = Link::Ready;
                self.opening(session);
            }
            Update::NoCoreBin => self.link = Link::Down(texts.no_core_bin.clone()),
            Update::Missing(path) => {
                self.link = Link::Down(texts.missing_core.replace("{path}", &path));
            }
            Update::Reconnected => self.link = Link::Ready,
            // 另外订阅着的会话推来的：界面照会话分给那个会话的正文（`app/sessions.rs`）。
            // 改名成了只弹提示（界面那头办了），标题照推送换。
            Update::Elsewhere { .. }
            | Update::Output { .. }
            | Update::Renamed(_)
            | Update::Sessions(_)
            | Update::HeadConfig(_)
            | Update::Human(_)
            | Update::ToolsChanged
            | Update::PackageProgress(_)
            | Update::Older { .. }
            | Update::Workspace { .. }
            | Update::SessionGone(_)
            | Update::Catalog(_)
            | Update::SlashRefused { .. }
            | Update::Widgets(_)
            | Update::WidgetChanged { .. }
            | Update::PackageCallRefused { .. }
            | Update::OpenPage(_)
            | Update::DirChecked(_)
            | Update::AnswerFilesLost(_)
            | Update::Account(_) => {}
            Update::CoolingUntil(until) => self.cooling_until(until),
            Update::CurrentModel(current) => self.current_model(current),
            Update::SessionPersona(persona) => self.persona = Some(persona),
            Update::Paged(more) => self.paged(more, texts),
            Update::Snapshot(s) => self.snapshot(s.usage.as_ref(), s.level, s.compactable),
            Update::SessionPreset(preset) => self.preset = Some(preset),
            Update::Configured(reference) => self.configured(reference),
            Update::Choices(_)
            | Update::Files { .. }
            | Update::Efforts(_)
            | Update::LinkCard { .. }
            | Update::BlobSaved { .. }
            | Update::Mermaid { .. }
            | Update::Answer { .. }
            | Update::CommandRan { .. }
            | Update::Dream { .. }
            | Update::UsageRows { .. }
            | Update::Personas(_)
            | Update::Packages(_)
            | Update::Presets(_)
            | Update::AnswerRefused { .. }
            | Update::ConfigChanged
            | Update::SessionChanged(_) => {}
            Update::Failed(reason) => {
                self.link = Link::Down(texts.core_failed.replace("{reason}", &reason));
            }
            // 核心断开了：在进行的那一轮当场收尾，马上重连（「连核心」第 7 条）。
            Update::Disconnected => {
                self.starting = Starting::No;
                self.cut_off(texts);
                self.link = Link::Reconnecting;
            }
            // 认得的原因码界面只弹提示框，到不了这里；认不得的照核心的原话写进正文（`tui.md`「正文」第 6 条）。
            // 没发出去的，先画上的那句界面已经撤掉了（`app/redo.rs`）。
            Update::Refused { message, .. } | Update::Unsent { message, .. } => {
                self.note(Kind::Error, texts.refused.replace("{reason}", &message));
            }
            Update::Push(push) => self.apply(push, texts),
            Update::Recap(text) => self.recap(&text, texts),
            // 撤销：记一行说明，全文照这一次撤掉的第一轮里你说的话，没有的照核心给的第一行。
            Update::Undone {
                restore: false,
                report,
            } => self.undo_line(report),
            // 恢复：那几轮已经照 `turn.unreverted` 显示回来了，去掉最近的那一行撤销说明，不另写一句。
            Update::Undone { restore: true, .. } => self.undo_gone(),
        }
    }

    fn apply(&mut self, push: Push, texts: &Texts) {
        match push {
            Push::TurnStarted(turn, trigger) => self.start(turn, trigger),
            // 你发的话照先后落盘：配给最早那句还没有序号的。
            Push::UserMessage(seq) => self.stamped(seq),
            Push::Withdrawn(seqs) => self.withdraw(&seqs),
            // 去掉标题推来的是空的：读成没有标题。
            Push::Title { title, named } => {
                self.named = named && !title.is_empty();
                self.title = (!title.is_empty()).then_some(title);
            }
            Push::Workspace(cwd) => self.workspace_changed(cwd, texts),
            Push::Policy { level, read_only } => {
                self.set_level(if read_only { Level::ReadOnly } else { level });
            }
            Push::Reverted(turns) => {
                self.cache.reverted();
                self.hide(&turns, true);
                self.decided(&turns, true);
                self.reverted = turns;
            }
            Push::Unreverted(turns) => {
                self.hide(&turns, false);
                self.decided(&turns, false);
            }
            Push::UndoLine => self.undo_line(crate::core::Report::default()),
            Push::UndoFiles(files) => self.undo_files(files),
            Push::UndoGone => self.undo_gone(),
            Push::Model { endpoint, model } => self.show_model(model, endpoint),
            Push::Heard(seen) => self.heard(seen),
            Push::Clock(at) => self.clock.set(at),
            Push::Tried { endpoint, model } => self.tried(&endpoint, &model),
            Push::ModelChanged {
                endpoint,
                model,
                limits,
                failover,
                reference,
                effort,
            } => self.model_changed(endpoint, model, limits, failover, reference, effort, texts),
            Push::ModelSet {
                reference,
                replaced,
            } => self.model_set(reference, replaced, texts),
            Push::PersonaSet(id) => self.persona_set(id, texts),
            Push::PresetSet(id) => self.preset_set(id, texts),
            Push::Said {
                seq,
                text,
                attached,
            } => {
                self.said(seq, text);
                self.attach_to(seq, &attached, &texts.attach);
            }
            Push::BlockStart { index, block } => {
                self.spoke = true;
                self.retry = None;
                self.close_open_blocks();
                let slot = self.open_block(block);
                self.blocks.insert(index, slot);
            }
            Push::Delta { index, text } => self.delta(index, &text),
            Push::BlockEnd(index) => self.block_end(index),
            Push::Calls(ids) => {
                for (id, at) in ids.into_iter().zip(std::mem::take(&mut self.unnamed)) {
                    if let Some(step) = self.step_mut(at) {
                        step.call_id = Some(id.clone());
                    }
                    self.calls.insert(id, at);
                }
            }
            push @ Push::ToolResult { .. } => self.tool_result(push),
            Push::ToolProgress { call_id, text } => self.tool_progress(&call_id, &text),
            Push::Media(images) => self.sent(images),
            Push::Shown(file) => self.shown(file),
            Push::ToolImages { call_id, blobs } => {
                let at = self.calls.get(&call_id).copied();
                if let Some(step) = at.and_then(|at| self.step_mut(at)) {
                    step.images = blobs;
                }
            }
            // 别处来的话、后台任务：界面照会话、任务表先办了（`app/sessions.rs`），正文不直接收。
            Push::GoalRound { seq, round } => self.goal_round(seq, round, &texts.goal),
            Push::Goal(goal) => self.goal = goal,
            Push::Foreign(_)
            | Push::JobStarted(_)
            | Push::JobEarlier(_)
            | Push::JobMessaged(_)
            | Push::Reported(_)
            | Push::JobEnded(_)
            | Push::PeerIdle { .. } => {}
            Push::Usage(usage) => self.used(usage),
            Push::Billed(cost) => self.bill.add(cost.as_ref()),
            Push::Sent {
                seen,
                changed,
                summary,
            } => {
                self.cache.sent(seen, changed, summary);
                self.took(seen);
            }
            Push::Compaction(push) => self.compaction(push, texts),
            Push::Compacted { clear } => {
                self.cache.compacted();
                if clear {
                    self.cleared(texts);
                }
            }
            // 回顾这类辅助请求：只算进累计用量（蓝图「回顾」第 5 条）。
            Push::AuxUsage(usage) => self.total.aux += usage.input() + usage.output,
            Push::Recapped(text) => self.recap(&text, texts),
            // 确认、提问归抽屉（`app/asking.rs`）：正文里的结果由那边写。
            Push::Asking(_) => {}
            // 待办归待办表（`app/sessions.rs`）。
            Push::Todos { .. } => {}
            Push::Compactable(on) => self.cache.compactable = Some(on),
            Push::Speed { output, ms } => self.sped(output, ms),
            Push::CallFailed(error) => self.call_failed(error),
            // 后来又成了：前面报过的错不算（蓝图「正文」第 4 条）。
            Push::CallOk => self.call_ok(),
            Push::Retry {
                attempt,
                limit,
                message,
            } => {
                self.retry = Some(
                    texts
                        .retry
                        .replace("{attempt}", &attempt.to_string())
                        .replace("{limit}", &limit.to_string())
                        .replace("{message}", &message),
                );
            }
            Push::TurnEnded(reason) => self.end(reason, texts),
            Push::CommandRan => self.settle(),
        }
    }

    /// 一共收起了几段（做完的时间线）：变多了就是刚收起一段，界面照它放开一次视口（蓝图「正文」第 1 条）。
    pub fn folds(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.segment.as_ref().is_some_and(|s| s.finished))
            .count()
    }

    /// 拿走被退回的排队消息的字，照先后。
    pub fn take_returned(&mut self) -> Vec<(String, Vec<Chip>)> {
        std::mem::take(&mut self.returned)
    }

    /// 记一条。你说的话等开轮时再归；别的归到在跑的这一轮。
    fn push(&mut self, kind: Kind, text: String) {
        let user = kind == Kind::User;
        let queued = user && self.running.is_some();
        let turn = if user { None } else { self.turn };
        // 你说的话记发出去时的级别（竖线的颜色）；收尾行记这一轮开始时的（打头的图标）。
        let level = match kind {
            Kind::User => self.known(self.level),
            Kind::Done | Kind::Cut => self.known(self.turn_level),
            _ => None,
        };
        let id = self.fresh_id();
        let at = user.then(|| self.clock.wall().timestamp());
        self.entries.push(Entry {
            turn,
            queued,
            level,
            at,
            ..Entry::new(id, kind, text)
        });
    }

    /// 发一个新编号。
    fn fresh_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }
}
