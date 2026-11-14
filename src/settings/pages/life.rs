//! 人格的窗里的「推演人生」（蓝图 `tui.md`「推演人生」第 2 条；核心 LF-5、LF-8，设计 70 第三节第 9 条，2026-11-14 项目
//! 主人定终端的做法）：一行开关照 `life.status` 写；关着的回车先问一句（写清影响）再 `life.start`，开着的 `life.pause`、
//! 暂停着的 `life.resume`，都不问。她写过「现在在干嘛」的下面暗色一行。

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::Deserialize;
use serde_json::{Value, json};

use super::choose::PersonaView;
use super::target::said;
use crate::core::Refusal;
use crate::settings::popup::{Confirm, Delete, Popup};
use crate::settings::{Settings, Texts, Tone, Waiting};

/// 推演人生的字（`text/zh.json` 的 `settings.more.persona_edit.life`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifeTexts {
    /// 那一行的名字。
    pub name: String,
    /// 在跑、暂停着、没开（没有、结束了）。
    pub states: [String; 3],
    /// 在跑的后面暗色接的：`{time}` 是下次醒来的时刻。
    pub next: String,
    /// 开之前问的：标题、三行影响、开启、取消。
    pub confirm: [String; 6],
    /// 推演方法那一行的名字。
    pub method: String,
    /// 人生会话上下文上限那一行的名字（也是编辑窗的标题）。
    pub context: String,
    /// 没写时暗色写的：`{n}` 是出厂的那个数。
    pub context_default: String,
}

/// 一个人格的人生会话在哪一步（`life.status` 的回应）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LifeState {
    /// `none`、`running`、`paused`、`ended`。
    pub state: String,
    /// 下次醒来的时刻；在跑的才有。
    pub next: Option<Timestamp>,
    /// 她最近写下的「现在在干嘛」。
    pub doing: Option<String>,
}

impl LifeState {
    /// 读 `life.status`、`life.pause`、`life.resume` 的回应。
    pub fn read(got: &Value) -> Self {
        Self {
            state: got["state"].as_str().unwrap_or("none").to_string(),
            next: got["next"].as_str().and_then(|t| t.parse().ok()),
            doing: got["doing"]["text"]
                .as_str()
                .filter(|t| !t.trim().is_empty())
                .map(str::to_string),
        }
    }

    /// 那一行的值和暗色接在后面的：在跑「开」接「下次醒来 13:07」（今天的只写时刻，别的天写「11/15 13:07」），暂停着
    /// 「暂停」，别的「关」。
    pub fn shown(&self, texts: &LifeTexts, now: Timestamp, tz: &TimeZone) -> (String, String) {
        match self.state.as_str() {
            "running" => {
                let tail = self
                    .next
                    .map(|next| texts.next.replace("{time}", &when(next, now, tz)))
                    .unwrap_or_default();
                (texts.states[0].clone(), tail)
            }
            "paused" => (texts.states[1].clone(), String::new()),
            _ => (texts.states[2].clone(), String::new()),
        }
    }
}

/// 一个时刻照本地写：今天的「13:07」，别的天「11/15 13:07」。
pub fn when(at: Timestamp, now: Timestamp, tz: &TimeZone) -> String {
    let (at, now) = (at.to_zoned(tz.clone()), now.to_zoned(tz.clone()));
    if at.date() == now.date() {
        at.strftime("%H:%M").to_string()
    } else {
        at.strftime("%m/%d %H:%M").to_string()
    }
}

impl Settings {
    /// 问这个人格的人生会话在哪一步：开人格的窗时、开开关关以后。
    pub(in crate::settings) fn ask_life(&mut self, persona: &str) {
        let waiting = Waiting::Life(persona.to_string());
        self.ask("life.status", json!({"persona": persona}), waiting);
    }

    /// 读到了（`life.status`、`life.pause`、`life.resume` 都回这一样）；被拒的写原话。
    pub(in crate::settings) fn life_answer(
        &mut self,
        persona: String,
        result: Result<Value, Refusal>,
    ) {
        match result {
            Ok(got) => self.more.life = Some((persona, LifeState::read(&got))),
            Err(refusal) => self.say(said(&refusal), Tone::Bad),
        }
    }

    /// 这个人格的推演人生开着（在跑；关就是暂停）：窗里露出推演方法（[`super::persona_edit::fields`]）。
    pub fn life_on(&self, persona: &str) -> bool {
        self.life_of(persona).is_some_and(|s| s.state == "running")
    }

    /// 这个人格现在的那一份；还没读到的是 `None`。
    pub fn life_of(&self, persona: &str) -> Option<&LifeState> {
        self.more
            .life
            .as_ref()
            .filter(|(id, _)| id == persona)
            .map(|(_, state)| state)
    }

    /// 那一行回车：在跑的暂停、暂停着的继续，不问；没开的先问一句（[`Settings::life_decided`]）。还没读到的不动。
    pub(in crate::settings) fn toggle_life(&mut self, view: PersonaView, texts: &Texts) {
        let method = match self.life_of(&view.id).map(|s| s.state.as_str()) {
            Some("running") => "life.pause",
            Some("paused") => "life.resume",
            Some(_) => {
                let words = &texts.more.persona_edit.life.confirm;
                self.popup = Some(Popup::Confirm(Confirm {
                    title: words[0].clone(),
                    lines: words[1..4].iter().map(|l| (l.clone(), false)).collect(),
                    buttons: vec![(words[4].clone(), false), (words[5].clone(), false)],
                    sel: 1,
                    ask: Delete::Life(Box::new(view)),
                }));
                return;
            }
            None => {
                self.popup = Some(Popup::Persona(view));
                return;
            }
        };
        let waiting = Waiting::Life(view.id.clone());
        self.ask(method, json!({"persona": view.id}), waiting);
        self.popup = Some(Popup::Persona(view));
    }

    /// 问过了：选了「开启」的 `life.start`；都回到人格的窗。
    pub(in crate::settings) fn life_decided(&mut self, view: &PersonaView, yes: bool) {
        if yes {
            let waiting = Waiting::LifeStarted(view.id.clone());
            self.ask("life.start", json!({"persona": view.id}), waiting);
        }
        self.popup = Some(Popup::Persona(view.clone()));
    }

    /// `life.start` 回来了：成了的重问一次状态（回应只有会话编号），被拒的写原话。
    pub(in crate::settings) fn life_started(
        &mut self,
        persona: &str,
        result: Result<Value, Refusal>,
    ) {
        match result {
            Ok(_) => self.ask_life(persona),
            Err(refusal) => self.say(said(&refusal), Tone::Bad),
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::tz::TimeZone;
    use serde_json::json;

    use super::{LifeState, when};

    #[test]
    fn the_status_reads_as_the_core_sends_it() {
        let got = LifeState::read(&json!({
            "state": "running", "session": "s1", "next": "2026-11-14T10:00:00Z",
            "doing": {"text": "在看书", "at": "2026-11-14T09:00:00Z"}
        }));
        assert_eq!(got.state, "running");
        assert_eq!(got.next, Some("2026-11-14T10:00:00Z".parse().unwrap()));
        assert_eq!(got.doing.as_deref(), Some("在看书"));
        let none = LifeState::read(&json!({"state": "none"}));
        assert_eq!((none.next, none.doing), (None, None));
    }

    #[test]
    fn today_writes_only_the_time_and_other_days_the_date_too() {
        let tz = TimeZone::UTC;
        let now = "2026-11-14T09:00:00Z".parse().unwrap();
        assert_eq!(
            when("2026-11-14T13:07:00Z".parse().unwrap(), now, &tz),
            "13:07"
        );
        assert_eq!(
            when("2026-11-15T01:30:00Z".parse().unwrap(), now, &tz),
            "11/15 01:30"
        );
    }
}
