//! 关掉终端窗口：伪终端主端这边的口子全关上，界面那一端挂断（[`Tui::hang_up`]）。

use std::sync::PoisonError;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use portable_pty::{PtySize, native_pty_system};

use super::{ROWS, Tui};

impl Tui {
    /// 关掉终端窗口。主端开着好几个口子（读的、写的、主端本身），一个没关界面那一端就不算挂断：读的那条线程先叫它收手
    /// （换掉收的那一头，再改一下大小让界面重画一次，它读到了交不出去就收工、放下手里那个口子），再放下写的、主端本身。
    /// 之后读不到屏幕了。
    pub fn hang_up(&mut self) {
        self.no_eof_on_close();
        let (_, dead) = channel();
        self.bytes = dead;
        self.resize(ROWS - 1, self.cols);
        std::thread::sleep(Duration::from_secs(1));
        *self.writer.lock().unwrap_or_else(PoisonError::into_inner) = Box::new(std::io::sink());
        let spare = native_pty_system()
            .openpty(PtySize {
                rows: ROWS,
                cols: self.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("开得了伪终端");
        self.master = spare.master;
    }

    /// portable-pty 放下写的那一头时照终端的 VEOF 往里写一个回车加 Ctrl+D（`UnixMasterWriter` 的 drop），界面收到会当按键
    /// 退出，测不到挂断：先把 VEOF 关掉（0），它就什么都不写。界面开着原始模式，VEOF 本来就不起作用。
    fn no_eof_on_close(&self) {
        use rustix::fs::{Mode, OFlags, open};
        use rustix::termios::{OptionalActions, SpecialCodeIndex, tcgetattr, tcsetattr};
        let path = self.master.tty_name().expect("伪终端有名字");
        let tty = open(&path, OFlags::RDWR | OFlags::NOCTTY, Mode::empty()).expect("开得了伪终端");
        let mut termios = tcgetattr(&tty).expect("读得了终端设置");
        termios.special_codes[SpecialCodeIndex::VEOF] = 0;
        tcsetattr(&tty, OptionalActions::Now, &termios).expect("改得了终端设置");
    }

    /// 等界面在 `wait` 以内退出：退出了交回 `true`。
    pub fn gone_within(&mut self, wait: Duration) -> bool {
        let end = Instant::now() + wait;
        while Instant::now() < end {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }
}
