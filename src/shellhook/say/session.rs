//! 在 shell 里画一轮的那一套（`say` 和后台回报的 `report` 共用）：连核心、原始模式、问终端颜色、读按键、系统通知；主循环
//! 收推送、每一帧刷新活动区，这一轮结束时整段定下来留在滚动区（图纸 `shellhook.md`「怎么印」）。

use std::io::{self, Stdout, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{DisableFocusChange, EnableFocusChange, Event};
use ratatui::text::Line;

use super::look::Look;
use super::paint::Painter;
use super::turn::{Incoming, Turn};
use super::view::{Media, Scope, View};
use crate::config::Config;
use crate::core::{Command, Core, EndReason, Start};
use crate::notify::{self, Notifier};

/// 这一轮结束后等多久看核心是不是叫醒她（`turn.rs` 的 `lingering`）：核心是同一刻开的，留够推送路上的时间。
const LINGER: Duration = Duration::from_millis(500);
/// 画一帧以前最多先办几件：补发的一页再长也不至于一直不画。
const BATCH: usize = 512;

/// 一次在 shell 里画：从连上到收工。
pub struct Session {
    pub config: Config,
    pub core: Core,
    incoming: mpsc::Receiver<Incoming>,
    reader: crate::reader::Reader,
    notifier: Notifier,
    painter: Painter<Stdout>,
    media: Media,
    _raw: RawMode,
}

impl Session {
    /// 连核心（不进任何会话）、开原始模式、问终端颜色、开焦点报告、起读按键的线程。交回配色的起头（照问到的底色）。
    ///
    /// # Errors
    ///
    /// 读配置、开原始模式失败。
    pub fn start() -> io::Result<(Self, Look)> {
        let config = super::config()?;
        let (sender, incoming) = mpsc::channel();
        let to_core = sender.clone();
        let core = crate::core::spawn(config.layout.reconnect_ms, Start::Bare, move |update| {
            to_core.send(Incoming::Core(update)).is_ok()
        });
        ratatui::crossterm::terminal::enable_raw_mode()?;
        let raw = RawMode;
        let look = Look::new(terminal_colors());
        // 终端在不在前台：不在时回答好了、在等你确认弹通知（同全屏「系统通知」，2026-10-20 项目主人验收要的）。
        ratatui::crossterm::execute!(io::stdout(), EnableFocusChange)?;
        let reader =
            crate::reader::Reader::spawn(move |event| sender.send(Incoming::Key(event)).is_ok());
        core.send(Command::FetchHuman(config.language.code().to_string()));
        let depth = crate::theme::Depth::detect(|name| std::env::var(name).ok());
        let session = Self {
            notifier: notifier(&config),
            media: Media::new(&config),
            config,
            core,
            incoming,
            reader,
            painter: Painter::new(io::stdout(), depth),
            _raw: raw,
        };
        Ok((session, look))
    }

    /// 画这一轮，到它结束（或者按了 Ctrl+C 不等了）；`banner` 是这一轮前面先印的几行（换了会话的那一行）。
    ///
    /// # Errors
    ///
    /// 写终端失败。
    pub fn drive(
        &mut self,
        turn: &mut Turn,
        banner: impl Fn(&Turn) -> Option<Line<'static>>,
    ) -> io::Result<()> {
        let started = Instant::now();
        let spinner = self.config.timeline.spinner_ms.max(16);
        let mut drawer_seen: Option<String> = None;
        let mut bannered = false;
        let mut lingered: Option<Instant> = None;
        // 她答话的时候不露光标（2026-10-20 验收）：活动区原地刷新时光标跟着跳；收工时还回来。
        ratatui::crossterm::execute!(io::stdout(), ratatui::crossterm::cursor::Hide)?;
        while !turn.done {
            // 这一轮结束了、有任务在这一轮里结束：核心要叫醒她的，同一刻就开下一轮；等过这么久没开的就是不叫了。
            match (turn.lingering, lingered) {
                (true, None) => lingered = Some(Instant::now()),
                (true, Some(since)) if since.elapsed() >= LINGER => break,
                (false, _) => lingered = None,
                _ => {}
            }
            // 等到一件，再把已经到了的都办完才画：接上会话补发的那一页一件一件来，每件画一帧的话，以前弹过、已经答了的
            // 确认框会闪一下（2026-10-20 项目主人验收）。
            let first = match self.incoming.recv_timeout(Duration::from_millis(spinner)) {
                Ok(first) => Some(first),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
            let rest: Vec<Incoming> = self.incoming.try_iter().take(BATCH).collect();
            if first
                .into_iter()
                .chain(rest)
                .any(|incoming| self.take(turn, incoming))
            {
                break;
            }
            turn.look.apply(&self.config);
            let (width, height) = size();
            let frame =
                usize::try_from(started.elapsed().as_millis() / u128::from(spinner)).unwrap_or(0);
            let view = self.view(turn, width, height, frame);
            // 不印运行状态行（词、用时）：shell 里只要转圈（2026-10-20 项目主人验收：不要文字进度条）。
            let lines = view.lines(&turn.transcript, turn.scope(), true);
            // 开着的抽屉画在最下面，最多占一半高（图纸「确认和提问」）。
            let drawer = turn
                .asking
                .height(&self.config, width, height / 2)
                .unwrap_or(0);
            // 活动区最多占终端高减一行：长过的只露最新的那一截，这一轮结束时整段印出来（「怎么印」第 2 条）。
            let room = usize::from(height.saturating_sub(1 + drawer)).max(1);
            let tail = lines.len().saturating_sub(room);
            let mut live = self.painter.lines(width, &lines[tail..])?;
            if drawer > 0 {
                let (asking, config) = (&mut turn.asking, &self.config);
                live.extend(self.painter.render(width, drawer, |frame| {
                    asking.draw(frame, frame.area(), config)
                })?);
            }
            let settled = match banner(turn).filter(|_| !bannered) {
                Some(line) => {
                    bannered = true;
                    self.painter.lines(width, &[Line::default(), line])?
                }
                None => Vec::new(),
            };
            self.painter.frame(&settled, &live)?;
            // 新开了一个抽屉：弹「在等你确认」「在等你回答」。
            let current = turn.asking.current();
            if let Some((call, question)) = &current
                && drawer_seen.as_ref() != Some(call)
            {
                self.notifier.tell(if *question {
                    notify::Event::Asking
                } else {
                    notify::Event::Approving
                });
            }
            drawer_seen = current.map(|(call, _)| call);
            self.send_out()?;
        }
        // 收工以前读按键的线程已经读走、还没处理的字也收下：先让它停下，再把排着的拿完（实测：答得快时最后几个字会丢）。
        self.reader.pause();
        for event in self.incoming.try_iter() {
            if let Incoming::Key(event) = event {
                turn.keep_typed(&event);
            }
        }
        self.reader.resume();
        // 定下来：整段照最后的样子印一遍（做完收起的照配置收起），留在滚动区。
        let (width, height) = size();
        let view = self.view(turn, width, height, 0);
        let lines = view.lines(&turn.transcript, turn.scope(), turn.todos_changed);
        let settled = self.painter.lines(width, &lines)?;
        self.painter.finish()?;
        self.painter.frame(&settled, &[])?;
        ratatui::crossterm::execute!(io::stdout(), ratatui::crossterm::cursor::Show)?;
        match turn.ended {
            Some(EndReason::Completed) => self.notifier.tell(notify::Event::Replied),
            Some(EndReason::Error) => self.notifier.tell(notify::Event::Failed),
            _ => {}
        }
        self.send_out()
    }

    /// 办一件：核心来的交给这一轮，按键先看焦点再交给这一轮。交回 `true` 是不等了（按了两下 Ctrl+C）。
    fn take(&mut self, turn: &mut Turn, incoming: Incoming) -> bool {
        match incoming {
            Incoming::Core(update) => {
                turn.core(update, &mut self.config, &self.core);
                false
            }
            Incoming::Key(event) => {
                match event {
                    Event::FocusGained => self.notifier.focus(true),
                    Event::FocusLost => self.notifier.focus(false),
                    _ => {}
                }
                turn.key(&event, &self.core, &self.config)
            }
        }
    }

    /// 等配色那几项读回来（最多半秒），照它挑好：只印定下来的几行时没有主循环替它等。
    pub fn style(&mut self, turn: &mut Turn) {
        let until = Instant::now() + Duration::from_millis(500);
        while !turn.look.ready() {
            let left = until.saturating_duration_since(Instant::now());
            match self.incoming.recv_timeout(left) {
                Ok(Incoming::Core(update)) => turn.core(update, &mut self.config, &self.core),
                Ok(Incoming::Key(_)) => {}
                Err(_) => break,
            }
        }
        turn.look.apply(&self.config);
    }

    /// 几行定下来的直接印（后台命令结束的那几行）。
    ///
    /// # Errors
    ///
    /// 写终端失败。
    pub fn print(&mut self, turn: &Turn) -> io::Result<()> {
        let (width, height) = size();
        let view = self.view(turn, width, height, 0);
        let lines = view.lines(&turn.transcript, Scope::All, false);
        let settled = self.painter.lines(width, &lines)?;
        self.painter.frame(&settled, &[])
    }

    /// 收工：关焦点报告；原始模式在丢掉时关。
    ///
    /// # Errors
    ///
    /// 写终端失败。
    pub fn finish(self) -> io::Result<()> {
        ratatui::crossterm::execute!(io::stdout(), DisableFocusChange)
    }

    fn view<'a>(&'a self, turn: &'a Turn, width: u16, height: u16, frame: usize) -> View<'a> {
        View {
            config: &self.config,
            human: &turn.human,
            media: &self.media,
            board: &turn.board,
            width,
            height,
            frame,
        }
    }

    /// 攒着的通知（OSC 序列）写给终端。
    fn send_out(&mut self) -> io::Result<()> {
        let mut out = io::stdout();
        for sequence in self.notifier.outbox() {
            out.write_all(sequence.as_bytes())?;
        }
        out.flush()
    }
}

/// 终端多宽多高，问不到的照 80×24。
fn size() -> (u16, u16) {
    ratatui::crossterm::terminal::size().unwrap_or((80, 24))
}

/// 问终端的字色、底色和 16 色（在开始读按键以前问，回话不让读按键读走），装上，交回底色深不深（明暗自动照它）。
fn terminal_colors() -> Option<bool> {
    let ground = crate::theme::terminal_colors::probe_ground();
    let colors = crate::theme::terminal_colors::probe();
    let depth = crate::theme::Depth::detect(|name| std::env::var(name).ok());
    crate::theme::terminal_colors::install(colors, ground, depth);
    ground.map(|g| g.dark)
}

/// 系统通知：照全屏的做法，提示音放在同一个缓存目录。
fn notifier(config: &Config) -> Notifier {
    let cache = miyu_client::places::cache_root(&miyu_client::places::Env::current()).ok();
    let sounds = cache.map(|root| root.join("tui").join("sounds"));
    Notifier::new(
        config.notify.clone(),
        config.text.notify.clone(),
        |name| std::env::var(name).ok(),
        sounds,
    )
}

/// 原始模式：收工、出错都关上，不把 shell 留在原始模式里。
struct RawMode;

impl Drop for RawMode {
    fn drop(&mut self) {
        if ratatui::crossterm::terminal::disable_raw_mode().is_err() {}
    }
}
