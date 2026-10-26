//! 画的时候读的（蓝图 `app.md` 第三节）：配置和内容各块的只读借用，和要读的几样跨块查询（焦点在哪、花了多少、现在的
//! 工作区、选的人格和预设、头像、引导走没走过……）。画面层拿它，不拿整个 `App`；`App` 上同名的查询转交给它。

use std::time::{Duration, Instant};

use super::avatars::{Avatar, Place, home_persona, sidebar_persona};
use super::boxes::Boxes;
use super::link::Link;
use super::mascot::MascotState;
use super::media::Media;
use super::new_session::Kind;
use super::screen::Screen;
use super::talk::Talk;
use super::typing::Typing;
use super::{App, Panel};
use crate::config::Config;
use crate::core::Persona;
use crate::focus::{Button, Focus, Stops};

/// 配置和内容各块，只读。
pub struct Seen<'a> {
    pub config: &'a Config,
    pub talk: &'a Talk,
    pub typing: &'a Typing,
    pub boxes: &'a Boxes,
    pub link: &'a Link,
    pub media: &'a Media,
    /// 终端所在的目录，家目录写成 `~`。
    pub cwd: &'a str,
    /// 界面起来的时刻。
    pub started: Instant,
}

impl App {
    /// 画一帧要的三样：读的那一份，和画的时候能改的上一帧画面、吉祥物（蓝图 `app.md` 第三节）。
    pub fn parts(&mut self) -> (Seen<'_>, &mut Screen, &mut MascotState) {
        let seen = Seen {
            config: &self.config,
            talk: &self.talk,
            typing: &self.typing,
            boxes: &self.boxes,
            link: &self.link,
            media: &self.media,
            cwd: &self.cwd,
            started: self.screen.started,
        };
        (seen, &mut self.screen, &mut self.mascot)
    }

    /// 画的时候读的那一份。
    pub fn seen(&self) -> Seen<'_> {
        Seen {
            config: &self.config,
            talk: &self.talk,
            typing: &self.typing,
            boxes: &self.boxes,
            link: &self.link,
            media: &self.media,
            cwd: &self.cwd,
            started: self.screen.started,
        }
    }
}

impl<'a> Seen<'a> {
    /// 新会话开头必须选的那一样（`new_session.rs`）：画在输入框的位置。
    pub fn start_pick(&self) -> Option<Kind> {
        super::new_session::start_pick(self.talk).filter(|_| !self.entering())
    }

    /// 框下面那一行现在有哪些按钮（从左到右）：有在跑的命令时的后台按钮。
    pub fn footer_buttons(&self) -> Vec<Button> {
        (self.talk.board.shells_running() > 0)
            .then_some(Button::Background)
            .into_iter()
            .collect()
    }

    /// 子代理状态行里焦点能停的几行：主会话那一行加上列着的子代理（`sessions.rs` 的 `listed_agents`）；
    /// 没有列着的子代理，这一块不在，一行都没有。
    pub(super) fn agent_stops(&self) -> usize {
        match self
            .talk
            .listed_agents()
            .len()
            .min(self.config.layout.agent_rows)
        {
            0 => 0,
            n => n + 1,
        }
    }

    /// 焦点现在在哪：按钮、行没了就退回（第 2 条）。
    pub fn focus(&self) -> Focus {
        let buttons = self.footer_buttons();
        let stops = Stops {
            buttons: &buttons,
            agents: self.agent_stops(),
        };
        self.typing.focus.settle(stops)
    }

    /// 花了多少写成字（`$0.42 + ¥1.30`，一笔都算不出的是 `None`）：侧边栏、框下面那一行用。
    pub fn spent(&self) -> Option<String> {
        let bill = self.talk.bill_total();
        crate::money::amounts(&bill, &self.link.currency, &self.config.layout.currencies)
    }

    /// 现在用的工作区：开着的会话照会话的，还没开的照换过的，没换的照终端所在的。
    pub fn current_workspace(&self) -> &'a str {
        match &self.talk.transcript.session {
            Some(_) => self
                .talk
                .transcript
                .workspace
                .as_deref()
                .unwrap_or(self.cwd),
            None => self.talk.workspace_pending.as_deref().unwrap_or(self.cwd),
        }
    }

    /// 开着的会话用的人格、预设，各一句（侧边栏写，订阅回应里的，核心 P-1 下、P-2 上）。
    pub fn session_picks(&self) -> Vec<String> {
        self.words(
            self.talk.transcript.persona.as_deref(),
            self.talk.transcript.preset.as_deref(),
        )
    }

    pub(super) fn words(&self, persona: Option<&str>, preset: Option<&str>) -> Vec<String> {
        let texts = &self.config.text.persona_box;
        let mut out = Vec::new();
        if let Some(id) = persona {
            out.push(texts.chosen.replace("{name}", self.talk.persona.name(id)));
        }
        if let Some(id) = preset {
            out.push(
                texts
                    .preset_chosen
                    .replace("{name}", self.talk.preset.name(id)),
            );
        }
        out
    }

    /// 框里画的：哪一样的一行行、默认的编号。
    pub fn pick_list(&self, kind: Kind) -> (&'a [Persona], Option<&'a str>) {
        let pick = match kind {
            Kind::Persona => &self.talk.persona,
            Kind::Preset => &self.talk.preset,
        };
        (
            pick.list.as_deref().unwrap_or_default(),
            pick.default.as_deref(),
        )
    }

    /// 这一处的人格和它的头像的版本；没有头像的是 `None`。
    pub(super) fn wanted(&self, place: Place) -> Option<(&'a str, &'a str)> {
        let persona = self.persona_at(place)?;
        let row = self
            .pick_list(Kind::Persona)
            .0
            .iter()
            .find(|p| p.id == persona)?;
        Some((persona, row.avatar.as_deref()?))
    }

    /// 跟随头像的配色看谁（「主题」第 5 条）：开着会话的照会话的人格，没开的照首页画的那一位。
    pub(super) fn theme_persona(&self) -> Option<&'a Persona> {
        let place = match self.talk.transcript.session {
            Some(_) => Place::Sidebar,
            None => Place::Home,
        };
        let id = self.persona_at(place)?;
        self.pick_list(Kind::Persona).0.iter().find(|p| p.id == id)
    }

    /// 这一处是哪位人格：首页照人格框停的那一行、选的、会话的、默认的，侧边栏照开着的会话的。
    fn persona_at(&self, place: Place) -> Option<&'a str> {
        let session = self.talk.transcript.persona.as_deref();
        match place {
            Place::Home => {
                let under = match self.boxes.panel {
                    Some(Panel::Pick {
                        kind: Kind::Persona,
                        selected,
                    }) => Some(
                        self.pick_list(Kind::Persona)
                            .0
                            .get(selected)
                            .map_or("", |p| p.id.as_str()),
                    ),
                    _ => None,
                };
                home_persona(under, &self.talk.persona, session)
            }
            Place::Sidebar => sidebar_persona(session),
        }
    }

    /// 这一处画什么。终端显示不了图的、要不到的当没有头像。
    pub fn avatar(&self, place: Place) -> Avatar {
        if !self.media.figures.borrow().shows() {
            return Avatar::None;
        }
        let Some((_, version)) = self.wanted(place) else {
            return Avatar::None;
        };
        match self.media.avatars.file(version) {
            Some(path) => Avatar::File(path),
            None if self.media.avatars.refused(version) => Avatar::None,
            None => Avatar::Waiting,
        }
    }

    /// 一个会话在用量表里写什么：标题，没标题的照预览，会话列表里没有的（子代理、删掉的）写短编号（最后 8 位）。
    pub fn usage_name(&self, session: &str) -> String {
        let known = self
            .talk
            .sessions_seen
            .iter()
            .flatten()
            .find(|s| s.session == session);
        known
            .and_then(|s| s.title.clone().or_else(|| s.preview.clone()))
            .unwrap_or_else(|| {
                let tail: String = session
                    .chars()
                    .rev()
                    .take(8)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                format!("#{tail}")
            })
    }

    /// 还在等 `ui.welcomed`：问到以前、最多 `oobe.json` 的 `welcome_wait_ms`，不画首页（「第一次打开的引导」第 1 条，
    /// 2026-10-10 项目主人报：进引导前闪一下空会话）。
    pub fn welcome_pending(&self, now: Instant) -> bool {
        !(self.boxes.welcome_known && self.boxes.personas_known) && now < self.welcome_until()
    }

    /// 等 `ui.welcomed` 等到什么时候。
    pub(super) fn welcome_until(&self) -> Instant {
        self.started + Duration::from_millis(self.config.oobe.welcome_wait_ms)
    }
}
