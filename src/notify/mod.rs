//! 系统通知（蓝图 `tui.md`「系统通知」）：终端不在前台时，回答好了、出错了、在等你确认或回答，弹一条通知、响一声；
//! 在 herdr 里把在做、在等、空闲报给它。
//!
//! 弹不弹、怎么弹、响不响由 [`plan`] 定（纯函数，好测）；[`Notifier`] 记着终端在不在前台，照它做。要写给终端的转义
//! 序列攒在 [`Notifier::outbox`]，主循环画完一帧再写出去。

mod click;
pub mod herdr;
mod icon;
mod mascot_icon;
mod osc;
mod sound;
mod system;

use std::path::PathBuf;

use crate::config::{NotifyLook, NotifyTexts};
pub use click::{Clicked, Wake, clicked};
pub use herdr::State;

/// 通知从哪条路出去（第 4 条）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// kitty 的 OSC 99：点一下跳回窗口，前台不前台 kitty 自己判。
    Kitty,
    /// iTerm2、WezTerm、ghostty 认的 OSC 9。
    Osc9,
    /// 交给系统：`notify-send`、`osascript`。
    System,
}

impl Route {
    /// 照环境变量认。在 tmux、herdr 里 OSC 过不去（tmux 不转，herdr 吞掉），交给系统。
    pub fn detect(var: &impl Fn(&str) -> Option<String>, osc9_programs: &[String]) -> Self {
        let has = |name: &str| var(name).is_some_and(|v| !v.is_empty());
        if has("TMUX") || var("HERDR_ENV").as_deref() == Some("1") {
            return Route::System;
        }
        if var("TERM").as_deref() == Some("xterm-kitty") || has("KITTY_WINDOW_ID") {
            return Route::Kitty;
        }
        let program = var("TERM_PROGRAM").unwrap_or_default();
        if osc9_programs.contains(&program) {
            return Route::Osc9;
        }
        Route::System
    }
}

/// 要通知的一件事（第 1、3 条）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// 一轮回答完了。
    Replied,
    /// 这一轮出错了。
    Failed,
    /// 在等你确认。
    Approving,
    /// 在等你回答。
    Asking,
}

/// 没有头像时画哪一张：过程中的（等你确认、等你回答）抱着电脑，完成的（回答完了、出错了）站着（2026-10-19 项目主人）。
fn pose(event: Event) -> mascot_icon::Icon {
    match event {
        Event::Approving | Event::Asking => mascot_icon::Icon::Working,
        Event::Replied | Event::Failed => mascot_icon::Icon::Standing,
    }
}

/// 这一条通知是哪位人格的（「系统通知」第 3 条，2026-11-10 项目主人：通知照会话的人格来）：标题写它的名字，有头像的带头像，
/// 没头像的不带图标。无人格的会话没有这一份：标题写程序名，图标照旧用吉祥物画成的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Who {
    /// 照连接语言挑好的名字（没有的是编号）。
    pub name: String,
    /// 头像存下来的文件；没有头像、还没存下来的是 `None`。
    pub avatar: Option<PathBuf>,
}

/// 通知的图标：人格的头像、不带（有人格、没头像）、吉祥物画成的（无人格的会话、给人的通知）。
#[derive(Debug, Clone, Copy)]
enum Face<'a> {
    Avatar(&'a std::path::Path),
    Bare,
    Mascot(mascot_icon::Icon),
}

/// 这一次怎么做。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    /// 弹通知走哪条路；不弹是 `None`。
    pub route: Option<Route>,
    /// 响一声。
    pub sound: bool,
}

/// 弹不弹、怎么弹、响不响（第 2、4、5 条）。`focused` 是终端报的在不在前台，一次都没报过是 `None`（当在前台）。
/// kitty 不管在不在前台都交给它（`o=unfocused` 它自己判），别的只在报过「离开」时弹；响只在报过「离开」时，
/// 在 herdr 里仍弹桌面通知，声音由 herdr 发出。
pub fn plan(look: &NotifyLook, route: Route, focused: Option<bool>, in_herdr: bool) -> Plan {
    if !look.enabled {
        return Plan {
            route: None,
            sound: false,
        };
    }
    let away = focused == Some(false);
    Plan {
        route: (route == Route::Kitty || away).then_some(route),
        sound: look.sound && away && !in_herdr,
    }
}

/// 别的会话来的（后台的会话做完了、别的会话在问，2026-11-02 项目主人：在前台也弹，不响）：在前台也弹，响照 [`plan`]。
/// 正在看的会话在屏幕上，别的会话不在，前台不前台挡不住要看它的理由。
pub fn plan_elsewhere(
    look: &NotifyLook,
    route: Route,
    focused: Option<bool>,
    in_herdr: bool,
) -> Plan {
    Plan {
        route: look.enabled.then_some(route),
        sound: plan(look, route, focused, in_herdr).sound,
    }
}

/// 界面这一边的通知：记着终端在不在前台，照 [`plan`] 弹、响，报给 herdr。
pub struct Notifier {
    look: NotifyLook,
    texts: NotifyTexts,
    route: Route,
    focused: Option<bool>,
    herdr: Option<herdr::Herdr>,
    /// 提示音写到哪个目录；找不到缓存目录的不响。
    sounds: Option<PathBuf>,
    /// kitty 通知的编号：同一个界面的新通知顶掉旧的。
    id: String,
    outbox: Vec<String>,
    /// 欠着的通知：一轮做完了、长期目标还在跑（蓝图「系统通知」第 1 条）。
    owed: Option<Event>,
    /// 通知的图标写到哪个目录（缓存目录下 `tui/notify`）；找不到缓存目录的不带图标。
    icons: Option<PathBuf>,
    /// 这一条通知是哪个会话的：会话标题（接在正文前面）、人格（标题写它的名字，没有的写 `texts.title`；第 3 条）。
    title: Option<String>,
    who: Option<Who>,
    /// 没有头像时画成图标的吉祥物（第 3 条，2026-10-19）。
    mascot: Option<icon::Mascot>,
    /// 点了的通知要切过去的会话（第 8 条，`click.rs`）。
    clicked: Clicked,
}

impl Notifier {
    /// 照环境变量认终端和 herdr。`sounds` 是提示音写到哪个目录。
    pub fn new(
        look: NotifyLook,
        texts: NotifyTexts,
        var: impl Fn(&str) -> Option<String>,
        sounds: Option<PathBuf>,
    ) -> Self {
        let icons = sounds
            .as_deref()
            .and_then(std::path::Path::parent)
            .map(|tui| tui.join("notify"));
        let route = Route::detect(&var, &look.osc9_programs);
        Self {
            route,
            focused: None,
            herdr: herdr::Herdr::detect(&var),
            sounds,
            id: format!("miyu-{}", std::process::id()),
            outbox: Vec::new(),
            owed: None,
            icons,
            title: None,
            who: None,
            mascot: None,
            clicked: Clicked::default(),
            look,
            texts,
        }
    }

    /// 终端报了在不在前台。
    pub fn focus(&mut self, focused: bool) {
        self.focused = Some(focused);
    }

    /// 报给 herdr（不在 herdr 里什么都不做）。
    pub fn state(&mut self, state: State) {
        if let Some(herdr) = self.herdr.as_mut() {
            herdr.report(state);
        }
    }

    /// 人真退出了：在 herdr 里的退还窗格（挂断、被杀的不退，留着恢复命令）。
    pub fn quit(&mut self) {
        if let Some(herdr) = self.herdr.as_mut() {
            herdr.release();
        }
    }

    /// 主会话变了：更新 herdr 恢复命令，空会话清除旧绑定。
    pub fn session(&mut self, session: Option<&str>) {
        if let Some(herdr) = self.herdr.as_mut() {
            herdr.session(session);
        }
    }

    /// 接下来的通知是哪个会话的：会话标题、会话的人格（无人格的是 `None`：标题写程序名，图标用吉祥物画成的）。
    pub fn about(&mut self, title: Option<String>, who: Option<Who>) {
        self.title = title.filter(|t| !t.trim().is_empty());
        self.who = who;
    }

    /// 没有头像时画成图标的吉祥物：现在的样子（颜色固定照网页晨光那一套，`mascot_icon.rs`）。
    pub fn mascot(&mut self, look: crate::mascot::Look) {
        self.mascot = Some(look);
    }

    /// 出了一件要通知的事：标题是人格的名字（无人格的写程序名），正文是会话标题接为什么，图标照人格（第 3 条）。
    pub fn tell(&mut self, event: Event) {
        let body = self.reason(event);
        let (title, who) = (self.title.clone(), self.who.clone());
        self.tell_as((title, who.as_ref()), &body, pose(event), (None, false));
    }

    /// 这件事为什么（正文的后半）。
    fn reason(&self, event: Event) -> String {
        match event {
            Event::Replied => self.texts.replied.clone(),
            Event::Failed => self.texts.failed.clone(),
            Event::Approving => self.texts.approving.clone(),
            Event::Asking => self.texts.asking.clone(),
        }
    }

    /// 别的会话在问（[`plan_elsewhere`]）：同 [`Notifier::tell`]，在前台也弹。
    pub fn tell_elsewhere(&mut self, event: Event) {
        let body = self.reason(event);
        let (title, who) = (self.title.clone(), self.who.clone());
        self.tell_as((title, who.as_ref()), &body, pose(event), (None, true));
    }

    /// 给人的通知（核心 J-3 中，第 8 条）：`about` 是那个会话的标题、人格（没有会话的两样都空），照 [`Notifier::tell`] 的写法；
    /// 无人格的图标是站着的吉祥物。带了 `session` 的点了切过去。
    pub fn tell_notice(
        &mut self,
        about: (Option<String>, Option<&Who>),
        body: &str,
        session: Option<String>,
    ) {
        let pose = mascot_icon::Icon::Standing;
        self.tell_as(about, body, pose, (session, false));
    }

    /// 后台的会话做完了（[`plan_elsewhere`]）：同 [`Notifier::tell_notice`]，在前台也弹。
    pub fn tell_finished(
        &mut self,
        about: (Option<String>, Option<&Who>),
        body: &str,
        session: String,
    ) {
        let pose = mascot_icon::Icon::Standing;
        self.tell_as(about, body, pose, (Some(session), true));
    }

    /// 照会话标题、人格写好标题、正文、图标再弹：标题是人格的名字（无人格的写程序名）；会话有标题的接在正文前面
    /// （`texts.titled`）；图标有头像的用头像、有人格没头像的不带，无人格的用吉祥物画成的（2026-11-10 项目主人）。
    fn tell_as(
        &mut self,
        (title, who): (Option<String>, Option<&Who>),
        reason: &str,
        pose: mascot_icon::Icon,
        how: (Option<String>, bool),
    ) {
        let (head, body, face) = shape(&self.texts, (title.as_deref(), who), reason, pose);
        let face = match face {
            Face::Avatar(avatar) => Faced::Avatar(avatar.to_path_buf()),
            Face::Bare => Faced::Bare,
            Face::Mascot(pose) => Faced::Mascot(pose),
        };
        self.send((&head, &body), face, how);
    }

    /// 照 [`plan`] 弹、响，别的会话来的（`elsewhere`）照 [`plan_elsewhere`]。`session` 是点了切过去的会话：kitty 要它
    /// 回话，`notify-send` 带动作等着。
    fn send(
        &mut self,
        (title, body): (&str, &str),
        face: Faced,
        (session, elsewhere): (Option<String>, bool),
    ) {
        // 标题是会话标题、正文有时带模型写的字：图格字、标签字符这种外来的不进通知（发布前第五批审查 T5-3）。
        let (title, body) = (crate::tabs::printable(title), crate::tabs::printable(body));
        let (title, body) = (title.as_ref(), body.as_ref());
        let judge = if elsewhere { plan_elsewhere } else { plan };
        let plan = judge(&self.look, self.route, self.focused, self.herdr.is_some());
        let icon = || {
            let dir = self.icons.as_deref()?;
            match &face {
                Faced::Avatar(avatar) => icon::pick(Some(avatar), None, dir),
                Faced::Bare => None,
                Faced::Mascot(pose) => {
                    let mascot = self.mascot.as_ref().map(|look| (look, *pose));
                    icon::pick(None, mascot, dir)
                }
            }
        };
        match plan.route {
            Some(Route::Kitty) => {
                let picture = icon().and_then(|p| std::fs::read(p).ok());
                let clickable = session.as_deref().and_then(click::kitty_id);
                let report = clickable.is_some();
                let id = clickable.unwrap_or_else(|| self.id.clone());
                let seq = osc::kitty(
                    (&id, report, elsewhere),
                    &self.look.app,
                    (title, body),
                    picture.as_deref(),
                );
                self.outbox.push(seq);
            }
            Some(Route::Osc9) => self.outbox.push(osc::osc9(title, body)),
            Some(Route::System) => {
                let picture = icon();
                let (program, plain) =
                    system::command(&self.look.app, title, body, picture.as_deref());
                match session.filter(|_| !cfg!(target_os = "macos")) {
                    Some(session) => {
                        let mut args = click::action_args(&self.texts.open).to_vec();
                        args.extend(plain.iter().cloned());
                        let clicked = (self.clicked.clone(), session);
                        click::spawn_waiting(program, args, plain, clicked);
                    }
                    None => {
                        system::spawn(&program, &plain);
                    }
                }
            }
            None => {}
        }
        if plan.sound
            && let Some(file) = self.sounds.as_deref().and_then(sound::materialize)
        {
            sound::play(self.look.players.here(), &file);
        }
    }

    /// 点了的通知要叫醒主循环（`click.rs`）。
    pub fn set_wake(&mut self, wake: Wake) {
        self.clicked.set_wake(wake);
    }

    /// 拿走点了的通知要切过去的会话。
    pub fn take_clicked(&self) -> Vec<String> {
        self.clicked.take()
    }

    /// 先欠着这一件（长期目标还在跑，后面多半还有续轮）：接着开了一轮的 [`Notifier::forgive`]，目标停下来的
    /// [`Notifier::settle`]。
    pub fn owe(&mut self, event: Event) {
        self.owed = Some(event);
    }

    /// 欠着的不弹了（接着开了一轮、断开了）。
    pub fn forgive(&mut self) {
        self.owed = None;
    }

    /// 欠着的现在弹。
    pub fn settle(&mut self) {
        if let Some(event) = self.owed.take() {
            self.tell(event);
        }
    }

    /// 攒着的、要写给终端的转义序列：主循环画完一帧写出去。
    pub fn outbox(&mut self) -> Vec<String> {
        std::mem::take(&mut self.outbox)
    }
}

/// [`Notifier::send`] 拿到的图标：头像的文件、不带、吉祥物画成的。
enum Faced {
    Avatar(PathBuf),
    Bare,
    Mascot(mascot_icon::Icon),
}

/// 一条通知的标题、正文、图标怎么定（纯函数，好测；「系统通知」第 3 条，2026-11-10 项目主人）：标题是人格的名字，无人格的写
/// 程序名（`texts.title`）；会话标题不空的接在正文前面；图标有人格有头像的是头像，有人格没头像的不带，无人格的是吉祥物画成的。
fn shape<'a>(
    texts: &NotifyTexts,
    (title, who): (Option<&str>, Option<&'a Who>),
    reason: &str,
    pose: mascot_icon::Icon,
) -> (String, String, Face<'a>) {
    let head = who
        .map(|w| w.name.trim())
        .filter(|n| !n.is_empty())
        .map_or_else(|| texts.title.clone(), str::to_string);
    let body = match title.map(str::trim).filter(|t| !t.is_empty()) {
        Some(title) => texts
            .titled
            .replace("{title}", title)
            .replace("{body}", reason),
        None => reason.to_string(),
    };
    let face = match who {
        Some(Who {
            avatar: Some(avatar),
            ..
        }) => Face::Avatar(avatar),
        Some(_) => Face::Bare,
        None => Face::Mascot(pose),
    };
    (head, body, face)
}

#[cfg(test)]
mod tests;
