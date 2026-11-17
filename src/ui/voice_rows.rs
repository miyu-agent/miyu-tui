//! 她说出来的语音（核心 SD-1，蓝图 `tui.md`「她说的语音」）：一行语音条，`▶ 0:01 · say-1.wav`，在放的打头换成 `■`。点这一行放，
//! 在放的再点停（经总线 `tts.play`、`tts.stop`，同 `/read-aloud`，`app/voice.rs`）。放不了的（总线上没有 `tts`、终端不认的格式）
//! 照样画这一行，后面暗色写「无法播放」，点了没反应。

use ratatui::text::Span;

use super::rows::{Ctx, Row, clip};
use crate::core::Voice;
use crate::theme;
use crate::transcript::Entry;

/// 语音条点中时挂的地址：后面是条目的编号（`app/voice.rs` 照它认）。
pub const SCHEME: &str = "miyu-voice:";

/// 终端放得了的格式：交给总线 `tts.play` 的语音包认 mp3、wav（`rodio` 带的解码）。
const PLAYABLE: [&str; 5] = [
    "audio/mpeg",
    "audio/mp3",
    "audio/wav",
    "audio/x-wav",
    "audio/wave",
];

/// 这个终端放声音的情况：总线上有没有 `tts`、在放哪一段（内容哈希）。
#[derive(Debug, Clone, Copy, Default)]
pub struct Speaker<'a> {
    /// 总线上 `tts` 可用。
    pub ready: bool,
    /// 在放的那一段；没在放的是 `None`。
    pub playing: Option<&'a str>,
}

/// 一段语音这一刻的样子：放不了、没在放、在放。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    /// 放不了。
    Unplayable,
    /// 没在放。
    Idle,
    /// 在放。
    Playing,
}

impl Speaker<'_> {
    /// `voice` 这一刻的样子。
    pub fn state(&self, voice: &Voice) -> State {
        if !self.ready || !playable(&voice.media_type) {
            State::Unplayable
        } else if self.playing == Some(voice.blob.as_str()) {
            State::Playing
        } else {
            State::Idle
        }
    }
}

/// 终端放得了这种格式（大小写、`;` 后面的参数不算）。
pub fn playable(media_type: &str) -> bool {
    let bare = media_type.split(';').next().unwrap_or_default().trim();
    PLAYABLE.iter().any(|t| t.eq_ignore_ascii_case(bare))
}

/// 工具报的文件名不可信：换行、制表符换成空格，别的控制字符去掉。
fn plain(name: &str) -> String {
    name.chars()
        .map(|c| {
            if matches!(c, '\n' | '\r' | '\t') {
                ' '
            } else {
                c
            }
        })
        .filter(|c| !c.is_control())
        .collect()
}

/// 时长：`0:01`，过了一小时的 `1:02:03`；没报的写 `--:--`。
fn length(ms: Option<u64>, unknown: &str) -> String {
    let Some(ms) = ms else {
        return unknown.to_string();
    };
    // 照网页（W-1212）四舍五入到秒。
    let secs = (ms + 500) / 1000;
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// 正文里这一条排成的行。
pub fn rows(entry: &Entry, ctx: &Ctx) -> Vec<Row> {
    let Some(voice) = &entry.voice else {
        return Vec::new();
    };
    let words = &ctx.config.text.voice;
    let state = ctx.speaker.state(voice);
    let mark = &words.clip_marks[usize::from(state == State::Playing)];
    let mut tail = length(voice.duration_ms, &words.clip_unknown);
    if let Some(name) = &voice.name {
        tail.push_str(&crate::sheet::sep());
        tail.push_str(&plain(name));
    }
    if state == State::Unplayable {
        tail.push_str(&crate::sheet::sep());
        tail.push_str(&words.unplayable);
    }
    let mark_style = if state == State::Unplayable {
        theme::dim()
    } else {
        theme::accent()
    };
    let line = clip(&format!("{mark} {tail}"), ctx.width);
    let rest = line
        .strip_prefix(mark.as_str())
        .unwrap_or(&line)
        .to_string();
    let mut row = ctx.row(
        ctx.blank_slot(),
        vec![
            Span::styled(mark.clone(), mark_style),
            Span::styled(rest, theme::dim()),
        ],
    );
    row.copy = false;
    if state != State::Unplayable {
        row.set_links(vec![(0, ctx.width, format!("{SCHEME}{}", entry.id))]);
    }
    vec![row]
}

#[cfg(test)]
mod tests {
    use super::{Speaker, State, length, playable};
    use crate::core::Voice;

    fn wav() -> Voice {
        Voice {
            blob: "sha256:aa".into(),
            media_type: "audio/wav".into(),
            name: Some("say-1.wav".into()),
            duration_ms: Some(1200),
        }
    }

    #[test]
    fn a_clip_reads_its_length_and_whether_it_plays() {
        assert_eq!(length(Some(1200), "--:--"), "0:01");
        assert_eq!(length(Some(1500), "--:--"), "0:02");
        assert_eq!(length(Some(61_000), "--:--"), "1:01");
        assert_eq!(length(Some(3_723_000), "--:--"), "1:02:03");
        assert_eq!(length(None, "--:--"), "--:--");
        assert!(playable("audio/mpeg") && playable("Audio/WAV; codecs=1"));
        assert!(!playable("audio/ogg"), "语音包不认 ogg");
        let voice = wav();
        let idle = Speaker {
            ready: true,
            playing: None,
        };
        assert_eq!(idle.state(&voice), State::Idle);
        let playing = Speaker {
            ready: true,
            playing: Some("sha256:aa"),
        };
        assert_eq!(playing.state(&voice), State::Playing);
        assert_eq!(
            Speaker::default().state(&voice),
            State::Unplayable,
            "没有 tts"
        );
        let ogg = Voice {
            media_type: "audio/ogg".into(),
            ..wav()
        };
        assert_eq!(idle.state(&ogg), State::Unplayable);
    }

    #[test]
    fn a_clip_is_one_line_that_says_when_it_cannot_play() {
        use crate::ui::test_support::Fixture;
        let f = Fixture::new();
        let mut entry =
            crate::transcript::Entry::new(7, crate::transcript::Kind::Voice, String::new());
        entry.voice = Some(wav());
        assert!(crate::ui::rows::shown(&entry), "字是空的也画");
        let text = |ctx: &crate::ui::rows::Ctx| {
            let rows = super::rows(&entry, ctx);
            assert_eq!(rows.len(), 1);
            (
                rows[0].line.to_string().trim_end().to_string(),
                rows[0].links().to_vec(),
            )
        };
        let (line, links) = text(&f.ctx());
        assert_eq!(line, "  ▶ 0:01 · say-1.wav · 无法播放");
        assert!(links.is_empty(), "放不了的点不动");
        let mut ctx = f.ctx();
        ctx.speaker = Speaker {
            ready: true,
            playing: None,
        };
        let (line, links) = text(&ctx);
        assert_eq!(line, "  ▶ 0:01 · say-1.wav");
        assert_eq!(links.first().map(|l| l.2.as_str()), Some("miyu-voice:7"));
        ctx.speaker.playing = Some("sha256:aa");
        assert_eq!(text(&ctx).0, "  ■ 0:01 · say-1.wav");
    }
}
