//! 盯后台的进程（`follow.rs`）和 shell 之间的信箱（设计 22 第七节「后台任务和回报」）：一个 shell 一个文件，盯后台的往里
//! 追加要印的，shell 回到提示符时 `report` 一次拿走。不在终端上直接写：前台可能跑着 vim 这类全屏程序。

use std::io::{self, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::{JobEnd, JobStart};

/// 要印的一件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Record {
    /// 一件后台任务结束了：印那一行（同全屏，绿点、红叉）。
    Ended {
        /// 它开始时的样子（标题、是不是子代理）。
        start: JobStart,
        /// 结束的样子。
        end: JobEnd,
    },
    /// 回报叫醒她开了一轮：接上画它（要确认、要回答的照常弹）。
    Turn {
        /// 终端会话。
        session: String,
        /// 那一轮。
        turn: u64,
    },
}

/// 信箱在哪：缓存目录下 `shellhook/`。
pub fn dir() -> Option<PathBuf> {
    let root = miyu_client::places::cache_root(&miyu_client::places::Env::current()).ok()?;
    Some(root.join("shellhook"))
}

/// 这个 shell 的信箱。
fn file(shell: u32) -> Option<PathBuf> {
    Some(dir()?.join(format!("{shell}.jsonl")))
}

/// `shell` 的信箱里有没有东西。
pub fn waiting(shell: u32) -> bool {
    file(shell)
        .and_then(|path| std::fs::metadata(path).ok())
        .is_some_and(|meta| meta.len() > 0)
}

/// 往 `shell` 的信箱里追加一件。
///
/// # Errors
///
/// 找不到缓存目录、写不进去。
pub fn push(shell: u32, record: &Record) -> io::Result<()> {
    let path = file(shell).ok_or_else(|| io::Error::other("no cache directory"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(record).map_err(io::Error::other)?;
    line.push('\n');
    let mut out = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    out.write_all(line.as_bytes())
}

/// 把 `shell` 信箱里的都拿走（先换个名字再读，拿的时候追加进来的留给下一次）。读不懂的一行跳过。
pub fn take(shell: u32) -> Vec<Record> {
    let Some(path) = file(shell) else {
        return Vec::new();
    };
    let taking = path.with_extension(format!("{}.taking", std::process::id()));
    if std::fs::rename(&path, &taking).is_err() {
        return Vec::new();
    }
    let text = std::fs::read_to_string(&taking).unwrap_or_default();
    drop(std::fs::remove_file(&taking));
    text.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// `say` 自己已经印了的从信箱里拿掉：这几个任务结束的那一行、这几轮（盯后台的同时也记了，不拿掉回到提示符又印一遍）。
pub fn forget(shell: u32, jobs: &[String], turns: &[u64]) {
    let left: Vec<Record> = take(shell)
        .into_iter()
        .filter(|record| match record {
            Record::Ended { end, .. } => !jobs.contains(&end.job),
            Record::Turn { turn, .. } => !turns.contains(turn),
        })
        .collect();
    for record in &left {
        if push(shell, record).is_err() {
            break;
        }
    }
}
