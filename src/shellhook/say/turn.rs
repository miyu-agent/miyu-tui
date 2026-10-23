//! 这一轮的状态：先问终端会话、接上，再说那句话；照推送认出这句话开的那一轮，那一轮结束就收工。

use ratatui::crossterm::event::Event;
use serde_json::json;

use crate::config::Config;
use crate::core::{Command, Core, Push, Update};
use crate::human::Human;
use crate::transcript::{Link, Starting, Transcript};

/// 读配置项、问终端会话的回应认它（`Command::Ask` 的编号）。
const TIMELINE_TAG: u64 = 1;
/// 同上：终端会话。
const TERMINAL_TAG: u64 = 2;
/// 同上：配色那几项。
const THEME_TAG: u64 = 3;

/// 主循环等的东西。
pub enum Incoming {
    Core(Update),
    Key(Event),
}

/// 这一轮的状态。
pub struct Turn {
    pub transcript: Transcript,
    /// 确认和提问的抽屉。
    pub asking: super::asking::Asking,
    pub human: Human,
    /// 配色（`look.rs`）。
    pub look: super::look::Look,
    /// 后台任务、待办（同全屏的任务表）。
    pub board: crate::jobs::Board,
    /// 主题包（`pack.rs`）：读 `tui.look`、换上。
    pack: super::pack::Pack,
    /// 打的是 `/compact`（带给摘要的要求的）：交 `session.compact`，不当一句话发。
    pub compact: Option<Option<String>>,
    /// 交了 `session.compact`：认它开的那一轮。
    manual: bool,
    /// 压缩进度条走满时的抖动（同全屏）。
    rng: crate::rng::Rng,
    /// 答话时插的话（`turn/interject.rs`）。
    interject: interject::Interject,
    /// 打的 `/dream` 正在整理（`dreaming.rs`）。
    pub(super) dream: Option<crate::dream::Dream>,
    /// 这一轮待办变过：收尾时把最后的样子印出来。
    pub todos_changed: bool,
    /// 这一轮派出去的后台任务（不算前台跑的子代理）：收工时还没结束的交给盯后台的进程（`follow.rs`）。
    pub jobs: Vec<crate::core::JobStart>,
    /// 其中结束了的任务编号。
    pub ended_jobs: Vec<String>,
    /// 还没发出去的那句话：问到终端会话、接上以后发。
    pending: Option<String>,
    /// 跟这句话一起发的附件（`Ctrl+V` 贴的图、文件）。
    pub files: Vec<std::path::PathBuf>,
    /// 这句话替换上一轮重来（`/edit` 改过的）。
    pub redo: bool,
    /// `/redo`：上一轮原样重来，不带字。
    pub bare_redo: bool,
    /// 终端界面不认的斜杠命令（核心的命令模板、扩展的）：整行交给核心的 `command.run`，不当一句话发。
    pub slash: bool,
    /// 这一句在哪个目录说。
    cwd: Option<String>,
    /// 接上了终端会话、在等它最新的一页读回来：读完再发，补发的那一页里同样的话不会认成这一句。
    paging: bool,
    /// 那句话发出去了：这以后推回来的 `said` 才认。
    sent: bool,
    /// 你说的那句：核心落了盘推回来（`Said`）照它认序号。
    said: String,
    /// 你说的那句落盘的序号。
    said_seq: Option<u64>,
    /// 这句话开的那一轮：只印这一轮的（接上的会话先补发最新一页，那些不印）。
    pub ours: Option<u64>,
    /// 正在跑的那一轮（发出去以后推来的）：你说的话排着、被它听到了的，就是这一轮。
    running: Option<u64>,
    /// 她还没看到的回报（`unseen.rs`）：这一轮结束时还有任务表里的，核心马上叫醒她再开一轮。
    unseen: super::unseen::Unseen,
    /// 这一轮结束了、还有她没看到的回报：等核心开叫醒的那一轮（`session.rs` 最多等多久）。
    pub lingering: bool,
    /// 这一轮结束后叫醒她的那几轮：接着印、等它结束（实测：任务在这一轮里跑完，叫醒的那一轮没人印）。
    pub woken: Vec<u64>,
    /// 问终端会话问得怎样：这个终端的键、只问不开、闲置久了换了新的（`terminal.rs`）。
    pub terminal: super::terminal::Asked,
    /// 这期间提前敲的字：交还给 shell 的命令行。
    pub typed: String,
    /// 按过 Ctrl+C。
    pub interrupted: bool,
    /// 这句话开的那一轮怎么结束的：收工时照它弹通知。
    pub ended: Option<crate::core::EndReason>,
    /// 收尾了。
    pub done: bool,
}

impl Turn {
    /// 要说 `text`，在 `cwd` 说。
    pub fn new(text: String, cwd: Option<String>, look: super::look::Look) -> Self {
        Self::blank(Some(text), cwd, look)
    }

    /// 接上一轮已经开了的（后台回报叫醒她的那一轮，`report.rs`）：不说话，照 `Open` 接上会话、补完最新一页以后画它。
    pub fn attach(session: String, turn: u64, look: super::look::Look) -> Self {
        let mut this = Self::blank(None, None, look);
        this.transcript.starting = Starting::No;
        this.transcript.session = Some(session);
        this.transcript.link = Link::Ready;
        this.sent = true;
        this.paging = true;
        this.ours = Some(turn);
        this
    }

    /// 画完了：配色交给下一件（`report.rs`）。
    pub fn into_look(self) -> super::look::Look {
        self.look
    }

    /// 排哪几条（`view.rs`）。
    pub fn scope(&self) -> super::view::Scope {
        // 交给核心的斜杠命令只回了一句（`/dream` 的回执、被拒）照全部印；回执到之前什么都不画（2026-10-22 验收）。
        if self.manual || (self.done && self.ours.is_none() && self.woken.is_empty()) {
            return super::view::Scope::All;
        }
        super::view::Scope::Turns(
            self.ours
                .into_iter()
                .chain(self.woken.iter().copied())
                .collect(),
        )
    }

    fn blank(text: Option<String>, cwd: Option<String>, look: super::look::Look) -> Self {
        let text = text.unwrap_or_default();
        let mut transcript = Transcript::default();
        if !text.is_empty() {
            transcript.user(text.clone(), Vec::new());
            // fish 已经在提示符那一行重放过这句，不再画（答话时插的话照画，`view.rs`）。
            if let Some(said) = transcript.entries.last_mut() {
                said.hidden = true;
            }
            transcript.starting = Starting::Waiting;
        }
        Self {
            transcript,
            asking: super::asking::Asking::default(),
            human: Human::default(),
            look,
            board: crate::jobs::Board::default(),
            pack: super::pack::Pack::default(),
            compact: None,
            manual: false,
            rng: crate::rng::Rng::from_clock(),
            interject: interject::Interject::default(),
            dream: None,
            todos_changed: false,
            jobs: Vec::new(),
            ended_jobs: Vec::new(),
            pending: (!text.is_empty()).then(|| text.clone()),
            files: Vec::new(),
            redo: false,
            bare_redo: false,
            slash: false,
            cwd,
            paging: false,
            sent: false,
            said: text,
            said_seq: None,
            ours: None,
            running: None,
            unseen: super::unseen::Unseen::default(),
            lingering: false,
            woken: Vec::new(),
            terminal: super::terminal::Asked::default(),
            typed: String::new(),
            interrupted: false,
            ended: None,
            done: false,
        }
    }

    /// 问核心终端会话是哪一个（`session.terminal`，要新开的开在 shell 所在的目录）。
    pub fn ask_terminal(&self, core: &Core) {
        core.send(Command::Ask {
            tag: TERMINAL_TAG,
            method: "session.terminal",
            params: self.terminal.params(self.cwd.as_deref()),
        });
    }

    /// 那句话发出去：接上了终端会话的说进它，没问到的（核心旧）照新会话开。
    fn send(&mut self, core: &Core) {
        if let Some(text) = self.pending.take() {
            self.sent = true;
            if self.bare_redo {
                core.send(Command::Redo {
                    text: None,
                    files: None,
                });
                return;
            }
            if let Some(words) = self.compact.take() {
                // 压缩那几行不带回合号：读回来的那一页清掉，之后来的都是这一轮的，照全部印（`scope`）。
                self.transcript.entries.clear();
                self.transcript.manual_compaction = true;
                self.manual = true;
                core.send(Command::Compact(words));
                return;
            }
            if self.slash {
                super::dreaming::start(self, &text, core);
                core.send(Command::Slash(text));
                return;
            }
            // `/edit` 改过的：替换上一轮重来（附件没贴新的照原来的）。
            if self.redo {
                core.send(Command::Redo {
                    text: Some(text),
                    files: (!self.files.is_empty()).then(|| self.files.clone()),
                });
                return;
            }
            core.send(Command::Send {
                text,
                files: self.files.clone(),
                cwd: self.cwd.clone(),
            });
        }
    }

    /// 核心那边来的：终端会话、头的配置、给人看的字归这里，别的交给正文。
    pub fn core(&mut self, update: Update, config: &mut Config, core: &Core) {
        // 终端会话的推送包着会话编号来的照常办（同全屏 `app/sessions.rs` 的 `route`），别的会话的不管。
        let update = match update {
            Update::Elsewhere { session, update }
                if self.transcript.session.as_deref() == Some(session.as_str()) =>
            {
                *update
            }
            Update::Elsewhere { .. } => return,
            update => update,
        };
        if super::dreaming::update(self, &update, core) {
            return;
        }
        self.watch(&update);
        self.follow_interject(&update);
        match self.pack.update(&update, config, core) {
            super::pack::Applied::No => {}
            super::pack::Applied::Yes => return,
            // 主题包换上了，`timeline` 照它重读：时间线的配置项再读一遍盖上（同全屏 `apply_layout_settings`）。
            super::pack::Applied::Retimeline => {
                core.send(Command::Ask {
                    tag: TIMELINE_TAG,
                    method: "config.get",
                    params: json!({ "keys": super::settings::layout_keys() }),
                });
                return;
            }
        }
        if let Update::Push(push) = &update {
            let owner = self.transcript.session.clone().unwrap_or_default();
            if self.asking.push(&owner, push, &mut self.transcript, config) {
                return;
            }
        }
        match update {
            Update::Answer { tag, result } if tag == TERMINAL_TAG => {
                if self.terminal.again(&result) {
                    self.ask_terminal(core);
                } else if let Some(found) = self.terminal.answered(result.ok().as_ref()) {
                    self.transcript.session = Some(found.session.clone());
                    self.transcript.link = Link::Ready;
                    core.send(Command::Open {
                        session: found.session,
                        keep: false,
                    });
                    // 最新一页读回来（`Paged`、读不成的 `Older`）再发。
                    self.paging = true;
                } else if self.terminal.none {
                    self.done = true;
                } else {
                    self.send(core);
                }
            }
            update @ (Update::Paged(_) | Update::Older { .. }) if self.paging => {
                self.paging = false;
                self.transcript.update(update, &config.text);
                // 那一页里看着还「排着」的旧话（「听到了」是瞬时的、补发里没有）藏起来，开轮时别当成带上的话（2026-10-23）。
                for entry in self.transcript.entries.iter_mut().filter(|e| e.queued) {
                    entry.hidden = true;
                }
                self.send(core);
            }
            Update::HeadConfig(head) => {
                super::settings::language(&head.language, config, core);
                super::pack::Pack::ask(core);
                core.send(Command::Ask {
                    tag: THEME_TAG,
                    method: "config.get",
                    params: json!({ "keys": super::look::Look::keys() }),
                });
                core.send(Command::ListPersonas);
                core.send(Command::Ask {
                    tag: TIMELINE_TAG,
                    method: "config.get",
                    params: json!({ "keys": super::settings::layout_keys() }),
                });
            }
            Update::Answer { tag, result } if tag == TIMELINE_TAG => {
                if let Ok(got) = result {
                    config.timeline.apply(&got);
                    config.layout.apply_keys(&got);
                }
            }
            Update::Answer { tag, result } if tag == THEME_TAG => {
                self.look.configured(result.as_ref().ok());
            }
            Update::Personas(list) => self.look.personas(list),
            Update::SessionPersona(id) => {
                self.look.persona(id.clone());
                self.transcript
                    .update(Update::SessionPersona(id), &config.text);
            }
            // 后台任务记进任务表：补发的那一页里以前派的也记，这一轮里结束了才写得出那一行（2026-10-20 验收 SH-30：以前派的
            // 结束了，绿点落到了最后）；这一轮派的收工时还没结束的交给盯后台的。前台跑的子代理不是后台任务（同全屏）。
            Update::Push(Push::JobStarted(start) | Push::JobEarlier(start))
                if !start.foreground =>
            {
                self.board.start(&start, None, std::time::Instant::now());
                if self.live() {
                    self.jobs.push(start);
                }
            }
            // 任务结束了：正文里照全屏写那一行（绿点、红叉）；补发的那一页里的只从任务表里划掉。
            Update::Push(Push::JobEnded(end)) => {
                let now = std::time::Instant::now();
                let known = self.board.end(&end, now);
                if self.live() {
                    self.unseen.ended(known.is_some(), end.reason);
                    if let Some(id) = known {
                        crate::jobs::announce(
                            &self.board,
                            &mut self.transcript,
                            id,
                            now,
                            &config.text.jobs,
                        );
                        self.ended_jobs.push(end.job);
                    }
                }
            }
            // 待办换了：发出去以后推来的才算（补发的那一页里的是以前的）。
            Update::Push(Push::Todos { todos }) if self.sent => {
                self.board.set_todos(&todos);
                self.todos_changed = true;
            }
            Update::Human(human) => self.human = human,
            update @ (Update::NoCoreBin
            | Update::Failed(_)
            | Update::Missing(_)
            | Update::SessionGone(_)) => {
                self.transcript.update(update, &config.text);
                self.done = true;
            }
            // 交给核心的那一行办完了：有回执的（`/dream` 这类不开轮的）写一行收工；没回执的（命令模板）等它开的那一轮。
            Update::CommandRan { said, .. } if self.slash && self.ours.is_none() => {
                if !said.is_empty() {
                    self.transcript.entries.clear();
                    self.transcript.note(crate::transcript::Kind::Note, said);
                    self.done = true;
                }
            }
            Update::SlashRefused { message, .. } if self.slash => {
                let text = config.text.refused.replace("{reason}", &message);
                self.transcript.entries.clear();
                self.transcript.note(crate::transcript::Kind::Error, text);
                self.done = true;
            }
            update @ (Update::Refused { .. } | Update::Unsent { .. }) => {
                // 还没开轮就被拒的（`/compact` 没有能压的这类）：正文里只留那一句，照全部印（`scope`）。
                if self.ours.is_none() {
                    self.transcript.entries.clear();
                }
                self.transcript.update(update, &config.text);
                if self.ours.is_none() {
                    self.done = true;
                }
            }
            update => self.transcript.update(update, &config.text),
        }
        self.take_back();
        // 这句话开的那一轮结束了：收工（接上的等最新一页补完；插的话没落着的不收，`turn/interject.rs`）。
        if self.ours.is_some()
            && !self.paging
            && self.transcript.running.is_none()
            && !self.interject.waiting()
        {
            // 说话的那一句（`report.rs` 接上的不算：叫醒的那一轮归盯后台的记）：有任务在这一轮里结束的先等等。
            if self.unseen.owed() && !self.said.is_empty() {
                self.lingering = true;
            } else if !self.transcript.compacting_shown() {
                // 压好了、进度条还在走满的等它换成结果（`climb`）。
                self.done = true;
            }
        }
    }

    /// 每一拍追一下压缩那一行的进度条（同全屏 `app` 的 `tick`）；`/compact` 那一轮已经结束、走满换成结果了的收工。
    pub fn climb(&mut self, config: &Config) {
        let layout = &config.layout;
        let now = std::time::Instant::now();
        self.transcript
            .climb(now, layout.bar.width, &layout.compaction, &mut self.rng);
        if self.manual
            && self.ours.is_some()
            && self.transcript.running.is_none()
            && !self.transcript.compacting_shown()
        {
            self.done = true;
        }
    }

    /// 现在推来的是这一轮里的：发出去了、补发的那一页读完了。
    fn live(&self) -> bool {
        self.sent && !self.paging
    }

    /// 还没结束的后台任务。
    pub fn running_jobs(&self) -> Vec<crate::core::JobStart> {
        self.jobs
            .iter()
            .filter(|j| !self.ended_jobs.contains(&j.job))
            .cloned()
            .collect()
    }

    /// 认出你说的那句落盘的序号、它开的那一轮。
    fn watch(&mut self, update: &Update) {
        // 正在跑哪一轮：补发的那一页里开了还没结束的也算（说话时她正在跑它）。
        match update {
            Update::Push(Push::TurnStarted(turn, trigger)) => {
                self.running = Some(*turn);
                // 照序号认出的叫醒她的那一轮，等着的时候开的：接着印。
                if self.live() && self.unseen.woke(*trigger).is_some() && self.lingering {
                    self.woken.push(*turn);
                    self.lingering = false;
                }
            }
            Update::Push(Push::Reported(seq)) if self.live() => self.unseen.reported(*seq),
            Update::Push(Push::Heard(seen)) if self.live() => self.unseen.heard(*seen),
            Update::Push(Push::TurnEnded(reason)) => {
                let waited = self.running == self.ours
                    || self.running.is_some_and(|t| self.woken.contains(&t));
                if self.ours.is_some() && waited && !self.paging {
                    self.ended = Some(reason.clone());
                }
                self.running = None;
            }
            _ => {}
        }
        // 接上已开的那一轮的不认：没说话，这一轮是给定的。
        if !self.sent || self.said.is_empty() {
            return;
        }
        match update {
            // 这条连接自己发的话推回来是 `user_message`（只带序号），别处看到的是 `said`（带字）：两种都认，认第一个。
            Update::Push(Push::UserMessage(seq)) if self.said_seq.is_none() => {
                self.said_seq = Some(*seq);
            }
            Update::Push(Push::Said { seq, text, .. })
                if *text == self.said && self.said_seq.is_none() =>
            {
                self.said_seq = Some(*seq);
            }
            Update::Push(Push::TurnStarted(turn, trigger))
                if trigger.is_some() && *trigger == self.said_seq =>
            {
                self.ours = Some(*turn);
            }
            // `/compact` 开的那一轮没有你说的话：发出去以后头一个开的就是它（同一个会话里有回合在进行核心拒 `turn_running`）。
            Update::Push(Push::TurnStarted(turn, None))
                if self.manual && self.ours.is_none() && self.live() =>
            {
                self.ours = Some(*turn);
            }
            // 说的时候她正在跑别的一轮（回报叫醒的）：这句话排着，被那一轮听到了就算在那一轮里，不另开一轮（实测）。
            Update::Push(Push::Heard(seen))
                if self.ours.is_none() && self.said_seq.is_some_and(|s| s <= *seen) =>
            {
                self.ours = self.running;
            }
            _ => {}
        }
    }
}

mod interject;
mod keys;

#[cfg(test)]
mod tests;
