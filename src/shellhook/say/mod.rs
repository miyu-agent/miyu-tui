//! `shellhook say`：把一句话交给她，这一轮照终端界面时间线的样子印在 shell 里（图纸 `shellhook.md`「say」）。
//!
//! 连核心、收推送、排时间线都照全屏那一套（`core::spawn`、`Transcript`、`ui/rows.rs`），只是不进全屏：光标下面一块活动区
//! 原地刷新（`paint.rs`），这一轮结束时整段定下来留在滚动区。说到哪个会话问核心（`session.terminal`，`terminal.rs`）。

mod asking;
mod look;
mod paint;
mod terminal;
mod turn;
mod view;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use ratatui::crossterm::event::Event;

use crate::config::Config;
use crate::core::{Command, Start};
use crate::notify;
use crate::shellhook::words::Words;
use crate::transcript::Link;
use paint::Painter;
use turn::{Incoming, Turn};
use view::{Media, View};

/// 按了 Ctrl+C 打断的退出码：同 `miyu ask`（蓝图 `cli/ask.md`）。
const INTERRUPTED: u8 = 3;

/// 照这几样办。
pub struct Ask {
    /// 说的那句话。
    pub text: String,
    /// 这期间提前敲的字写进这个文件，脚本读了放回命令行；没给的不收。
    pub leftover: Option<PathBuf>,
}

/// 说一句、印这一轮，交回退出码。
///
/// # Errors
///
/// 读配置、开原始模式、写终端失败。
pub fn run(ask: Ask, words: &Words) -> io::Result<ExitCode> {
    let text = ask.text.trim_end_matches(['\n', '\r']).to_string();
    if text.trim().is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    let table = crate::language::LanguageTable::builtin().map_err(io::Error::other)?;
    let language = table.detect(|name| std::env::var(name).ok());
    let mut config = Config::load(&language, true).map_err(io::Error::other)?;
    let (sender, incoming) = mpsc::channel();
    let to_core = sender.clone();
    // 不进任何会话：先问核心终端会话是哪一个，再接上（`turn.rs`）。
    let core = crate::core::spawn(config.layout.reconnect_ms, Start::Bare, move |update| {
        to_core.send(Incoming::Core(update)).is_ok()
    });
    ratatui::crossterm::terminal::enable_raw_mode()?;
    let raw = RawMode;
    let look = look::Look::new(terminal_colors());
    // 终端在不在前台：不在时回答好了、在等你确认弹通知（同全屏「系统通知」，2026-10-20 项目主人验收要的）。
    ratatui::crossterm::execute!(io::stdout(), ratatui::crossterm::event::EnableFocusChange)?;
    let mut notifier = notifier(&config);
    let mut drawer_seen: Option<String> = None;
    let reader =
        crate::reader::Reader::spawn(move |event| sender.send(Incoming::Key(event)).is_ok());
    core.send(Command::FetchHuman(language.code().to_string()));
    // 这一句照 shell 所在的目录说（设计 22 第七节「她在哪个目录干活」）。
    let cwd = std::env::current_dir()
        .ok()
        .map(|d| d.to_string_lossy().into_owned());
    let mut turn = Turn::new(text, cwd, look);
    turn.ask_terminal(&core);
    let media = Media::new(&config);
    let depth = crate::theme::Depth::detect(|name| std::env::var(name).ok());
    let mut painter = Painter::new(io::stdout(), depth);
    let started = Instant::now();
    let spinner = config.timeline.spinner_ms.max(16);
    while !turn.done {
        match incoming.recv_timeout(Duration::from_millis(spinner)) {
            Ok(Incoming::Core(update)) => turn.core(update, &mut config, &core),
            Ok(Incoming::Key(event)) => {
                match event {
                    Event::FocusGained => notifier.focus(true),
                    Event::FocusLost => notifier.focus(false),
                    _ => {}
                }
                if turn.key(&event, &core, &config) {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        turn.look.apply(&config);
        let (width, height) = ratatui::crossterm::terminal::size().unwrap_or((80, 24));
        let frame =
            usize::try_from(started.elapsed().as_millis() / u128::from(spinner)).unwrap_or(0);
        let view = View {
            config: &config,
            human: &turn.human,
            media: &media,
            board: &turn.board,
            width,
            height,
            frame,
        };
        // 不印运行状态行（词、用时）：shell 里只要转圈（2026-10-20 项目主人验收：不要文字进度条）。
        let lines = view.lines(&turn.transcript, turn.ours, true);
        // 开着的抽屉画在最下面，最多占一半高（图纸「确认和提问」）。
        let drawer = turn.asking.height(&config, width, height / 2).unwrap_or(0);
        // 活动区最多占终端高减一行：长过的只露最新的那一截，这一轮结束时整段印出来（图纸「怎么印」第 2 条）。
        let room = usize::from(height.saturating_sub(1 + drawer)).max(1);
        let tail = lines.len().saturating_sub(room);
        let mut live = painter.lines(width, &lines[tail..])?;
        if drawer > 0 {
            let asking = &mut turn.asking;
            live.extend(painter.render(width, drawer, |frame| {
                asking.draw(frame, frame.area(), &config)
            })?);
        }
        let settled = match turn.renewed.take() {
            Some(since) => painter.lines(width, &[renewed_line(since, words)])?,
            None => Vec::new(),
        };
        painter.frame(&settled, &live)?;
        // 新开了一个抽屉：弹「在等你确认」「在等你回答」。
        let current = turn.asking.current();
        if let Some((call, question)) = &current
            && drawer_seen.as_ref() != Some(call)
        {
            notifier.tell(if *question {
                notify::Event::Asking
            } else {
                notify::Event::Approving
            });
        }
        drawer_seen = current.map(|(call, _)| call);
        send_out(&mut notifier)?;
    }
    // 收工以前敲进来的、读按键的线程已经读走还没处理的字也收下：先让它停下，再把排着的拿完（真机实测：答得快时最后
    // 那几个字会丢）。
    reader.pause();
    for event in incoming.try_iter() {
        if let Incoming::Key(event) = event {
            turn.keep_typed(&event);
        }
    }
    // 定下来：整段照最后的样子印一遍（做完收起的照配置收起），留在滚动区。
    let (width, height) = ratatui::crossterm::terminal::size().unwrap_or((80, 24));
    let view = View {
        config: &config,
        human: &turn.human,
        media: &media,
        board: &turn.board,
        width,
        height,
        frame: 0,
    };
    let lines = view.lines(&turn.transcript, turn.ours, turn.todos_changed);
    let settled = painter.lines(width, &lines)?;
    painter.finish()?;
    painter.frame(&settled, &[])?;
    match turn.ended {
        Some(crate::core::EndReason::Completed) => notifier.tell(notify::Event::Replied),
        Some(crate::core::EndReason::Error) => notifier.tell(notify::Event::Failed),
        _ => {}
    }
    send_out(&mut notifier)?;
    ratatui::crossterm::execute!(io::stdout(), ratatui::crossterm::event::DisableFocusChange)?;
    drop(reader);
    drop(raw);
    if let Some(path) = &ask.leftover
        && !turn.typed.is_empty()
    {
        std::fs::write(path, &turn.typed)?;
    }
    if let Link::Down(reason) = &turn.transcript.link {
        writeln!(io::stderr(), "{reason}")?;
        return Ok(ExitCode::FAILURE);
    }
    Ok(if turn.interrupted {
        ExitCode::from(INTERRUPTED)
    } else {
        ExitCode::SUCCESS
    })
}

/// 闲置久了换了新会话的那一行：暗色，印在这一轮前面（设计 22 第七节「终端会话」）。
fn renewed_line(since: jiff::Timestamp, words: &Words) -> Line<'static> {
    let ago = terminal::ago(since, jiff::Timestamp::now(), &words.units);
    let text = crate::shellhook::words::fill(&words.renewed, &[("ago", &ago)]);
    Line::from(Span::styled(
        format!("  {text}"),
        Style::default().patch(crate::theme::dim()),
    ))
}

/// 问终端的字色、底色和 16 色（在开始读按键以前问，回话不让读按键读走），装上，交回底色深不深（明暗自动照它）。
fn terminal_colors() -> Option<bool> {
    let ground = crate::theme::terminal_colors::probe_ground();
    let colors = crate::theme::terminal_colors::probe();
    let depth = crate::theme::Depth::detect(|name| std::env::var(name).ok());
    crate::theme::terminal_colors::install(colors, ground, depth);
    ground.map(|g| g.dark)
}

/// 系统通知：照全屏的做法、提示音放在同一个缓存目录。
fn notifier(config: &Config) -> notify::Notifier {
    let cache = miyu_client::places::cache_root(&miyu_client::places::Env::current()).ok();
    let sounds = cache.map(|root| root.join("tui").join("sounds"));
    notify::Notifier::new(
        config.notify.clone(),
        config.text.notify.clone(),
        |name| std::env::var(name).ok(),
        sounds,
    )
}

/// 攒着的通知（OSC 序列）写给终端。
fn send_out(notifier: &mut notify::Notifier) -> io::Result<()> {
    let mut out = io::stdout();
    for sequence in notifier.outbox() {
        out.write_all(sequence.as_bytes())?;
    }
    out.flush()
}

/// 原始模式：收工、出错都关上，不把 shell 留在原始模式里。
struct RawMode;

impl Drop for RawMode {
    fn drop(&mut self) {
        if ratatui::crossterm::terminal::disable_raw_mode().is_err() {}
    }
}
