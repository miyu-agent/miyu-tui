//! 在一条连接上说 JSON-RPC 2.0，一行一条（`docs/designs/04-核心协议.md` P1）。
//!
//! 照 `crates/miyu-cli/src/rpc.rs` 的做法：读另起一个任务，等回应时来的推送留着，之后照先后交出去。
//! 那边的是 crate 私有的，演示程序用不上，只好照着写一份最小的。

use std::collections::VecDeque;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader, WriteHalf};
use tokio::sync::mpsc;

use miyu_client::connect::Connection;

/// 连着核心的一头。
pub struct Rpc {
    writer: WriteHalf<Connection>,
    incoming: mpsc::UnboundedReceiver<Value>,
    /// 等回应时来的推送。
    held: VecDeque<Value>,
}

impl Rpc {
    /// 在 `connection` 上说话：另起一个读的任务。要在 tokio 运行时里调。
    pub fn new(connection: Connection) -> Self {
        let (reader, writer) = tokio::io::split(connection);
        let (sender, incoming) = mpsc::unbounded_channel();
        tokio::spawn(read_all(BufReader::new(reader), sender));
        Self {
            writer,
            incoming,
            held: VecDeque::new(),
        }
    }

    /// 发一条请求，交回它的编号。
    ///
    /// # Errors
    ///
    /// 写不出去。
    pub async fn send(&mut self, method: &str, params: Value) -> io::Result<String> {
        let id = next_id();
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.writer
            .write_all(format!("{request}\n").as_bytes())
            .await?;
        Ok(id)
    }

    /// 发一条请求，等到它的回应，交回 `result`。
    ///
    /// # Errors
    ///
    /// 写不出去、核心断开、被拒绝：交回说清楚的一句。
    pub async fn call(&mut self, method: &str, params: Value) -> Result<Value, Failure> {
        let message = self.reply(method, params).await?;
        match message.get("error") {
            None => Ok(message["result"].clone()),
            Some(error) => Err(Failure::Refused(
                error["message"].as_str().unwrap_or_default().to_string(),
            )),
        }
    }

    /// 发一条请求，等它的回应，整条交回（拒了的照样交回，原因码在 `error.data.reason`）；中间来的别的留着。
    ///
    /// # Errors
    ///
    /// 写不出去、核心断开。
    pub async fn reply(&mut self, method: &str, params: Value) -> Result<Value, Failure> {
        let id = self.send(method, params).await.map_err(Failure::Io)?;
        while let Some(message) = self.incoming.recv().await {
            if message["id"] == json!(id) {
                return Ok(message);
            }
            self.held.push_back(message);
        }
        Err(Failure::Disconnected)
    }

    /// 下一条回应或推送，先交留着的。核心断开了是 `None`。
    pub async fn next(&mut self) -> Option<Value> {
        match self.held.pop_front() {
            Some(message) => Some(message),
            None => self.incoming.recv().await,
        }
    }
}

/// 一条请求没成的几种情形。
#[derive(Debug)]
pub enum Failure {
    /// 写不出去。
    Io(io::Error),
    /// 核心断开了。
    Disconnected,
    /// 核心拒绝了，带着它照握手时的语言说的原因。
    Refused(String),
}

/// 读一行的缓冲收过长过这么多的一行（`human.get` 这种上百 KB 的回应），这一行处理完缩回 [`LINE_KEPT`]。
const LINE_MOST: usize = 64 * 1024;
/// 缓冲缩回多大：平常的一条（推送、回应，几百字节到几 KB）放得下，不用每条重新要。
const LINE_KEPT: usize = 8 * 1024;

/// 读的一头：一行一条，读不懂的不要。读完了（核心断开）就停。
async fn read_all(mut reader: impl AsyncBufRead + Unpin, sender: mpsc::UnboundedSender<Value>) {
    let mut line = String::new();
    loop {
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let gone =
            serde_json::from_str::<Value>(&line).is_ok_and(|message| sender.send(message).is_err());
        handled(&mut line);
        if gone {
            return;
        }
    }
}

/// 一行处理完：清空；长过 [`LINE_MOST`] 的缩回 [`LINE_KEPT`]，不然收过一次大回应，缓冲就一直撑在那么大（2026-10-31 量过：
/// 连上以后常驻 128 KB）。
fn handled(line: &mut String) {
    line.clear();
    if line.capacity() > LINE_MOST {
        line.shrink_to(LINE_KEPT);
    }
}

/// 这个界面发的请求编号的开头：整个进程只定一次，重连以后也一样，核心推来的 `cause` 照它认是不是自己发的（蓝图
/// 「别处来的话」第 1 条）。进程号加启动时刻，两个界面不会撞上。
pub fn prefix() -> &'static str {
    static PREFIX: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PREFIX.get_or_init(|| {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        format!("tui-{:x}{nanos:x}", std::process::id())
    })
}

/// 下一条请求的编号：前缀加整个进程一串往下数的序号，重连以后接着数。核心按编号去重（`02-内核.md` 不变量 9），
/// 两次启动靠前缀分开，同一次启动里的两条连接靠序号分开（2026-10-02：原来每条连接从 1 数起，核心重启以后的撤销、
/// 恢复撞上以前的编号，拿回旧命令的回应，什么都没做）。
pub fn next_id() -> String {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{}-{n}", prefix())
}

/// 这个编号是这个界面发的请求。
pub fn owns(id: &str) -> bool {
    id.strip_prefix(prefix())
        .is_some_and(|rest| rest.starts_with('-'))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{LINE_KEPT, LINE_MOST, handled, owns, prefix, read_all};

    #[test]
    fn a_long_line_does_not_keep_the_buffer_large_and_a_short_one_keeps_it_as_is() {
        let mut long = String::with_capacity(128 * 1024);
        long.push('x');
        handled(&mut long);
        assert!(long.is_empty());
        assert!(long.capacity() <= LINE_KEPT, "{}", long.capacity());
        // 平常的一条不重新要：同一块、一样大。
        let mut short = String::with_capacity(LINE_MOST);
        short.push_str("{}");
        let at = short.as_ptr();
        handled(&mut short);
        assert_eq!((short.as_ptr(), short.capacity()), (at, LINE_MOST));
    }

    #[test]
    fn every_line_arrives_in_order_long_or_short_and_unreadable_ones_are_dropped() {
        let big = "a".repeat(200 * 1024);
        let input = format!("{{\"n\":1}}\n{{\"big\":\"{big}\"}}\nnot json\n{{\"n\":2}}\n");
        let (sender, mut incoming) = tokio::sync::mpsc::unbounded_channel();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime.block_on(read_all(input.as_bytes(), sender));
        let mut got: Vec<Value> = Vec::new();
        while let Ok(message) = incoming.try_recv() {
            got.push(message);
        }
        assert_eq!(
            got,
            vec![json!({"n": 1}), json!({"big": big}), json!({"n": 2})]
        );
    }

    #[test]
    fn request_ids_keep_counting_across_reconnects() {
        // 2026-10-02 项目主人报：核心重启以后 /undo 写了「已撤销」却没撤、/restore 也没恢复。原来每条连接从 1 数起，
        // 重连以后的编号和同一个会话里以前的命令撞上，核心按编号去重，交回了那条旧命令的回应，什么都没做。
        let first: Vec<String> = (0..3).map(|_| super::next_id()).collect();
        let after_reconnect: Vec<String> = (0..3).map(|_| super::next_id()).collect();
        assert!(
            first.iter().all(|id| !after_reconnect.contains(id)),
            "{first:?} {after_reconnect:?}"
        );
        assert!(first.iter().chain(&after_reconnect).all(|id| owns(id)));
    }

    #[test]
    fn our_request_ids_are_known_by_their_prefix() {
        // 核心推来的 `cause` 照它认是不是这个界面发的（蓝图「别处来的话」第 1 条），重连以后也一样。
        assert!(owns(&format!("{}-12", prefix())));
        assert!(!owns("web-12"));
        assert!(
            !owns(&format!("{}9-1", prefix())),
            "开头一样、后面多了字的不算"
        );
    }
}
