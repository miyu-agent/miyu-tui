//! 新会话先选人格、再选预设（蓝图 `tui.md`「新会话：人格、工作区」第 1 到 3 条；2026-10-07 项目主人定每次开新会话
//! 都先弹框选，2026-10-08 定先人格、再预设，两个分开的框）：起来进空会话、`/new` 以后，在输入框上面开框，选了才能
//! 打字；光标停在上一次选的（最近那个会话用的，2026-10-09 项目主人：「照旧弹框，光标停在上一次的」）上，没有的停在默认
//! 的上，`Enter` 选，`Esc` 照光标起初停的那一个（用不了时不关、弹一句：开会话一定要有预设，核心 Y12）。
//! 选的记着，开会话时带上；开了会话不能换。测具设了 `MIYU_TUI_PERSONA`、`MIYU_TUI_PRESET` 的照它、不弹框（人格写 `-` 是
//! 无人格）。

use ratatui::crossterm::event::KeyEvent;

use super::{App, Panel};
use crate::core::{Command, Persona};
use crate::pick_box::{Pressed, usable};
use crate::transcript::Link;

/// 新会话要选的哪一样。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 人格：她是谁。
    Persona,
    /// 预设：这次怎么工作。
    Preset,
}

/// 照这个先后问（2026-10-08 项目主人定先人格、再预设）。
const ORDER: [Kind; 2] = [Kind::Persona, Kind::Preset];

/// 新会话开头必须选的那一样（第 1、3 条；2026-10-21 项目主人：「不是以列表的形式出，而是直接替换掉输入框」，列表读回来
/// 以前那个位置写「正在加载…」，这时打不了字）：还没开会话，照先后第一样还没选、测具没设、核心不是不认（列表不是空的）。
/// 都选了、开了会话的是 `None`。
pub fn start_pick(talk: &super::talk::Talk) -> Option<Kind> {
    if talk.transcript.session.is_some() {
        return None;
    }
    ORDER.into_iter().find(|kind| {
        let pick = match kind {
            Kind::Persona => &talk.persona,
            Kind::Preset => &talk.preset,
        };
        pick.chosen.is_none()
            && pick.env.is_none()
            && pick.list.as_ref().is_none_or(|list| !list.is_empty())
    })
}

/// 一样要选的：读来的一行行（人格和预设一个形状）、默认的、这个新会话选了的、测具设的。
#[derive(Debug, Default)]
pub struct Pick {
    /// 核心交回的；还没读到的是 `None`，读不了（旧核心）的是空的。
    pub list: Option<Vec<Persona>>,
    /// 默认的编号（`persona.default`、`preset.default`）。
    pub default: Option<String>,
    /// 默认的这一回从核心读到过（`picks_configured`）：不问人格预设时（`tui.skip_pick`）等它到了才照它选。
    pub default_known: bool,
    /// 这个新会话选了的；还没选的是 `None`。
    pub chosen: Option<String>,
    /// 测具设的（空的不算）。
    pub env: Option<String>,
    /// 框开着时光标起初停的那一行：会话列表晚到了，人还没挪过的照它改停的地方。
    pub opened: Option<usize>,
    /// 最近那个会话用的（`Some(None)` 是它没带）：起来时先照记下的（`look.rs`），会话列表到了照它换。会话列表还没到时
    /// 首页照它画、框照它停，不先闪默认人格的头像（2026-10-19 项目主人报：无人格也闪一帧头像）。
    pub last: Option<Option<String>>,
}

impl Pick {
    /// 照环境变量 `name` 起一份（测具设了的照它、不弹框）。
    pub fn from_env(name: &str) -> Self {
        Self {
            // `-` 是明着不带（「无人格」，编号空着）。
            env: std::env::var(name)
                .ok()
                .filter(|v| !v.is_empty())
                .map(|v| if v == "-" { String::new() } else { v }),
            ..Self::default()
        }
    }

    /// 默认的在第几行；没有、用不了的是 `None`。
    fn default_index(&self) -> Option<usize> {
        let default = self.default.as_deref()?;
        let list = self.list.as_ref()?;
        list.iter()
            .position(|p| p.id == default && p.problem.is_none())
    }

    /// 框起初停在第几行：上一次选的（`last`：最近那个会话用的，`Some(None)` 是它没带）列表里有、用得了的停它，不然停
    /// 默认的，再不然停第一个用得了的。人格框第一行「无人格」的编号是空的，没带人格的停它。
    pub(super) fn start(&self, last: Option<Option<&str>>) -> usize {
        let list = self.list.as_deref().unwrap_or_default();
        self.preferred(last)
            .or_else(|| usable(list).next())
            .unwrap_or(0)
    }

    /// 上一次选的、默认的，用得了的那一行；都没有的是 `None`（`Esc` 照它：预设都用不了时不关框，核心 Y12）。
    fn preferred(&self, last: Option<Option<&str>>) -> Option<usize> {
        let list = self.list.as_deref().unwrap_or_default();
        let found = |id: &str| list.iter().position(|p| p.id == id && p.problem.is_none());
        last.and_then(|id| found(id.unwrap_or_default()))
            .or_else(|| self.default_index())
    }

    /// 编号写成名字（照列表，没有的写编号）。
    pub(crate) fn name<'a>(&'a self, id: &'a str) -> &'a str {
        self.list
            .iter()
            .flatten()
            .find(|p| p.id == id)
            .map_or(id, |p| p.label())
    }
}

impl App {
    fn pick(&self, kind: Kind) -> &Pick {
        match kind {
            Kind::Persona => &self.talk.persona,
            Kind::Preset => &self.talk.preset,
        }
    }

    fn pick_mut(&mut self, kind: Kind) -> &mut Pick {
        match kind {
            Kind::Persona => &mut self.talk.persona,
            Kind::Preset => &mut self.talk.preset,
        }
    }

    /// 核心交回了人格、预设：记下，该问的时候开框。人格最前面加一行「无人格」（编号空着，开会话时发 `"persona": null`，
    /// 2026-10-08 项目主人：人格允许为空）；核心不认人格的（列表是空的）不加、不问。
    pub(super) fn listed(&mut self, kind: Kind, mut list: Vec<Persona>) {
        if kind == Kind::Persona && !list.is_empty() {
            let texts = &self.config.text.persona_box;
            list.insert(
                0,
                Persona {
                    id: String::new(),
                    name: Some(texts.none.clone()),
                    summary: Some(texts.none_note.clone()),
                    problem: None,
                    avatar: None,
                    seed: None,
                    seeds: None,
                },
            );
        }
        // 正文里「已换成」照名字写（`transcript/switched.rs`）。
        let names = list
            .iter()
            .filter(|p| !p.id.is_empty())
            .map(|p| (p.id.clone(), p.label().to_string()))
            .collect();
        crate::transcript::set_names(kind == Kind::Persona, names);
        self.pick_mut(kind).list = Some(list);
        self.refit_pick(kind);
        self.ask_next();
    }

    /// 框照记下的列表开着、核心的到了：人还没挪过的照上一次选的重新停，挪过的留在原处（新列表短了的停最后一行）。
    fn refit_pick(&mut self, kind: Kind) {
        let Some(Panel::Pick {
            kind: open,
            selected,
        }) = self.boxes.panel
        else {
            return;
        };
        if open != kind {
            return;
        }
        let selected = if self.pick(kind).opened == Some(selected) {
            let start = self.pick(kind).start(self.last_pick(kind));
            self.pick_mut(kind).opened = Some(start);
            start
        } else {
            let len = self.pick(kind).list.as_ref().map_or(0, Vec::len);
            selected.min(len.saturating_sub(1))
        };
        self.boxes.panel = Some(Panel::Pick { kind, selected });
    }

    /// 该问的时候开框：还没开会话、没开着别的框；照先后，没选的先问。测具设了的直接用；核心不认的（列表是空的）
    /// 不问；还没读到的等读到了再问。还没连上也开（照记下的列表，`look.rs`），只要记着走过引导：第一帧就是人格框，选的
    /// 排进命令里，连上再发（2026-10-22 项目主人：「依旧不是秒开」，照推荐定连上以前就开）。
    pub(super) fn ask_next(&mut self) {
        let offline = self.talk.transcript.link != Link::Ready;
        if self.talk.transcript.session.is_some()
            || (offline && !self.boxes.welcome_known)
            || self.boxes.panel.is_some()
        {
            return;
        }
        for kind in ORDER {
            let pick = self.pick(kind);
            if pick.chosen.is_some() {
                continue;
            }
            if let Some(id) = pick.env.clone() {
                self.choose(kind, id);
                continue;
            }
            let Some(list) = pick.list.as_ref() else {
                return;
            };
            if list.is_empty() {
                continue;
            }
            // 不问人格预设（第 1a 条）：等默认的读到，照它选；默认的预设用不了的照旧开框（开会话一定要有预设，核心 Y12）。
            if self.config.layout.skip_pick {
                if !pick.default_known {
                    return;
                }
                if let Some(id) = default_choice(list, pick.default_index(), kind) {
                    self.choose(kind, id);
                    continue;
                }
            }
            let selected = pick.start(self.last_pick(kind));
            self.pick_mut(kind).opened = Some(selected);
            self.boxes.panel = Some(Panel::Pick { kind, selected });
            return;
        }
    }

    /// 新会话开头要选、列表还没读回来（框还没开）：输入框的位置写「正在加载…」，按键、粘贴不进输入框。
    pub(super) fn start_loading(&self) -> bool {
        start_pick(&self.talk).is_some() && !matches!(self.boxes.panel, Some(Panel::Pick { .. }))
    }

    /// 上一次选的：最近那个会话用的（`Some(None)` 是它没带）；会话列表还没到、一个会话都没有的是 `None`。
    fn last_pick(&self, kind: Kind) -> Option<Option<&str>> {
        let Some(seen) = self.talk.sessions_seen.as_deref() else {
            // 会话列表还没到：照起来时记下的。
            return self.pick(kind).last.as_ref().map(Option::as_deref);
        };
        let latest = crate::core::latest_session(seen)?;
        Some(match kind {
            Kind::Persona => latest.persona.as_deref(),
            Kind::Preset => latest.preset.as_deref(),
        })
    }

    /// 会话列表到了、变了：框开着、人还没挪过光标的，照上一次选的重新停。
    pub(super) fn repoint_pick(&mut self) {
        let Some(Panel::Pick { kind, selected }) = self.boxes.panel else {
            return;
        };
        if self.pick(kind).opened != Some(selected) {
            return;
        }
        let selected = self.pick(kind).start(self.last_pick(kind));
        self.pick_mut(kind).opened = Some(selected);
        self.boxes.panel = Some(Panel::Pick { kind, selected });
    }

    /// 框开着时的按键（`pick_box.rs`）：上下挪（跳过写错的），`Enter` 选，`e` 编辑（第 2b 条），`Esc` 照光标起初停的
    /// 那一个。别的键不管：选了才能打字。
    pub(super) fn pick_key(&mut self, kind: Kind, selected: usize, key: KeyEvent) {
        let Some(list) = self.pick(kind).list.clone() else {
            self.boxes.panel = None;
            return;
        };
        match crate::pick_box::press(&list, selected, key) {
            Pressed::Moved(selected) => self.boxes.panel = Some(Panel::Pick { kind, selected }),
            Pressed::Chose(at) => {
                let id = list[at].id.clone();
                self.boxes.panel = None;
                match self.current_pick(kind) {
                    // 开着的会话中途换：选的就是现在的不发，换成了照推来的办（第 2a 条）。
                    Some(current) => {
                        if current != id {
                            self.switch_pick(kind, id);
                        }
                    }
                    None => {
                        self.choose(kind, id);
                        self.ask_next();
                    }
                }
            }
            Pressed::Edit(at) => self.edit_pick(kind, selected, list[at].id.clone()),
            Pressed::CantEdit => {
                let note = self.config.text.persona_box.cant_edit.clone();
                self.hint(note, false);
            }
            // 开着的会话中途换、还没开会话但已经选过再打开的：`Esc` 关、不换（2026-10-20 查出：原来照默认的，选好的被换掉）。
            Pressed::Escape if !self.not_opened() || self.pick(kind).chosen.is_some() => {
                self.boxes.panel = None;
            }
            // 新会话开头（2026-10-23 项目主人：「esc 不应该是选择默认，而应该是返回。不然我人格选错了就没法返回了」）：预设框
            // 回人格框重选，光标停在刚选的那个；人格框是第一步，`Esc` 不动。
            Pressed::Escape => {
                // 没有人格框可回（测具设了人格、核心不认人格）：弹一句先选预设（开会话一定要有预设，核心 Y12）。
                if kind == Kind::Preset && !self.back_to_persona() {
                    let note = self.config.text.persona_box.preset_need.clone();
                    self.hint(note, false);
                }
            }
            Pressed::Ignored => {}
        }
    }

    /// 开着的会话中途换：交给核心（`session.configure`）。
    fn switch_pick(&mut self, kind: Kind, id: String) {
        let command = match kind {
            Kind::Persona => Command::Persona(id),
            Kind::Preset => Command::Preset(id),
        };
        self.link.core.send(command);
    }

    /// 新会话开头从预设框回人格框：人格的选择清掉重选，光标停在刚选的那个；测具设了人格的、核心不认人格的（列表是空的）
    /// 没有人格框可回，交回 `false`。
    fn back_to_persona(&mut self) -> bool {
        let pick = &self.talk.persona;
        if pick.env.is_some() {
            return false;
        }
        let Some(list) = pick.list.as_ref().filter(|l| !l.is_empty()) else {
            return false;
        };
        let selected = pick
            .chosen
            .as_deref()
            .and_then(|id| list.iter().position(|p| p.id == id))
            .unwrap_or_else(|| pick.start(self.last_pick(Kind::Persona)));
        self.talk.persona.chosen = None;
        self.talk.persona.opened = Some(selected);
        self.boxes.panel = Some(Panel::Pick {
            kind: Kind::Persona,
            selected,
        });
        true
    }

    /// 选了：记着，告诉核心开会话时带上。
    fn choose(&mut self, kind: Kind, id: String) {
        let command = match kind {
            Kind::Persona => Command::Persona(id.clone()),
            Kind::Preset => Command::Preset(id.clone()),
        };
        self.link.core.send(command);
        self.pick_mut(kind).chosen = Some(id);
    }

    /// 第一次引导刚走完：还没开会话的照引导里写成默认的人格、预设开这一个会话，开着的框收掉，不再问（蓝图「第一次
    /// 打开的引导」第 11 条，2026-10-09 项目主人定）。已经开了会话的不动；`/new` 以后照常问。
    pub(super) fn adopt_picks(&mut self, persona: Option<String>, preset: Option<String>) {
        if !self.not_opened() {
            return;
        }
        if matches!(self.boxes.panel, Some(Panel::Pick { .. })) {
            self.boxes.panel = None;
        }
        for (kind, id) in [(Kind::Persona, persona), (Kind::Preset, preset)] {
            if let Some(id) = id {
                self.choose(kind, id);
            }
        }
        self.ask_next();
    }

    /// `/persona`、`/preset`：还没开会话的再开框换；开了的中途换（「新会话：人格、工作区」第 2a 条，核心 P-13），在回答时
    /// 不开、弹一句先打断或者等它做完。
    pub(super) fn pick_command(&mut self, kind: Kind) {
        let opened = !self.not_opened();
        if opened && self.talk.transcript.running.is_some() {
            let note = self.config.text.persona_box.busy.clone();
            self.hint(note, false);
            return;
        }
        let current = self.current_pick(kind);
        let pick = self.pick(kind);
        let Some(list) = pick.list.as_ref().filter(|l| !l.is_empty()) else {
            return;
        };
        let wanted = if opened { current } else { pick.chosen.clone() };
        let at = wanted
            .as_deref()
            .and_then(|id| list.iter().position(|p| p.id == id))
            .or_else(|| pick.default_index())
            .unwrap_or(0);
        self.boxes.panel = Some(Panel::Pick { kind, selected: at });
    }

    /// 开着的会话现在用的那一个：无人格的是空的编号（列表第一行）；还没开会话的是 `None`。
    pub(super) fn current_pick(&self, kind: Kind) -> Option<String> {
        if self.not_opened() {
            return None;
        }
        let t = &self.talk.transcript;
        Some(match kind {
            Kind::Persona => t.persona.clone().unwrap_or_default(),
            Kind::Preset => t.preset.clone().unwrap_or_default(),
        })
    }

    /// 新会话：选的都清掉，再问（`/new`）。
    pub(super) fn forget_picks(&mut self) {
        self.talk.persona.chosen = None;
        self.talk.preset.chosen = None;
        self.ask_next();
        self.link.core.send(Command::ListPersonas);
        self.link.core.send(Command::ListPresets);
    }

    /// 「新会话不再询问人格预设」读到了（连上、配置变了）：开着的是这个还没开的会话的框、人还没挪过光标的，收掉照
    /// 默认的选（第 1a 条：设置读回来比人格列表晚时框已经开了）。
    pub(super) fn skip_pick_read(&mut self) {
        if !self.config.layout.skip_pick || !self.not_opened() {
            return;
        }
        if let Some(Panel::Pick { kind, selected }) = self.boxes.panel
            && self.pick(kind).chosen.is_none()
            && self.pick(kind).opened == Some(selected)
        {
            self.boxes.panel = None;
        }
        self.ask_next();
    }

    /// 连上、配置变了：默认的照配置，两样都再读一次。手里有上次记下的列表的，先照它开框（`look.rs`）：核心冷启动时列人格
    /// 要近一秒（2026-10-22 项目主人：进界面到人格框出来慢），新的到了照它换。
    pub(super) fn picks_configured(&mut self, persona: Option<String>, preset: Option<String>) {
        self.talk.persona.default = persona;
        self.talk.preset.default = preset;
        self.talk.persona.default_known = true;
        self.talk.preset.default_known = true;
        self.ask_next();
        self.link.core.send(Command::ListPersonas);
        self.link.core.send(Command::ListPresets);
    }
}

/// 不问人格预设时照哪一个（编号）：默认的那一行；没有、用不了的人格是第一行无人格，预设一定要有（核心 Y12），没有能用的
/// 交回 `None`，照旧开框。
fn default_choice(list: &[Persona], default: Option<usize>, kind: Kind) -> Option<String> {
    match default.and_then(|i| list.get(i)) {
        Some(p) => Some(p.id.clone()),
        None => {
            (kind == Kind::Persona).then(|| list.first().map_or_else(String::new, |p| p.id.clone()))
        }
    }
}

#[cfg(test)]
mod skip_tests;

#[cfg(test)]
mod tests {
    use super::Pick;
    use crate::core::Persona;

    fn row(id: &str, problem: Option<&str>) -> Persona {
        Persona {
            id: id.into(),
            name: None,
            summary: None,
            problem: problem.map(str::to_string),
            avatar: None,
            seed: None,
            seeds: None,
        }
    }

    #[test]
    fn the_box_starts_on_the_last_pick_then_the_default() {
        // 2026-10-09 项目主人：「照旧弹框，光标停在上一次的」。
        let pick = Pick {
            list: Some(vec![
                row("", None),
                row("miyu", None),
                row("broken", Some("写错了")),
                row("engineer", None),
            ]),
            default: Some("miyu".into()),
            ..Pick::default()
        };
        assert_eq!(pick.start(Some(Some("engineer"))), 3, "上一次选的");
        assert_eq!(pick.start(Some(None)), 0, "上一次没带人格：无人格");
        assert_eq!(
            pick.start(Some(Some("broken"))),
            1,
            "上一次的用不了：默认的"
        );
        assert_eq!(pick.start(Some(Some("gone"))), 1, "上一次的没了：默认的");
        assert_eq!(pick.start(None), 1, "没有会话：默认的");
    }
}
