//! 伪终端里的端到端测试共用的（蓝图 `tui.md`「守着它的」）：临时的数据根、照剧本回话的核心（和 `miyu-endpoint` 的测试
//! 同一套：`Core::new` 加剧本，开在数据根的套接字上）、伪终端里起的真界面、读屏幕、回终端的查询。
//!
//! 不走网络、不花钱，同一个剧本每次一样。界面照 `MIYU_HOME` 找到数据根里记着的套接字、令牌，自己连上去。

#![allow(dead_code, reason = "几个测试各用其中一部分")]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

/// 窗口多大。
pub const COLS: u16 = 100;
/// 窗口多高。
pub const ROWS: u16 = 40;
/// 等一样东西出现最多等多久。
pub const WAIT: Duration = Duration::from_secs(15);

/// 钉住的那份核心检出里出厂的资源目录（工具的显示名、核心给人看的字；设计 32 第四节第 3 条）。
fn resources() -> PathBuf {
    Path::new(miyu_client::WORKSPACE_ROOT).join("resources")
}

mod core;
mod home;

pub use home::Home;

/// 伪终端里跑着的界面。
/// 一个颜色的三个字节。
type Rgb = (u8, u8, u8);

pub struct Tui {
    screen: vt100::Parser,
    /// 终端几列宽。
    cols: u16,
    /// 录着的界面写出来的原样字节（[`Tui::record`]）。
    recording: Option<Vec<u8>>,
    /// 问终端 16 色（OSC 4）时答什么（[`Tui::answer_colors`]）；不设的不答，像不认 OSC 4 的终端。
    colors: Option<[Rgb; 16]>,
    /// 问终端字色、底色（OSC 10、11）时答什么（[`Tui::answer_ground`]）；不设的不答。
    ground: Option<(Rgb, Rgb)>,
    bytes: Receiver<Vec<u8>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Box<dyn Child + Send + Sync>,
    _master: Box<dyn MasterPty + Send>,
    /// 起进程以前的那一刻（量首帧，[`Tui::wait_for_at`]）。
    pub started: Instant,
}

impl Tui {
    fn spawn(home: &Path, work: &Path, lang: &str, env: &[(&str, &str)], args: &[&str]) -> Tui {
        Self::program(home, work, lang, env, args, env!("CARGO_BIN_EXE_miyu-tui"))
    }

    /// 在伪终端运行指定程序，仅供隔离的 herdr 恢复实测。
    pub fn program(
        home: &Path,
        work: &Path,
        lang: &str,
        env: &[(&str, &str)],
        args: &[&str],
        program: &str,
    ) -> Tui {
        Self::program_sized((home, work), lang, env, args, program, COLS)
    }

    /// 同 [`Tui::program`]，终端开 `cols` 列宽。
    fn program_sized(
        (home, work): (&Path, &Path),
        lang: &str,
        env: &[(&str, &str)],
        args: &[&str],
        program: &str,
        cols: u16,
    ) -> Tui {
        let pty = native_pty_system()
            .openpty(PtySize {
                rows: ROWS,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("开得了伪终端");
        let mut command = CommandBuilder::new(program);
        command.args(args);
        command.cwd(work);
        for name in [
            "MIYU_CORE_BIN",
            "MIYU_TUI_FRAME_LOG",
            "TMUX",
            "KITTY_WINDOW_ID",
            "LC_ALL",
            "LC_MESSAGES",
        ] {
            command.env_remove(name);
        }
        for (name, _) in std::env::vars().filter(|(k, _)| k.starts_with("HERDR_")) {
            command.env_remove(name);
        }
        command.env("MIYU_HOME", home);
        // 不切跑测试这台机器上的输入法（`ime.rs`）。
        command.env("MIYU_TUI_IME", "0");
        // 新会话照无人格开、不弹人格框（`app/new_session.rs`，`-` 是无人格；出厂不再带人格，核心 P-4）；测人格框的测试
        // 把它设成空的，要用哪个人格的测试设它的编号。
        command.env("MIYU_TUI_PERSONA", "-");
        // 新会话照功能全开、不弹预设框；测预设框的测试把它设成空的。
        command.env("MIYU_TUI_PRESET", "full");
        command.env("MIYU_RESOURCES", resources());
        command.env("XDG_CACHE_HOME", home.join("cache"));
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        // 跟随壁纸只认测试家目录里的 matugen 那一份，不看跑测试的这台电脑的桌面。
        command.env("MIYU_TUI_DESKTOP_WALLPAPER", "0");
        command.env("LANG", lang);
        for (name, value) in env {
            command.env(name, value);
        }
        let started = Instant::now();
        let child = pty.slave.spawn_command(command).expect("起得来界面");
        drop(pty.slave);
        let mut reader = pty.master.try_clone_reader().expect("读得了");
        let writer = Arc::new(Mutex::new(pty.master.take_writer().expect("写得了")));
        let (send, bytes) = channel();
        std::thread::spawn(move || {
            let mut buf = [0u8; 65536];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 || send.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
        Tui {
            screen: vt100::Parser::new(ROWS, cols, 0),
            cols,
            recording: None,
            colors: None,
            ground: None,
            bytes,
            writer,
            child,
            _master: pty.master,
            started,
        }
    }

    /// 写给界面（按键、粘贴）。
    pub fn send(&mut self, bytes: &[u8]) {
        let mut writer = self.writer.lock().unwrap_or_else(PoisonError::into_inner);
        writer.write_all(bytes).expect("写得进去");
        writer.flush().expect("写得进去");
    }

    /// 按一个键，停一下让界面画完：单独的 Esc 后面紧跟着字，终端会读成 Alt 加那个字，真人按键之间总会停一下。
    pub fn key(&mut self, bytes: &[u8]) {
        self.send(bytes);
        self.pump(Duration::from_millis(250));
    }

    /// 打一串字（不回车）。
    pub fn type_text(&mut self, text: &str) {
        self.send(text.as_bytes());
        self.pump(Duration::from_millis(150));
    }

    /// 打一句、回车。
    pub fn say(&mut self, text: &str) {
        self.type_text(text);
        self.send(b"\r");
    }

    /// 收界面写出来的，最多 `wait`：喂给屏幕，顺手回终端的查询（像真终端那样答：认得键盘协议、没有图的协议）。
    pub fn pump(&mut self, wait: Duration) {
        let end = Instant::now() + wait;
        loop {
            let left = end.saturating_duration_since(Instant::now());
            match self.bytes.recv_timeout(left) {
                Ok(data) => self.feed(&data),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn feed(&mut self, data: &[u8]) {
        let has = |needle: &[u8]| data.windows(needle.len()).any(|w| w == needle);
        if has(b"_Gi=31") {
            self.send(b"\x1b[?62c\x1b[6;20;10t\x1b[0n");
            return;
        }
        if has(b"\x1b[?u") {
            self.send(b"\x1b[?0u");
        }
        // 先答颜色、再答 DA1：真终端照问的先后答。
        let rgb = |(r, g, b): Rgb| format!("rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}");
        if let Some((fg, bg)) = self.ground.filter(|_| has(b"\x1b]10;?")) {
            let answer = format!("\x1b]10;{}\x1b\\\x1b]11;{}\x1b\\", rgb(fg), rgb(bg));
            self.send(answer.as_bytes());
        }
        if let Some(colors) = self.colors.filter(|_| has(b"\x1b]4;")) {
            let answer: String = colors
                .iter()
                .enumerate()
                .map(|(n, (r, g, b))| {
                    format!("\x1b]4;{n};rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}\x1b\\")
                })
                .collect();
            self.send(answer.as_bytes());
        }
        if has(b"\x1b[c") {
            self.send(b"\x1b[?62c");
        }
        if has(b"\x1b[6n") {
            self.send(b"\x1b[1;1R");
        }
        if let Some(recording) = &mut self.recording {
            recording.extend_from_slice(data);
        }
        self.screen.process(data);
    }

    /// 界面起来问终端 16 色时照这一组答（kitty 这类终端会答）：要在第一次读屏以前设。
    pub fn answer_colors(&mut self, colors: [Rgb; 16]) {
        self.colors = Some(colors);
    }

    /// 界面起来问终端字色、底色时照这一对答：要在第一次读屏以前设。
    pub fn answer_ground(&mut self, fg: Rgb, bg: Rgb) {
        self.ground = Some((fg, bg));
    }

    /// 从现在起录下界面写出来的每一个字节：看中间有没有闪过哪一帧（读屏只看得到最后那一帧）。
    pub fn record(&mut self) {
        self.recording = Some(Vec::new());
    }

    /// 录下的字节，停止录。
    pub fn recorded(&mut self) -> Vec<u8> {
        self.recording.take().unwrap_or_default()
    }

    /// 从现在起录，交回现在的屏幕：交给 [`frames`] 一帧一帧重放录下的字节（只读屏幕看不到中间闪过的帧）。
    pub fn record_from_here(&mut self) -> vt100::Parser {
        let mut start = vt100::Parser::new(ROWS, self.cols, 0);
        start.process(&self.screen.screen().contents_formatted());
        self.record();
        start
    }

    /// 屏幕上的字，一行一行。
    pub fn lines(&self) -> Vec<String> {
        self.screen
            .screen()
            .rows(0, self.cols)
            .map(|l| l.trim_end().to_string())
            .collect()
    }

    /// 屏幕上有没有这串字。
    pub fn shows(&self, text: &str) -> bool {
        self.lines().iter().any(|l| l.contains(text))
    }

    /// 等到屏幕上出现这串字，最多 [`WAIT`]；等不到的把屏幕打出来再报错。
    pub fn wait_for(&mut self, text: &str) {
        let end = Instant::now() + WAIT;
        while !self.shows(text) {
            assert!(
                Instant::now() < end,
                "等不到「{text}」，屏幕是：\n{}",
                self.lines().join("\n")
            );
            self.pump(Duration::from_millis(50));
        }
    }

    /// 等到屏幕上有一整行（去掉两头的空白）就是 `text`，最多 [`WAIT`]。别处带着这几个字的不算：比如首页的工作目录放不下时
    /// 以 `…` 打头（macOS 的临时目录很长），等她吐出的那个 `…` 就要只认整行。
    pub fn wait_for_line(&mut self, text: &str) {
        let end = Instant::now() + WAIT;
        while !self.lines().iter().any(|l| l.trim() == text) {
            assert!(
                Instant::now() < end,
                "等不到整行「{text}」，屏幕是：\n{}",
                self.lines().join("\n")
            );
            self.pump(Duration::from_millis(50));
        }
    }

    /// 配置页主菜单上挪到名字是 `name` 的那一项、回车进去：照选中那一行（左边的 `┃`）认，不数第几项（2026-10-14 主菜单加了
    /// 「外观」，数行的全错一位）。清单没读到以前菜单短几项，挪过头的往回找。
    pub fn open_menu(&mut self, name: &str) {
        self.wait_for(name);
        let picked = |tui: &Self| {
            tui.lines().iter().any(|l| {
                l.split_once('┃')
                    .and_then(|(_, rest)| rest.split_whitespace().next())
                    == Some(name)
            })
        };
        for key in [b"j", b"k"] {
            for _ in 0..12 {
                if picked(self) {
                    self.key(b"\r");
                    return;
                }
                self.key(key);
            }
        }
        panic!("主菜单上挪不到「{name}」：\n{}", self.lines().join("\n"));
    }

    /// 光标在哪：（行, 列），从 0 数。
    pub fn cursor(&self) -> (u16, u16) {
        self.screen.screen().cursor_position()
    }

    /// 一格里写的字（宽字的后半格是空的）。
    pub fn cell(&self, row: u16, col: u16) -> String {
        self.screen
            .screen()
            .cell(row, col)
            .map(|c| c.contents().to_string())
            .unwrap_or_default()
    }

    /// 一格的字色。
    pub fn foreground(&self, row: u16, col: u16) -> vt100::Color {
        self.screen
            .screen()
            .cell(row, col)
            .map_or(vt100::Color::Default, vt100::Cell::fgcolor)
    }

    /// 一格的底色。
    pub fn background(&self, row: u16, col: u16) -> vt100::Color {
        self.screen
            .screen()
            .cell(row, col)
            .map_or(vt100::Color::Default, vt100::Cell::bgcolor)
    }

    /// 一格的字带不带下划线。
    pub fn underlined(&self, row: u16, col: u16) -> bool {
        self.screen
            .screen()
            .cell(row, col)
            .is_some_and(vt100::Cell::underline)
    }

    /// 输入框里那一行的字（提示符后面的）。
    pub fn input(&self) -> String {
        self.lines()
            .iter()
            .rev()
            .find(|l| l.contains('❯') && l.contains('│'))
            .and_then(|l| l.split_once('❯'))
            .map(|(_, rest)| rest.trim_end_matches('│').trim().to_string())
            .unwrap_or_default()
    }
}

impl Tui {
    /// 等到屏幕上出现这串字，交回带出它的那一块字节到的时刻：一块一块地看，不等满一拍（量首帧用）。
    pub fn wait_for_at(&mut self, text: &str) -> Instant {
        let end = Instant::now() + WAIT;
        while !self.shows(text) {
            let left = end.saturating_duration_since(Instant::now());
            let data = self.bytes.recv_timeout(left).unwrap_or_else(|_| {
                panic!("等不到「{text}」，屏幕是：\n{}", self.lines().join("\n"))
            });
            let at = Instant::now();
            self.feed(&data);
            if self.shows(text) {
                return at;
            }
        }
        Instant::now()
    }

    /// 界面的进程号。
    pub fn pid(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// 界面这个进程的所有线程到现在一共切进来几次（自愿的加被抢的）：量空闲时醒没醒。只有 Linux 读得到
    /// （`/proc/<进程>/task/*/status`），别的系统交回 `None`。
    pub fn wakeups(&self) -> Option<u64> {
        let pid = self.child.process_id()?;
        let tasks = std::fs::read_dir(format!("/proc/{pid}/task")).ok()?;
        let mut total = 0;
        for task in tasks.flatten() {
            let Ok(status) = std::fs::read_to_string(task.path().join("status")) else {
                continue;
            };
            total += status
                .lines()
                .filter(|l| l.contains("voluntary_ctxt_switches:"))
                .filter_map(|l| l.split_whitespace().last()?.parse::<u64>().ok())
                .sum::<u64>();
        }
        Some(total)
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        drop(self.child.kill());
        drop(self.child.wait());
    }
}

/// 从 `start` 那一屏起，把录下的字节照同步输出的结尾（`CSI ? 2026 l`）一帧一帧放出来，交回每一帧的屏幕，一行一行。
pub fn frames(mut start: vt100::Parser, recorded: &[u8]) -> Vec<Vec<String>> {
    const END: &[u8] = b"\x1b[?2026l";
    let mut out = Vec::new();
    let mut rest = recorded;
    while let Some(at) = rest.windows(END.len()).position(|w| w == END) {
        start.process(&rest[..at + END.len()]);
        out.push(
            start
                .screen()
                .rows(0, COLS)
                .map(|l| l.trim_end().to_string())
                .collect(),
        );
        rest = &rest[at + END.len()..];
    }
    out
}
