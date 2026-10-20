//! 跟随壁纸（蓝图 `tui.md`「主题」的「跟随壁纸」，2026-10-14 项目主人定）：取色和网页同一个来处（门面
//! `miyu_client::wallpaper`，核心施工 X-29 从网页仓库搬进核心仓库）。先找 matugen 生成的那一份，取它的 `source` 当主色；没有的照桌面找壁纸图，取图的主色（同跟随头像的
//! 取法）。主色照「跟随头像的配色」第 3 条那一套生成。找要跑命令（一家最多等 2 秒），放在后台，找好了叫醒主循环：起来时
//! 找一次，打开配色列表、选上跟随壁纸时再找一次；跟随着的，壁纸来处的文件变了（`miyu_client::wallpaper` 的 `Watch`：只用系统的
//! 文件通知，起不来的不退回定时去看、记一行日志）、窗口拿回焦点时（只能靠命令换壁纸的，hyprpaper、swaybg）也再找
//! （2026-10-14 项目主人：换了壁纸要自动跟着变，绝对不轮询）。都找不到的配色里不列它，选着它的照 tokyonight。
//!
//! 图只在跟随着时解（取色）：不跟随的只看找不找得到（配色列表列不列它），一张 4K 的图解开要三十多 MB；解过的照路径、
//! 大小、改动时间记在缓存里（`wallpaper_cache.rs`），同一张下次不再解（2026-10-15 性能对比查出的内存尖峰）。
//!
//! `MIYU_TUI_DESKTOP_WALLPAPER=0` 不看桌面、只认 matugen 那一份（测试里这么设：不碰跑测试的这台电脑的桌面）；
//! `MIYU_TUI_WALLPAPER_WATCH=0` 不盯文件、只在拿回焦点时找（测拿回焦点那一条、看有没有偷偷定时去看）。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

use miyu_client::wallpaper::watch::Watch;
use serde_json::Value;

/// 找到了什么。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Found {
    /// 还没找过。
    #[default]
    Unknown,
    /// 找过了，没有能取色的。
    Nothing,
    /// 找到了壁纸图，还没取色（没跟随壁纸时不解图）。
    Image,
    /// 能当主色的几个候选，好的在前（matugen 那一份只有它的 `source` 一个）。
    Seeds(Vec<u32>),
}

/// 叫醒主循环（盯着的文件变了，下一拍再找）。
pub type Wake = Arc<dyn Fn() + Send + Sync>;

/// 后台找壁纸。
pub struct Wallpaper {
    found: Found,
    looking: bool,
    /// 在找的时候又要找的：深色那一份、要不要解图；找完接着找。
    queued: Option<(bool, bool)>,
    /// 跟随着壁纸：才盯文件。
    following: bool,
    /// 上一次找到的那一家要盯的文件。
    targets: Vec<PathBuf>,
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
            queued: None,
            following: false,
            targets: Vec::new(),
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
    /// 在后台找一次（正在找的，找完接着找）。`dark` 是现在用深色那一份（GNOME、KDE 深浅可以各设一张）；`decode` 是找到了
    /// 图要不要解开取色（跟随着壁纸才解）。
    pub fn look(&mut self, dark: bool, decode: bool) {
        // 单元测试里不找：不碰跑测试的这台电脑（伪终端里的测试照 `MIYU_TUI_DESKTOP_WALLPAPER` 只认 matugen 那一份）。
        if cfg!(test) {
            self.found = Found::Nothing;
            return;
        }
        if self.looking {
            let decode = decode || self.queued.is_some_and(|(_, d)| d);
            self.queued = Some((dark, decode));
            return;
        }
        self.looking = true;
        let (tx, wake) = (self.tx.clone(), self.wake.clone());
        let desktop = std::env::var("MIYU_TUI_DESKTOP_WALLPAPER").map_or(true, |v| v != "0");
        std::thread::spawn(move || {
            // 界面已经关了的，找好了也没人收。
            if tx.send(probe(dark, desktop, decode)).is_ok()
                && let Some(wake) = wake
            {
                wake();
            }
        });
    }

    /// 跟随着壁纸才盯它来处的文件；不跟随了放掉。
    pub fn follow(&mut self, on: bool) {
        if on == self.following {
            return;
        }
        self.following = on;
        if on {
            self.watch_targets(self.targets.clone());
        } else {
            self.watch = None;
        }
    }

    /// 找到了图还没解（刚选上跟随壁纸）。
    pub fn undecoded(&self) -> bool {
        self.found == Found::Image
    }

    /// 先画后连：还没找过的，先照上次记下的候选主色（`app/look.rs`）；找好了照找到的换。
    pub fn restore(&mut self, seeds: &[u32]) {
        if self.found == Found::Unknown && !seeds.is_empty() {
            self.found = Found::Seeds(seeds.to_vec());
        }
    }

    /// 文件变了时怎么叫醒主循环（`main.rs` 装上）。
    pub fn set_wake(&mut self, wake: Wake) {
        self.wake = Some(wake);
    }

    /// 盯着的文件变过：交回 `true` 一次（再找一次）。
    pub fn take_changed(&self) -> bool {
        self.changed.swap(false, Ordering::Relaxed)
    }

    /// 收下后台找好的、照找到的换盯着的文件（跟随着的才盯）：变了的交回 `true`（要重挑一套颜色）。找的时候又要找的接着找。
    pub fn collect(&mut self) -> bool {
        let mut changed = false;
        while let Ok((found, targets)) = self.rx.try_recv() {
            self.looking = false;
            changed |= found != self.found;
            self.found = found;
            if self.following {
                self.watch_targets(targets.clone());
            }
            self.targets = targets;
        }
        if !self.looking
            && let Some((dark, decode)) = self.queued.take()
        {
            self.look(dark, decode);
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

    /// 能当主色的几个候选，好的在前；没找到、还没找好的是空的。
    pub fn seeds(&self) -> &[u32] {
        match &self.found {
            Found::Seeds(seeds) => seeds,
            _ => &[],
        }
    }

    /// 配色里列不列「跟随壁纸」：找到了壁纸图（还没解的也算）、能取色的才列。
    pub fn available(&self) -> bool {
        self.found == Found::Image || !self.seeds().is_empty()
    }
}

/// 找：matugen 那一份优先，没有的（`desktop` 开着时）照桌面找壁纸图，缓存里记着这张的直接用，没记的 `decode` 才解开取色
/// 再记下。一并交回要盯的文件：找到了图的照它的（图本身、找到它的那一家的文件、matugen 那一份），不然只盯 matugen 那一份
/// （`miyu_client::wallpaper` 的 `watch_list`；别家的不盯，后来才设上的靠拿回焦点时再找）。
fn probe(dark: bool, desktop: bool, decode: bool) -> (Found, Vec<PathBuf>) {
    let list = miyu_client::wallpaper::watch::watch_list;
    let matugen = miyu_client::wallpaper::matugen::find()
        .and_then(|m| m.read().ok())
        .and_then(|json| source_of(&json));
    if let Some(seed) = matugen {
        return (Found::Seeds(vec![seed]), list());
    }
    if !desktop {
        return (Found::Nothing, list());
    }
    let Some(wallpaper) = miyu_client::wallpaper::find(dark).ok().flatten() else {
        return (Found::Nothing, list());
    };
    let cache = super::wallpaper_cache::file();
    let found = colors_of(
        &wallpaper.path,
        || wallpaper.read(),
        decode,
        cache.as_deref(),
    );
    (found, wallpaper.watch)
}

/// 找到的壁纸图 `path` 的颜色：缓存 `cache` 里记着这张的直接用；没记的 `decode` 才读（`read`）、解开取色、记下，不然只说
/// 找到了图。
fn colors_of(
    path: &Path,
    read: impl FnOnce() -> std::io::Result<Vec<u8>>,
    decode: bool,
    cache: Option<&Path>,
) -> Found {
    if let Some(seeds) = cache.and_then(|c| super::wallpaper_cache::get(c, path)) {
        return Found::Seeds(seeds);
    }
    if !decode {
        return Found::Image;
    }
    let seeds = read()
        .map(|bytes| super::seeds::of_image(&bytes))
        .unwrap_or_default();
    if seeds.is_empty() {
        return Found::Nothing;
    }
    if let Some(cache) = cache {
        super::wallpaper_cache::put(cache, path, &seeds);
    }
    Found::Seeds(seeds)
}

/// matugen 那一份（网页仓库 `web/matugen/miyu-web.json` 模板生成的）的 `source`：`#rrggbb`。
fn source_of(json: &[u8]) -> Option<u32> {
    let got: Value = serde_json::from_slice(json).ok()?;
    got["source"].as_str().and_then(super::persona::hex)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::path::Path;

    use super::{Found, colors_of, source_of};

    /// 一张绿色的小图的字节。
    fn green() -> Vec<u8> {
        let mut bytes = Vec::new();
        image::RgbaImage::from_pixel(8, 8, image::Rgba([0x30, 0xb0, 0x40, 255]))
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        bytes
    }

    #[test]
    fn the_picture_is_decoded_only_when_following_and_only_once() {
        let dir = std::env::temp_dir().join(format!("miyu-tui-wall-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (picture, cache) = (dir.join("wall.png"), dir.join("tui/wallpaper.json"));
        std::fs::write(&picture, green()).unwrap();
        let reads = Cell::new(0);
        let read = || {
            reads.set(reads.get() + 1);
            std::fs::read(&picture)
        };
        let colors =
            |decode: bool, read| colors_of(&picture, read, decode, Some(Path::new(&cache)));
        assert_eq!(colors(false, &read), Found::Image, "没跟随：只说找到了图");
        assert_eq!(reads.get(), 0, "没跟随的不读图、不解");
        let Found::Seeds(seeds) = colors(true, &read) else {
            panic!("跟随着：解开取色");
        };
        assert_eq!(reads.get(), 1);
        assert_eq!(
            colors(false, &read),
            Found::Seeds(seeds.clone()),
            "记着的直接用"
        );
        assert_eq!(colors(true, &read), Found::Seeds(seeds), "同一张不再解");
        assert_eq!(reads.get(), 1, "只读过一次");
        std::fs::remove_dir_all(&dir).unwrap();
    }

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
