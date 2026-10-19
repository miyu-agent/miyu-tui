//! `shellhook say`：把一句话交给她，这一轮照终端界面时间线的样子印在 shell 里（图纸 `shellhook.md`「say」）。
//!
//! 连核心、收推送、排时间线都照全屏那一套（`core::spawn`、`Transcript`、`ui/rows.rs`），只是不进全屏：光标下面一块活动区
//! 原地刷新（`paint.rs`），这一轮结束时整段定下来留在滚动区。说到哪个会话问核心（`session.terminal`，`terminal.rs`）。

mod asking;
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

use crate::config::Config;
use crate::core::{Command, Start};
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
    theme(&config);
    let reader =
        crate::reader::Reader::spawn(move |event| sender.send(Incoming::Key(event)).is_ok());
    core.send(Command::FetchHuman(language.code().to_string()));
    // 这一句照 shell 所在的目录说（设计 22 第七节「她在哪个目录干活」）。
    let cwd = std::env::current_dir()
        .ok()
        .map(|d| d.to_string_lossy().into_owned());
    let mut turn = Turn::new(text, cwd);
    turn.ask_terminal(&core);
    let media = Media::new(&config);
    let depth = crate::theme::Depth::detect(|name| std::env::var(name).ok());
    let mut painter = Painter::new(io::stdout(), depth);
    let mut pulse = crate::pulse::Pulse::new(seed());
    let started = Instant::now();
    let spinner = config.timeline.spinner_ms.max(16);
    while !turn.done {
        match incoming.recv_timeout(Duration::from_millis(spinner)) {
            Ok(Incoming::Core(update)) => turn.core(update, &mut config, &core),
            Ok(Incoming::Key(event)) => {
                if turn.key(&event, &core, &config) {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let (width, height) = ratatui::crossterm::terminal::size().unwrap_or((80, 24));
        let frame =
            usize::try_from(started.elapsed().as_millis() / u128::from(spinner)).unwrap_or(0);
        let view = View {
            config: &config,
            human: &turn.human,
            media: &media,
            width,
            height,
            frame,
        };
        let mut lines = view.lines(&turn.transcript, turn.ours);
        if let Some(run) = turn.transcript.running {
            lines.push(Line::default());
            lines.push(crate::ui::pulse_line(
                run,
                turn.transcript.beat(),
                turn.transcript.retry.as_deref(),
                &mut pulse,
                &config,
            ));
        }
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
        width,
        height,
        frame: 0,
    };
    let settled = painter.lines(width, &view.lines(&turn.transcript, turn.ours))?;
    painter.finish()?;
    painter.frame(&settled, &[])?;
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

/// 先照出厂的设置挑一套颜色（跟随终端，问到的终端底色照它挑深浅）；shell 里不铺底色（滚动区里铺一块底很怪）。
fn theme(config: &Config) {
    let ground = crate::theme::terminal_colors::probe_ground();
    let colors = crate::theme::terminal_colors::probe();
    let depth = crate::theme::Depth::detect(|name| std::env::var(name).ok());
    crate::theme::terminal_colors::install(colors, ground, depth);
    let choice = crate::theme::choice::Choice {
        ground: false,
        ..Default::default()
    };
    if let Some(picked) =
        crate::theme::choice::pick(&choice, ground.map(|g| g.dark), &config.themes, None)
    {
        crate::theme::set(picked.palette);
    }
}

/// 运行状态行挑词的随机数：照这一刻定。
fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| u64::try_from(d.as_nanos()).unwrap_or(1))
}

/// 原始模式：收工、出错都关上，不把 shell 留在原始模式里。
struct RawMode;

impl Drop for RawMode {
    fn drop(&mut self) {
        if ratatui::crossterm::terminal::disable_raw_mode().is_err() {}
    }
}
