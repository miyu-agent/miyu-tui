//! 吉祥物要界面这一头办的（蓝图 `tui.md`「空会话的首页」第 6–9 条）：什么时候醒来画下一帧。

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ratatui::crossterm::event::{Event, MouseEventKind};

use super::App;
use crate::mascot::{Attention, Gaze, Idle, Mouth, Perch};
use crate::pulse::Pulse;

/// 吉祥物和运行状态行会动的那几样（蓝图 `app.md` 的 `mascot` 块）：画之前推一步，按键、动鼠标时记一笔。
pub struct MascotState {
    /// 运行状态行正在写的词。
    pub pulse: Pulse,
    /// 首页的吉祥物朝哪看（`tui.md`「空会话的首页」第 6、7 条）。
    pub gaze: Gaze,
    /// 首页吉祥物的待机小动作。
    pub idle: Idle,
    /// 首页吉祥物看鼠标还是看输入光标：最近一次动鼠标、按键的时刻。
    pub attention: Attention,
    /// 首页吉祥物被列表顶上去以后待在哪。
    pub perch: Perch,
    /// 吉祥物的嘴（`tui.md`「空会话的首页」第 9 条）。
    pub mouth: Mouth,
}

impl MascotState {
    /// 起头的随机数照启动的时刻定，每次启动不一样。
    pub fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);
        Self {
            pulse: Pulse::new(nanos),
            gaze: Gaze::default(),
            idle: Idle::new(nanos),
            attention: Attention::default(),
            perch: Perch::default(),
            mouth: Mouth::new(nanos >> 5),
        }
    }

    /// 一个终端事件：有人按键、动鼠标、粘贴，停下待机的晃（「空会话的首页」第 8 条）；记下最近一次动鼠标、按键的时刻，
    /// 看鼠标还是看输入光标照它（第 6 条）。
    pub fn heard(&mut self, event: &Event, now: Instant) {
        if matches!(event, Event::Key(_) | Event::Mouse(_) | Event::Paste(_)) {
            self.idle.poke(now);
        }
        match event {
            Event::Key(_) | Event::Paste(_) => self.attention.typed(now),
            Event::Mouse(m)
                if matches!(m.kind, MouseEventKind::Moved | MouseEventKind::Drag(_)) =>
            {
                self.attention.pointed(now);
            }
            _ => {}
        }
    }
}

impl App {
    /// 吉祥物画着的时候，下一次该画的时刻：转头、待机小动作、鼠标停够了转回来、走下来、嘴在动。头像替掉吉祥物的只照
    /// 走下来（「空会话的首页」第 10 条）。
    pub(super) fn mascot_deadline(&self, now: Instant) -> Vec<Instant> {
        let look = &self.config.mascot;
        let walking = self
            .home_slot_shown()
            .then(|| self.mascot.perch.wake(now, &look.perch))
            .flatten();
        if !self.mascot_shown() {
            return walking.into_iter().collect();
        }
        // 转头：照它的节拍画，转到了就停（第 7 条）。
        let turning = self
            .mascot
            .gaze
            .moving()
            .then(|| now + Duration::from_millis(look.gaze.frame_ms));
        // 框里有字、鼠标停够了：到点画一帧，转回来看输入光标（第 6 条）。
        let settle = Duration::from_millis(look.gaze.pointer_settle_ms);
        let settling = (!self.typing.input.editor.is_empty())
            .then(|| self.mascot.attention.settles_at(settle))
            .flatten()
            .filter(|at| *at > now);
        [
            turning,
            self.mascot.idle.wake(now, &look.idle),
            settling,
            walking,
            self.mascot.mouth.wake(now, &look.mouth),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}
