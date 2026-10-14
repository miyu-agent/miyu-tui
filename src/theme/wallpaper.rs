//! 跟随壁纸（蓝图 `tui.md`「主题」的「跟随壁纸」，2026-10-14 项目主人定）：取色和网页同一个来处（网页仓库的
//! `miyu-wallpaper`）。先找 matugen 生成的那一份，取它的 `source` 当主色；没有的照桌面找壁纸图，取图的主色（同跟随头像的
//! 取法）。主色照「跟随头像的配色」第 3 条那一套生成。找要跑命令（一家最多等 2 秒），放在后台：起来时找一次，打开配色
//! 列表、选上跟随壁纸时再找一次；壁纸来处的文件变了（`miyu-wallpaper` 的 `Watch`：只用系统的文件通知，起不来的不退回
//! 定时去看、记一行日志）、窗口拿回焦点时（只能靠命令换壁纸的，hyprpaper、swaybg）也再找（2026-10-14 项目主人：换了
//! 壁纸要自动跟着变，绝对不轮询）。都找不到的配色里不列它，选着它的照 tokyonight。
//!
//! `MIYU_TUI_DESKTOP_WALLPAPER=0` 不看桌面、只认 matugen 那一份（测试里这么设：不碰跑测试的这台电脑的桌面）；
//! `MIYU_TUI_WALLPAPER_WATCH=0` 不盯文件、只在拿回焦点时找（测拿回焦点那一条、看有没有偷偷定时去看）。

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

use miyu_wallpaper::watch::Watch;
use serde_json::Value;

/// 找到了什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Found {
    /// 还没找过。
    #[default]
    Unknown,
    /// 找过了，没有能取色的。
    Nothing,
    /// 主色。
    Seed(u32),
}

/// 叫醒主循环（盯着的文件变了，下一拍再找）。
pub type Wake = Arc<dyn Fn() + Send + Sync>;

/// 后台找壁纸。
pub struct Wallpaper {
    found: Found,
    looking: bool,
    tx: Sender<(Found, Vec<PathBuf>)>,
    rx: Receiver<(Found, Vec<PathBuf>)>,
    /// 盯着壁纸来处的文件；还没找过、起不来的是 `None`。
    watch: Option<Watch>,
    /// 盯不起来：记过一行了，不再试。
    watch_failed: bool,
    /// 盯着的文件变了，还没再找。
    changed: Arc<AtomicBool>,
    /// 叫醒主循环。
    wake: Option<Wake>,
}

impl std::fmt::Debug for Wallpaper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wallpaper")
            .field("found", &self.found)
            .field("looking", &self.looking)
            .field("watching", &self.watch.is_some())
            .finish_non_exhaustive()
    }
}

impl Default for Wallpaper {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            found: Found::Unknown,
            looking: false,
            tx,
            rx,
            watch: None,
            watch_failed: false,
            changed: Arc::new(AtomicBool::new(false)),
            wake: None,
        }
    }
}

/// 盯不盯壁纸的文件：`MIYU_TUI_WALLPAPER_WATCH=0` 的不盯。
fn watching() -> bool {
    std::env::var("MIYU_TUI_WALLPAPER_WATCH").map_or(true, |v| v != "0")
}

impl Wallpaper {
    /// 在后台找一次（正在找的不再找）。`dark` 是现在用深色那一份（GNOME、KDE 深浅可以各设一张）。
    pub fn look(&mut self, dark: bool) {
        // 单元测试里不找：不碰跑测试的这台电脑（伪终端里的测试照 `MIYU_TUI_DESKTOP_WALLPAPER` 只认 matugen 那一份）。
        if cfg!(test) {
            self.found = Found::Nothing;
            return;
        }
        if self.looking {
            return;
        }
        self.looking = true;
        let tx = self.tx.clone();
        let desktop = std::env::var("MIYU_TUI_DESKTOP_WALLPAPER").map_or(true, |v| v != "0");
        std::thread::spawn(move || {
            // 界面已经关了的，找好了也没人收。
            if tx.send(probe(dark, desktop)).is_err() {}
        });
    }

    /// 文件变了时怎么叫醒主循环（`main.rs` 装上）。
    pub fn set_wake(&mut self, wake: Wake) {
        self.wake = Some(wake);
    }

    /// 盯着的文件变过：交回 `true` 一次（再找一次）。
    pub fn take_changed(&self) -> bool {
        self.changed.swap(false, Ordering::Relaxed)
    }

    /// 收下后台找好的、照找到的换盯着的文件：变了的交回 `true`（要重挑一套颜色）。
    pub fn collect(&mut self) -> bool {
        let mut changed = false;
        while let Ok((found, targets)) = self.rx.try_recv() {
            self.looking = false;
            changed |= found != self.found;
            self.found = found;
            self.watch_targets(targets);
        }
        changed
    }

    /// 照这一份清单盯：第一次开始盯，以后换清单；起不来的记一行，以后只靠拿回焦点时找。
    fn watch_targets(&mut self, targets: Vec<PathBuf>) {
        if self.watch_failed || !watching() {
            return;
        }
        if let Some(watch) = &self.watch {
            if let Err(e) = watch.set(targets) {
                crate::note::note(&format!("跟随壁纸：换盯着的文件没成：{e}"));
            }
            return;
        }
        let (changed, wake) = (self.changed.clone(), self.wake.clone());
        let started = Watch::start(targets, move || {
            changed.store(true, Ordering::Relaxed);
            if let Some(wake) = &wake {
                wake();
            }
        });
        match started {
            Ok(watch) => self.watch = Some(watch),
            Err(e) => {
                self.watch_failed = true;
                crate::note::note(&format!(
                    "跟随壁纸：盯不了壁纸的文件，只在拿回焦点时再找：{e}"
                ));
            }
        }
    }

    /// 主色；没找到、还没找好的是 `None`。
    pub fn seed(&self) -> Option<u32> {
        match self.found {
            Found::Seed(seed) => Some(seed),
            _ => None,
        }
    }

    /// 配色里列不列「跟随壁纸」：找到了能取色的才列。
    pub fn available(&self) -> bool {
        self.seed().is_some()
    }

    /// 在找：界面每一拍醒来看一次（`app/deadline.rs`）。
    pub fn busy(&self) -> bool {
        self.looking
    }
}

/// 找：matugen 那一份优先，没有的（`desktop` 开着时）照桌面找壁纸图。一并交回要盯的文件：找到了图的照它的（图本身加上
/// 各家的清单），不然照这台电脑的清单；不看桌面的只盯 matugen 那一份。
fn probe(dark: bool, desktop: bool) -> (Found, Vec<PathBuf>) {
    let list = || {
        if desktop {
            miyu_wallpaper::watch::watch_list()
        } else {
            miyu_wallpaper::matugen::output_path().into_iter().collect()
        }
    };
    let matugen = miyu_wallpaper::matugen::find()
        .and_then(|m| m.read().ok())
        .and_then(|json| source_of(&json));
    if let Some(seed) = matugen {
        return (Found::Seed(seed), list());
    }
    if !desktop {
        return (Found::Nothing, list());
    }
    let Some(wallpaper) = miyu_wallpaper::find(dark).ok().flatten() else {
        return (Found::Nothing, list());
    };
    let seed = wallpaper
        .read()
        .ok()
        .and_then(|bytes| super::persona::seed_of_bytes(&bytes));
    (seed.map_or(Found::Nothing, Found::Seed), wallpaper.watch)
}

/// matugen 那一份（网页仓库 `web/matugen/miyu-web.json` 模板生成的）的 `source`：`#rrggbb`。
fn source_of(json: &[u8]) -> Option<u32> {
    let got: Value = serde_json::from_slice(json).ok()?;
    got["source"].as_str().and_then(super::persona::hex)
}

#[cfg(test)]
mod tests {
    use super::source_of;

    #[test]
    fn matugens_source_is_the_main_color() {
        let json =
            r##"{"note": "生成的", "source": "#2fab34", "light": {}, "dark": {}}"##.as_bytes();
        assert_eq!(source_of(json), Some(0x2f_ab34));
        assert_eq!(
            source_of(br#"{"source": "{{colors.source_color.default.hex}}"}"#),
            None,
            "没生成的模板不算"
        );
        assert_eq!(source_of(b"not json"), None);
    }
}
