//! 她说的语音在 shell 里也当场放（2026-11-18 项目主人定，图纸 `shellhook.md`「怎么印」第 3c 条）：同全屏（`app/voice/clips.rs`），
//! 经总线 `tts.play {"audio"}` 从核心那台机器的音箱出来，请求、放不了的判断都用 `crate::voice::clips`。放不放照
//! `transcript/voice.rs`：这一句开的那一轮、没人开口的那一轮（回报叫醒的）当场放，别处开的只画，补发的不放。shell 里点不了，
//! 不停、不重放；放着的时候语音条打头是 `■`，定下来印进滚动区的照没在放画。

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use super::{Incoming, Turn};
use crate::config::Config;
use crate::core::{Core, Update};
use crate::ui::voice_rows::Speaker;
use crate::voice::clips::Tags;

/// 放语音的 `tts.play`、`tts.stop` 认它（同一条连接上别的请求是 1～6）。
const TAGS: Tags = Tags { play: 7, stop: 8 };
/// 收工前，刚发出去的放最少再连这么久：连接一断，核心丢掉这条连接还在后台办的请求，还没转给语音包的就放不出来了。
const GRACE: Duration = Duration::from_millis(500);

/// 这个 shell 放声音的情况。
#[derive(Debug, Default)]
pub struct Clips {
    /// 总线上可用的名字、在放哪一段。
    voice: crate::voice::Voice,
    /// 最后一次发出去放的时刻。
    sent: Option<Instant>,
}

impl Turn {
    /// 语音的那几件（总线上可用的名字、放的回应）归这里，交回 `true`。放出错的不说：shell 里没有地方提示，语音条照旧画。
    pub(super) fn clip_update(&mut self, update: &Update) -> bool {
        match update {
            Update::Bus(names) => self.clips.voice.ready.clone_from(names),
            Update::Answer { tag, .. } if *tag == TAGS.play => self.clips.voice.clip_answered(),
            Update::Answer { tag, .. } if *tag == TAGS.stop => {}
            _ => return false,
        }
        true
    }

    /// 正文收完一件：刚来、该放的那一段当场放。
    pub(super) fn clip_arrived(&mut self, core: &Core) {
        let Some((clip, _)) = self.transcript.take_play() else {
            return;
        };
        for call in self.clips.voice.play_clip(&clip, TAGS) {
            self.clips.sent = Some(Instant::now());
            core.send(call);
        }
    }

    /// 画语音条用的。
    pub fn speaker(&self) -> Speaker<'_> {
        self.clips.voice.speaker()
    }

    /// 收工前：刚发出去的放还没连满 [`GRACE`] 的，接着收核心的、留着敲的字等够；之后不再管在放的（定下来的语音条照没在放画）。
    pub fn settle_clips(
        &mut self,
        incoming: &Receiver<Incoming>,
        config: &mut Config,
        core: &Core,
    ) {
        while let Some(left) = self.clip_grace() {
            match incoming.recv_timeout(left) {
                Ok(Incoming::Core(update)) => self.core(update, config, core),
                Ok(Incoming::Key(event)) => self.keep_typed(&event, config),
                Err(_) => break,
            }
        }
        self.clips.voice.clip = None;
    }

    /// 还要再连多久：没有在放的、发出去够久了的是 `None`。
    fn clip_grace(&self) -> Option<Duration> {
        self.clips.voice.clip.as_ref()?;
        let left = GRACE.saturating_sub(self.clips.sent?.elapsed());
        (!left.is_zero()).then_some(left)
    }
}
