//! 头像取主色放在后台（蓝图 `tui.md`「主题」的「跟随头像的配色」第 1 条）：引导里填的头像可能是几千像素的照片，解码要
//! 一两百毫秒，在界面线程里做会卡一下。一张只取一次，照键（头像的版本、本机文件的路径）记着。

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};

/// 取过的、在取的。
#[derive(Debug)]
pub struct Seeds {
    known: HashMap<String, Option<u32>>,
    pending: HashSet<String>,
    tx: Sender<(String, Option<u32>)>,
    rx: Receiver<(String, Option<u32>)>,
}

impl Default for Seeds {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            known: HashMap::new(),
            pending: HashSet::new(),
            tx,
            rx,
        }
    }
}

impl Seeds {
    /// 这张图的主色：取过的交回（取不出来的是 `None`）；没取的在后台取，取好以前是 `None`。
    pub fn of(&mut self, key: &str, path: &Path) -> Option<u32> {
        self.collect();
        if let Some(seed) = self.known.get(key) {
            return *seed;
        }
        if self.pending.insert(key.to_string()) {
            let (tx, key, path) = (self.tx.clone(), key.to_string(), path.to_path_buf());
            std::thread::spawn(move || {
                // 界面已经关了的，取好了也没人收。
                if tx.send((key, super::persona::seed_of_file(&path))).is_err() {}
            });
        }
        None
    }

    /// 有在取的：界面每一拍醒来看一次（`app/deadline.rs`）。
    pub fn busy(&self) -> bool {
        !self.pending.is_empty()
    }

    /// 收下后台取好的。
    fn collect(&mut self) {
        while let Ok((key, seed)) = self.rx.try_recv() {
            self.pending.remove(&key);
            self.known.insert(key, seed);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

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
        assert_eq!(seeds.of("v1", &file), None, "第一次在后台取");
        assert!(seeds.busy());
        let until = Instant::now() + Duration::from_secs(5);
        let mut seed = None;
        while seed.is_none() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
            seed = seeds.of("v1", &file);
        }
        let seed = seed.expect("取好了");
        assert!((seed >> 8) & 0xff > (seed >> 16) & 0xff, "绿的：{seed:06x}");
        assert!(!seeds.busy());
        std::fs::remove_file(&file).unwrap();
        assert_eq!(seeds.of("v1", &file), Some(seed), "取过的不再读文件");
        std::fs::remove_dir(&dir).unwrap();
    }
}
