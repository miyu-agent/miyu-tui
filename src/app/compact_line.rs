//! 这个会话的压缩触发线（`/compact-line`，核心 CT-1；蓝图 `tui.md`「会话的压缩触发线」，2026-11-11 项目主人定单开一个命令）：
//! `/compact-line 70` 钉成窗口的 70%，`/compact-line off` 回到跟着配置（`compaction.line`），只打 `/compact-line` 说现在是多少、
//! 从哪来。钉的照推来的 `session.policy_changed` 记在会话上（`Transcript::compaction_pin`），配置的值连上、配置变了时读一次。

use serde_json::json;

use super::App;
use crate::core::{Command, Update};
use crate::oobe::asker::TAG_BASE;

/// 读配置的压缩触发线。
const LINE_TAG: u64 = TAG_BASE - 44;
/// 换这个会话的压缩触发线（`session.configure`）。
const SET_TAG: u64 = TAG_BASE - 45;
/// 配置项。
const LINE_KEY: &str = "compaction.line";
/// 核心收的范围（窗口的百分之几）。
const RANGE: std::ops::RangeInclusive<u64> = 20..=95;

/// `/compact-line` 后面写的是什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Asked {
    /// 只打了命令：说现在是多少。
    Show,
    /// 钉成多少，`None` 是回到跟着配置（`off`）。
    Set(Option<u64>),
    /// 不是 20 到 95 的整数、也不是 `off`。
    Bad,
}

/// 读 `/compact-line` 后面的字。
pub(super) fn read(words: Option<&str>) -> Asked {
    match words.map(str::trim).filter(|w| !w.is_empty()) {
        None => Asked::Show,
        Some(w) if w.eq_ignore_ascii_case("off") => Asked::Set(None),
        Some(w) => match w.trim_end_matches('%').parse::<u64>() {
            Ok(n) if RANGE.contains(&n) => Asked::Set(Some(n)),
            _ => Asked::Bad,
        },
    }
}

impl App {
    /// `/compact-line`：照 [`read`] 办。还没开会话的当场说（同 `/rename`），不去开一个。
    pub(super) fn compact_line(&mut self, words: Option<&str>) {
        let texts = self.config.text.compact_line.clone();
        let asked = read(words);
        if asked == Asked::Bad {
            self.hint(texts.range, false);
            return;
        }
        let Some(session) = self.talk.transcript.session.clone() else {
            self.hint(texts.not_open, false);
            return;
        };
        match asked {
            Asked::Show => {
                let note = self.compact_line_now();
                self.hint(note, false);
            }
            Asked::Set(line) => {
                self.talk.compaction_asked = Some(line);
                self.link.core.send(Command::Ask {
                    tag: SET_TAG,
                    method: "session.configure",
                    params: json!({"session": session, "compaction_line": line}),
                });
            }
            Asked::Bad => {}
        }
    }

    /// 现在的压缩线说成一句：钉了的写「（这个会话）」，没钉的照配置写「（跟着设置）」，配置还没读到的不写数。
    fn compact_line_now(&self) -> String {
        let texts = &self.config.text.compact_line;
        let fill = |text: &str, n: u64| text.replace("{n}", &n.to_string());
        match (
            self.talk.transcript.compaction_pin,
            self.talk.compaction_line,
        ) {
            (Some(pin), _) => fill(&texts.pinned, pin),
            (None, Some(line)) => fill(&texts.system, line),
            (None, None) => texts.system_bare.clone(),
        }
    }

    /// 连上、配置变了读配置的那一项；读回来的、换成了没有的回应归这里（交回 `true`）。
    pub(super) fn compact_line_update(&mut self, update: &Update) -> bool {
        match update {
            Update::HeadConfig(_) => {
                self.ask_config(LINE_TAG, &[LINE_KEY]);
                false
            }
            Update::Answer { tag, result } if *tag == LINE_TAG => {
                if let Ok(got) = result {
                    self.talk.compaction_line = got["items"][LINE_KEY]["value"].as_u64();
                }
                true
            }
            Update::Answer { tag, result } if *tag == SET_TAG => {
                let asked = self.talk.compaction_asked.take().unwrap_or(None);
                match result {
                    Ok(_) => {
                        // 推来的 `session.policy_changed` 也会记；先记上，紧接着打 `/compact-line` 也对。
                        self.talk.transcript.compaction_pin = asked;
                        let note = self.compact_line_receipt(asked);
                        self.hint(note, false);
                    }
                    Err(refusal) if refusal.data["field"] == "compaction_line" => {
                        let note = self.config.text.compact_line.range.clone();
                        self.hint(note, false);
                    }
                    Err(refusal) => self.slash_refused(&refusal.message),
                }
                true
            }
            _ => false,
        }
    }

    /// 换成了的回执：钉成多少；回到跟着配置的带上配置的值（读不到不带）。
    fn compact_line_receipt(&self, line: Option<u64>) -> String {
        let texts = &self.config.text.compact_line;
        match (line, self.talk.compaction_line) {
            (Some(n), _) => texts.set_to.replace("{n}", &n.to_string()),
            (None, Some(n)) => texts.off.replace("{n}", &n.to_string()),
            (None, None) => texts.off_bare.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
