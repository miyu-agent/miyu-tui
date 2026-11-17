//! 通讯平台上的人说的话怎么写来处（蓝图 `tui.md`「别处来的话」第 2 条，核心 VW-2、VP-1补；同网页 W-1210、W-1212 的写法）：
//! 看场所会话（`--resume` 一个群、私聊的会话）时才见得到。来处写「昵称（qq:号）」，没名字的写号，都没有的写「外部」；
//! 管理员后面接「 · 管理员」。戳了 AI 一下的不画成一句话，照 QQ 群里的灰字提示画一行「阿杰（qq:20001） 戳了戳你的小脑袋」：
//! 名字同来处、不带管理员，后缀是那人自己设的半句，没写的不接。名字、后缀都不可信，照别处来的名字清理（[`untrusted`]）。

use super::notes::untrusted;
use crate::config::JobTexts;
use crate::core::Speaker;

/// 说话的人：「昵称（qq:号）」，没名字的写号，都没有的写「外部」。不带管理员。
fn who(speaker: &Speaker, words: &JobTexts) -> String {
    match (&speaker.name, &speaker.id) {
        (Some(name), Some(id)) => words
            .external_speaker
            .replace("{name}", &untrusted(name))
            .replace("{id}", &untrusted(id)),
        (Some(name), None) => untrusted(name),
        (None, Some(id)) => untrusted(id),
        (None, None) => words.external.clone(),
    }
}

/// 来处那一行：管理员的后面接「 · 管理员」。
pub(super) fn label(speaker: &Speaker, words: &JobTexts) -> String {
    let who = who(speaker, words);
    if speaker.manager {
        words.external_manager.replace("{who}", &who)
    } else {
        who
    }
}

/// 戳了一下的那一行：`suffix` 是那人设的后缀，空的不接。
pub(super) fn poked(speaker: &Speaker, suffix: &str, words: &JobTexts) -> String {
    let suffix = if suffix.trim().is_empty() {
        String::new()
    } else {
        untrusted(suffix)
    };
    words
        .poke
        .replace("{who}", &who(speaker, words))
        .replace("{suffix}", &suffix)
}

#[cfg(test)]
mod tests {
    use super::{label, poked};
    use crate::config::Config;
    use crate::core::Speaker;

    fn speaker(id: Option<&str>, name: Option<&str>, manager: bool) -> Speaker {
        Speaker {
            id: id.map(str::to_string),
            name: name.map(str::to_string),
            manager,
        }
    }

    #[test]
    fn venue_speakers_read_as_nickname_and_number_and_pokes_as_one_line() {
        let config = Config::builtin().expect("出厂配置读得懂");
        let words = &config.text.jobs;
        let member = speaker(Some("qq:20001"), Some("阿杰"), false);
        assert_eq!(label(&member, words), "阿杰（qq:20001）");
        let manager = speaker(Some("qq:20002"), Some("小美"), true);
        assert_eq!(label(&manager, words), "小美（qq:20002） · 管理员");
        assert_eq!(
            label(&speaker(Some("qq:20003"), None, false), words),
            "qq:20003"
        );
        assert_eq!(label(&speaker(None, None, false), words), "外部");
        assert_eq!(
            poked(&member, "的小脑袋", words),
            "阿杰（qq:20001） 戳了戳你的小脑袋"
        );
        // 管理员戳的不带「 · 管理员」；没设后缀的不接。
        assert_eq!(poked(&manager, "", words), "小美（qq:20002） 戳了戳你");
        // 名字、后缀不可信：换行、控制字符去掉。
        let odd = speaker(Some("qq:20004"), Some("老王\n\u{1b}[31m"), false);
        assert_eq!(
            poked(&odd, "的脸\r\n", words),
            "老王 [31m（qq:20004） 戳了戳你的脸  "
        );
    }
}
