//! shell 里打的 `/dream`（同终端界面「整理记忆」，`app/dream.rs`）：交给核心以前先订阅这个会话的记忆流，推来的照 [`Dream`]
//! 算，活动区照终端界面那一块画（第几批、滚动区、夜空，`ui::dream_rows`）；回执来了她醒一下，再收成回执那一行、退订。
//! 被拒的收掉，照旧印原话。`Ctrl+C` 叫停（同终端界面的两下 `Esc`），再按一下不等了，整理照样在后台跑完。

use std::time::{Duration, Instant};

use crate::config::Config;
use crate::core::{Command, Core, Update};
use crate::dream::Dream;
use crate::transcript::Kind;

use super::turn::Turn;

/// 打的这一行是 `/dream`：交出去以前订阅记忆流（连接那头先订阅、再交命令，开头那一批不漏）。
pub fn start(turn: &mut Turn, line: &str, core: &Core) {
    if !crate::dream::asks_to_dream(line) {
        return;
    }
    let session = turn.transcript.session.clone().unwrap_or_default();
    turn.dream = Some(Dream::new(session, Instant::now()));
    core.send(Command::Dream(true));
}

/// 核心来的一件：记忆流推来的、`/dream` 的回执、被拒，归这里办了交回 `true`。
pub fn update(turn: &mut Turn, update: &Update, core: &Core) -> bool {
    let Some(dream) = turn.dream.as_mut() else {
        return false;
    };
    match update {
        Update::Dream { session, change } => {
            // 还没认上会话的（核心替它开的）认第一回推来的那个。
            if dream.session.is_empty() {
                dream.session.clone_from(session);
            }
            if *session == dream.session && dream.woke.is_none() {
                dream.apply(change.clone(), Instant::now());
            }
            true
        }
        Update::CommandRan { command, said } if command == "dream" && dream.woke.is_none() => {
            dream.woke = Some((Instant::now(), said.clone()));
            core.send(Command::Dream(false));
            true
        }
        // 被拒（多半是 `/dream` 自己：没装记忆、正在整理）：收掉、退订，原话照交给核心的那一行印。
        Update::SlashRefused { .. } => {
            turn.dream = None;
            core.send(Command::Dream(false));
            false
        }
        _ => false,
    }
}

/// `Ctrl+C`：在整理的叫停一次（合完的批留着，正在合的那一批放弃，回执是「整理已中断」）；叫停过的、醒了的交回 `false`，
/// 照旧不等了。
pub fn stop(turn: &mut Turn, core: &Core) -> bool {
    let Some(dream) = turn
        .dream
        .as_mut()
        .filter(|d| d.woke.is_none() && !d.stopping)
    else {
        return false;
    };
    dream.stopping = true;
    core.send(Command::DreamStop);
    true
}

/// 每一拍：到点浮下一条；醒够了收成回执那一行，这一句收工（同终端界面「整理记忆」第 6 条）。
pub fn tick(turn: &mut Turn, config: &Config) {
    let look = &config.layout.dream;
    let Some(dream) = turn.dream.as_mut() else {
        return;
    };
    let now = Instant::now();
    dream.tick(now, Duration::from_millis(look.float_ms));
    let wake = Duration::from_millis(look.wake_ms);
    let Some((_, said)) = dream.woke.clone().filter(|(at, _)| now >= *at + wake) else {
        return;
    };
    let kept = dream.kept().to_vec();
    turn.dream = None;
    turn.transcript.entries.clear();
    let said = said.trim_end().trim_end_matches(['。', '.']).to_string();
    if !said.is_empty() {
        turn.transcript.note(Kind::Dreamt, said);
        if let Some(entry) = turn.transcript.entries.last_mut() {
            entry.kept = kept;
        }
    }
    turn.done = true;
}
