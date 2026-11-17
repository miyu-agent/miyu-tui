//! 她说出来的语音（核心 SD-1，蓝图 `tui.md`「她说的语音」）：一段一条语音条，排在那一步工具后面，和她发的图一样收起前面那一段。
//! 新来的照这一轮是谁开的定放不放（核心 `view.md` 的 `voice` 条目）：开这一轮的请求是这个终端发的，当场放；开这一轮的那句话是
//! 到点叫醒的、这一轮不是哪句话开的，是没人开口的一轮，正在看这个会话的才放；别的头上说的、群里的人说的只画。补发的、翻页读进来的
//! 一律不放。放不放记在 [`Transcript::take_play`] 里，界面收完这一条推送拿走去放（`app/voice.rs`）。

use std::collections::BTreeMap;

use super::{Kind, Transcript};
use crate::core::{Spoken, Voice};

/// 记着的话最多几句（只为认出开这一轮的那一句）。
const KEPT: usize = 64;

/// 这一轮是谁开的。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Opener {
    /// 这个终端发的请求开的。
    Here,
    /// 没人开口：到点叫醒的、不是哪句话开的。
    #[default]
    Nobody,
    /// 别的头上、平台上的人说的话开的。
    Elsewhere,
}

/// 语音条要记着的。
#[derive(Debug, Default)]
pub(super) struct Voices {
    /// 落了盘的话：序号到是不是到点叫醒的。
    asked: BTreeMap<u64, bool>,
    /// 在跑的这一轮是谁开的。
    opener: Opener,
    /// 刚来、该放的那一段：放的人照 `Opener` 再挑（停放着的会话只放这个终端开的）。
    play: Option<(Voice, Opener)>,
}

impl Transcript {
    /// 收一条语音条要的。
    pub(super) fn spoken(&mut self, spoken: Spoken) {
        match spoken {
            Spoken::Asked { seq, nobody } => {
                let asked = &mut self.voices.asked;
                asked.insert(seq, nobody);
                while asked.len() > KEPT {
                    asked.pop_first();
                }
            }
            // 补发来的这一轮（切进会话、重连）认不全是谁开的（补发的人说的话认不出是哪个头），当别处开的：补完以后接着来的
            // 语音也只画不放。
            Spoken::Opened { trigger, mine } => {
                let said_by = trigger.and_then(|t| self.voices.asked.get(&t).copied());
                self.voices.opener = match (self.replaying(), mine, said_by) {
                    (true, ..) => Opener::Elsewhere,
                    (false, true, _) => Opener::Here,
                    (false, false, Some(false)) => Opener::Elsewhere,
                    (false, false, Some(true) | None) => Opener::Nobody,
                };
            }
            Spoken::Clip(voice) => self.voice(voice),
        }
    }

    /// 她说了一段：画一条；看着流出来的、这一轮这个终端开的或者没人开口的，记下来等界面拿去放。
    fn voice(&mut self, voice: Voice) {
        self.finish_segment();
        self.push(Kind::Voice, String::new());
        let opener = self.voices.opener;
        if !self.replaying() && opener != Opener::Elsewhere {
            self.voices.play = Some((voice.clone(), opener));
        }
        if let Some(last) = self.entries.last_mut() {
            last.voice = Some(voice);
        }
    }

    /// 拿走刚来、该放的那一段和这一轮是谁开的；没有的是 `None`。
    pub fn take_play(&mut self) -> Option<(Voice, Opener)> {
        self.voices.play.take()
    }
}

#[cfg(test)]
mod tests {
    use super::Opener;
    use crate::config::Config;
    use crate::core::{Push, Spoken, Update, Voice};
    use crate::transcript::{Kind, Transcript};

    fn clip() -> Voice {
        Voice {
            blob: "sha256:aa".into(),
            media_type: "audio/wav".into(),
            name: Some("say-1.wav".into()),
            duration_ms: Some(1200),
        }
    }

    fn feed(transcript: &mut Transcript, pushes: Vec<Push>) {
        let config = Config::builtin().expect("出厂配置读得懂");
        for push in pushes {
            transcript.update(Update::Push(push), &config.text);
        }
    }

    #[test]
    fn who_opened_the_turn_decides_whether_a_voice_plays() {
        // 核心 `view.md` 的 `voice`：这个终端开的当场放；到点叫醒的、不是哪句话开的，开着会话的都放；别处说的只画。
        let cases = [
            (Some(false), true, Some(Opener::Here)),
            (Some(true), false, Some(Opener::Nobody)),
            (None, false, Some(Opener::Nobody)),
            (Some(false), false, None),
        ];
        for (asked, mine, want) in cases {
            let mut t = Transcript::default();
            let mut pushes: Vec<Push> = asked
                .map(|nobody| Push::Voice(Spoken::Asked { seq: 4, nobody }))
                .into_iter()
                .collect();
            pushes.extend([
                Push::TurnStarted(5, Some(4)),
                Push::Voice(Spoken::Opened {
                    trigger: Some(4),
                    mine,
                }),
                Push::Voice(Spoken::Clip(clip())),
            ]);
            feed(&mut t, pushes);
            assert_eq!(t.take_play().map(|(_, o)| o), want, "{asked:?} {mine}");
            assert!(t.take_play().is_none(), "拿走了就没了");
            let drawn = t.entries.iter().find(|e| e.kind == Kind::Voice);
            assert_eq!(drawn.and_then(|e| e.voice.clone()), Some(clip()), "都画");
        }
    }

    #[test]
    fn a_replayed_voice_never_plays() {
        let mut t = Transcript::default();
        let at = "2026-11-18T00:00:00Z".parse().expect("时刻");
        feed(
            &mut t,
            vec![
                Push::Clock(Some(at)),
                Push::TurnStarted(5, None),
                Push::Voice(Spoken::Opened {
                    trigger: None,
                    mine: false,
                }),
                Push::Voice(Spoken::Clip(clip())),
            ],
        );
        assert!(t.take_play().is_none());
        assert!(t.entries.iter().any(|e| e.kind == Kind::Voice));
    }
}
