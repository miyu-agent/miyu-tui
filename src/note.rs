//! 看不见的失败记一行（蓝图 `tui.md`「主题」的「跟随壁纸」第 3 条）：机器缓存目录下 `tui/tui.log` 追加时刻和一句话，
//! 界面上不弹（起不来的功能有退路，只是排查用）。

use std::fs::{self, OpenOptions};
use std::io::Write;

use miyu_client::places::Env;

/// 记一行；缓存目录找不到、写不进去的就算了。
pub fn note(text: &str) {
    let Ok(root) = miyu_client::places::cache_root(&Env::current()) else {
        return;
    };
    let dir = root.join("tui");
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let time = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
    let line = format!("{time} {text}\n");
    let opened = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("tui.log"));
    // 写不进去的就算了：只是排查用的一行。
    if let Ok(mut file) = opened
        && file.write_all(line.as_bytes()).is_err()
    {}
}
