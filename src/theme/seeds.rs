//! 引导里刚选、人格还没存的头像文件取候选主色，放在后台（蓝图 `tui.md`「主题」的「跟随头像的配色」第 1 条）：可能是几千
//! 像素的照片，解码要一两百毫秒，在界面线程里做会卡一下。一张只取一次，照键（文件的路径）记着。取法是核心的那一份门面
//! （`miyu_client::seeds`），存好的人格的候选由核心给（2026-10-14 项目主人定：头像取色归核心）。取好了叫醒主循环，
//! 不每一拍醒来看（蓝图「空闲不醒」）。

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};

use super::wallpaper::Wake;

/// 取过的、在取的。
pub struct Seeds {
    known: HashMap<String, Vec<u32>>,
    pending: HashSet<String>,
    tx: Sender<(String, Vec<u32>)>,
    rx: Receiver<(String, Vec<u32>)>,
    /// 取好了叫醒主循环（`main.rs` 装上）。
    wake: Option<Wake>,
}

impl std::fmt::Debug for Seeds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Seeds")
            .field("known", &self.known)
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

impl Default for Seeds {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            known: HashMap::new(),
            pending: HashSet::new(),
            tx,
            rx,
            wake: None,
        }
    }
}

impl Seeds {
    /// 取好了怎么叫醒主循环（`main.rs` 装上）。
    pub fn set_wake(&mut self, wake: Wake) {
        self.wake = Some(wake);
    }

    /// 这张图的候选主色（好的在前）：取过的交回（取不出来的是空的）；没取的在后台取，取好以前是 `None`。
    pub fn of(&mut self, key: &str, path: &Path) -> Option<Vec<u32>> {
        self.collect();
        if let Some(seeds) = self.known.get(key) {
            return Some(seeds.clone());
        }
        if self.pending.insert(key.to_string()) {
            let (tx, key, path) = (self.tx.clone(), key.to_string(), path.to_path_buf());
            let wake = self.wake.clone();
            std::thread::spawn(move || {
                let seeds = std::fs::read(&path)
                    .map(|b| of_image(&b))
                    .unwrap_or_default();
                // 界面已经关了的，取好了也没人收。
                if tx.send((key, seeds)).is_ok()
                    && let Some(wake) = wake
                {
                    wake();
                }
            });
        }
        None
    }

    /// 收下后台取好的。
    fn collect(&mut self) {
        while let Ok((key, seed)) = self.rx.try_recv() {
            self.pending.remove(&key);
            self.known.insert(key, seed);
        }
    }
}

/// 一张图的字节交几个候选主色 `0xRRGGBB`（好的在前）：核心的门面，和核心给人格算的同一份；读不成的是空的。
pub fn of_image(bytes: &[u8]) -> Vec<u32> {
    miyu_client::seeds::from_image(bytes)
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.argb() & 0x00ff_ffff)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, mpsc};
    use std::time::Duration;

    use super::Seeds;

    #[test]
    fn a_picture_is_read_in_the_background_once() {
        let dir = std::env::temp_dir().join(format!("miyu-tui-seeds-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("green.png");
        image::RgbaImage::from_pixel(8, 8, image::Rgba([0x30, 0xb0, 0x40, 255]))
            .save(&file)
            .unwrap();
        let mut seeds = Seeds::default();
        let (ring, rang) = mpsc::channel();
        seeds.set_wake(Arc::new(move || if ring.send(()).is_err() {}));
        assert_eq!(seeds.of("v1", &file), None, "第一次在后台取");
        rang.recv_timeout(Duration::from_secs(5))
            .expect("取好了叫醒主循环");
        let got = seeds.of("v1", &file).expect("叫醒以后拿得到");
        let top = got[0];
        assert!((top >> 8) & 0xff > (top >> 16) & 0xff, "绿的：{top:06x}");
        std::fs::remove_file(&file).unwrap();
        assert_eq!(seeds.of("v1", &file), Some(got), "取过的不再读文件");
        std::fs::remove_dir(&dir).unwrap();
    }
}
