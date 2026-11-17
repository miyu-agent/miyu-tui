//! 人生卡（蓝图 `tui.md`「推演人生」第 7 条；2026-11-17 项目主人本机验收第 23 条，核心 LF-13）：看着的会话挂着人生那一行
//! （`loop` 是 `life`）时，点输入框上面的人生记号、侧边栏「循环」那一段开这张卡，和长期目标的浮窗一个位置：人格、开始、
//! 下次继续、上一轮、状态，下面两个按钮「暂停」（暂停着的是「继续」）、「结束人生」。通用的定时方法对这一行回
//! `life_schedule`，按钮只调 `life.pause`、`life.resume`、`life.kill {persona}`；「结束人生」按一下变「确认结束」、上面一行写清
//! 后果，再按才结束。画在 `ui/life_card.rs`。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Position;
use serde_json::json;
use unicode_width::UnicodeWidthStr;

use super::link::Link;
use super::talk::Talk;
use super::{App, Panel};
use crate::core::{Command, Loop, Update};
use crate::oobe::asker::TAG_BASE;

/// 暂停、继续、结束的请求编号。
const ACT_TAG: u64 = TAG_BASE - 50;

/// 第一个按钮：暂停（暂停着的是继续）。
pub const TOGGLE: usize = 0;
/// 第二个按钮：结束人生。
pub const KILL: usize = 1;

/// 看着的会话挂着的人生那一行；不是人生会话、看着子会话的是 `None`。
pub(crate) fn life_row<'a>(talk: &Talk, link: &'a Link) -> Option<&'a Loop> {
    let session = talk.transcript.session.as_deref();
    let session = session.filter(|_| talk.viewing_title().is_none());
    crate::ui::loop_tag::of(&link.loops, session)
        .into_iter()
        .find(|l| l.life.is_some())
}

/// 人生那一行是哪个人格的：行里带的，没带的（核心刚起来还没认出来）照会话的人格。
pub(crate) fn life_persona(talk: &Talk, row: &Loop) -> Option<String> {
    row.life
        .as_ref()
        .and_then(|l| l.persona.clone())
        .or_else(|| talk.transcript.persona.clone())
        .filter(|p| !p.is_empty())
}

impl App {
    fn life_row(&self) -> Option<&Loop> {
        life_row(&self.talk, &self.link)
    }

    /// 点在输入框上面的人生记号、侧边栏「循环」那一段上：开人生卡，交回 `true`。不是人生会话的不开。
    pub(super) fn life_clicked(&mut self, at: Position) -> bool {
        let areas = self.screen.areas;
        if !(areas.loop_text.contains(at) || areas.side_loops.contains(at))
            || self.life_row().is_none()
        {
            return false;
        }
        self.boxes.panel = Some(Panel::Life {
            button: TOGGLE,
            arming: false,
        });
        true
    }

    /// 卡开着时的按键：`←` `→`、`Tab` 换按钮，`Enter` 按，`Esc` 关。人生那一行没了（结束了、别处删了）的关卡。
    pub(super) fn life_card_key(&mut self, (button, arming): (usize, bool), key: KeyEvent) {
        let button = match key.code {
            KeyCode::Left | KeyCode::BackTab => TOGGLE,
            KeyCode::Right | KeyCode::Tab => KILL,
            KeyCode::Enter => return self.life_press(button, arming),
            KeyCode::Esc => {
                self.boxes.panel = None;
                return;
            }
            _ => button,
        };
        // 挪开了「确认结束」就不算按过一下：回到「结束人生」。
        let arming = arming && button == KILL;
        self.boxes.panel = Some(Panel::Life { button, arming });
    }

    /// 按下一个按钮：暂停、继续当场发；结束要按两下，第二下才发。
    pub(super) fn life_press(&mut self, button: usize, arming: bool) {
        let Some(row) = self.life_row() else {
            self.boxes.panel = None;
            return;
        };
        let paused = row.paused;
        let Some(persona) = life_persona(&self.talk, row) else {
            return;
        };
        let method = match (button, arming) {
            (KILL, false) => {
                self.boxes.panel = Some(Panel::Life {
                    button,
                    arming: true,
                });
                return;
            }
            (KILL, true) => "life.kill",
            _ if paused => "life.resume",
            _ => "life.pause",
        };
        if button == KILL {
            self.boxes.panel = None;
        }
        self.link.core.send(Command::Ask {
            tag: ACT_TAG,
            method,
            params: json!({ "persona": persona }),
        });
    }

    /// 点在卡上：点中按钮那一行的哪个按钮就按它（`[ 字 ]`，隔两列，同 `prefix_ask.rs`）。交回点没点在卡上。
    pub(super) fn life_card_click(&mut self, at: Position, press: bool) -> bool {
        let Some(Panel::Life { arming, .. }) = self.boxes.panel else {
            return false;
        };
        let areas = self.screen.areas;
        if !areas.menu.contains(at) {
            return false;
        }
        let row = at.y.checked_sub(areas.menu_text.y).map(usize::from);
        let on_buttons = row.and_then(|row| self.screen.panel_rows.get(row).copied().flatten());
        if on_buttons.is_none() || !press {
            return true;
        }
        let paused = self.life_row().is_some_and(|r| r.paused);
        let mut x = areas.menu_text.x + 2;
        for (i, label) in crate::ui::life_card::labels(&self.config, paused, arming)
            .iter()
            .enumerate()
        {
            let w = u16::try_from(format!("[ {label} ]").width()).unwrap_or(0);
            if (x..x + w).contains(&at.x) {
                self.life_press(i, arming && i == KILL);
                break;
            }
            x += w + 2;
        }
        true
    }

    /// 暂停、继续、结束的回应：拒了的弹核心的原话；成了的照推来的定时表换（卡上的状态跟着变，结束了的那一行删掉、卡关上）。
    /// 回应归这里交回 `true`。
    pub(super) fn life_card_answer(&mut self, update: &Update) -> bool {
        let Update::Answer { tag, result } = update else {
            return false;
        };
        if *tag != ACT_TAG {
            return false;
        }
        if let Err(refused) = result {
            self.hint(refused.message.clone(), false);
        }
        true
    }
}
