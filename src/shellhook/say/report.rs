//! `shellhook report`：shell 回到提示符时印后台回报（设计 22 第七节「后台任务和回报」，图纸 `shellhook.md`「后台回报」）。
//! 盯后台的（`follow.rs`）记进信箱的照先后印：任务结束的那一行照全屏写（绿点、红叉）；回报叫醒她的那一轮接上画，要确认、
//! 要回答的照常弹。

use std::io;
use std::process::ExitCode;
use std::time::Instant;

use super::session::Session;
use super::spool::{self, Record};
use super::turn::Turn;
use crate::core::Command;

/// 印 `shell` 信箱里的；没有的什么都不做。
///
/// # Errors
///
/// 读配置、开原始模式、写终端失败。
pub fn run(shell: u32, leftover: Option<&std::path::Path>) -> io::Result<ExitCode> {
    let records = spool::take(shell);
    if records.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    let (mut session, look) = Session::start()?;
    // 配色、给人看的字、时间线的配置读一次，几件共用：上一件画完的配色交给下一件。
    let mut look = Some(look);
    // 印的时候敲的字：收工时写进 `leftover`，脚本接在命令行后面。
    let mut typed = String::new();
    for record in records {
        let Some(current) = look.take() else { break };
        match record {
            Record::Ended { start, end } => {
                let mut turn = Turn::attach(String::new(), 0, current);
                session.style(&mut turn);
                let now = Instant::now();
                turn.board.start(&start, None, now);
                if let Some(id) = turn.board.end(&end, now) {
                    crate::jobs::announce(
                        &turn.board,
                        &mut turn.transcript,
                        id,
                        now,
                        &session.config.text.jobs,
                    );
                }
                session.print(&turn)?;
                look = Some(turn.into_look());
            }
            Record::Turn { session: id, turn } => {
                let mut woken = Turn::attach(id.clone(), turn, current);
                session.core.send(Command::Open {
                    session: id,
                    keep: false,
                });
                session.drive(&mut woken, |_| None)?;
                typed.push_str(&woken.typed);
                look = Some(woken.into_look());
            }
        }
    }
    session.finish()?;
    if let Some(path) = leftover
        && !typed.is_empty()
    {
        std::fs::write(path, typed)?;
    }
    Ok(ExitCode::SUCCESS)
}
