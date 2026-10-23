//! `shellhook report`：shell 回到提示符时印后台回报（设计 22 第七节「后台任务和回报」，图纸 `shellhook.md`「后台回报」）。
//! 盯后台的（`follow.rs`）记进信箱的照先后印：任务结束的那一行照全屏写（绿点、红叉）；回报叫醒她的那一轮接上画，要确认、
//! 要回答的照常弹。

use std::io::{self, Write};
use std::process::ExitCode;
use std::time::Instant;

use super::session::Session;
use super::spool::{self, Record};
use super::turn::Turn;
use crate::core::Command;

/// 信箱里有没有要印的（不拿走）。
pub fn waiting(shell: u32) -> bool {
    spool::waiting(shell)
}

/// 信箱里没有要印的：退出码（停在提示符上的脚本照它不补换行、不重画）。
const NOTHING: u8 = 1;

/// 印 `shell` 信箱里的；没有的什么都不做。`above`：停在提示符上印，提示符在光标上面占几行，有要印的先擦掉提示符在那里
/// 印（没有要印的不擦：`say` 已经印过、从信箱里拿掉了的，信号照样来，2026-10-20 验收擦掉了她的收尾行）。
///
/// # Errors
///
/// 读配置、开原始模式、写终端失败。
pub fn run(
    shell: u32,
    leftover: Option<&std::path::Path>,
    above: Option<u16>,
) -> io::Result<ExitCode> {
    let records = spool::take(shell);
    if records.is_empty() {
        return Ok(ExitCode::from(NOTHING));
    }
    if let Some(rows) = above {
        let mut out = io::stdout();
        if rows > 0 {
            write!(out, "\x1b[{rows}A")?;
        }
        write!(out, "\r\x1b[J")?;
        out.flush()?;
    }
    let (mut session, look) = Session::start()?;
    // 配色、给人看的字、时间线的配置读一次，几件共用：上一件画完的配色交给下一件。
    let mut look = Some(look);
    // 给人看的字（工具的显示名）同样交给下一件：核心只回一次（原来接上的那一轮工具名是原名 `jobs`，2026-10-20 验收）。
    let mut human = crate::human::Human::default();
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
                human = turn.human.clone();
                look = Some(turn.into_look());
            }
            Record::Turn { session: id, turn } => {
                let mut woken = Turn::attach(id.clone(), turn, current);
                woken.human = human.clone();
                session.core.send(Command::Open {
                    session: id,
                    keep: false,
                });
                session.drive(&mut woken, |_| None)?;
                typed.push_str(&woken.typed);
                human = woken.human.clone();
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
