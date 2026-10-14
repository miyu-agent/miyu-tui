//! 一条后台命令到这时为止的输出（`protocol.md` 的 `job.output`，施工 7-4 补）：头一次要最后几行，之后照 `size` 带 `after`、
//! `wait` 接着读，长了核心才答（核心 V-6 上；旧核心不交 `size` 的，照旧隔一阵要一次最后几行）。

use serde_json::Value;

/// `job.output` 的回应。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JobOutput {
    /// 交的字：最后 `tail` 行，照原样接起来，没清过转义序列。
    pub text: String,
    /// 一共几行。
    pub lines: u64,
    /// 还在跑（回报还没落盘）。
    pub running: bool,
    /// 前面还有没交的。
    pub truncated: bool,
    /// 到这一段为止一共多少字节，停在完整的字后面：下一次照它写 `after`。旧核心不交是 `None`。
    pub size: Option<u64>,
    /// 这一段后面还有没交的（带 `after` 的一次最多 128 KiB）：接着读，不等。
    pub more: bool,
}

/// 接着读时，界面留最后几行、最多几个字节（面板只露最后几行；一行再长也不留整份）。
const KEEP_BYTES: usize = 128 * 1024;

impl JobOutput {
    /// 照回应的 `result` 读。
    pub fn read(result: &Value) -> Self {
        Self {
            text: result["output"].as_str().unwrap_or_default().to_string(),
            lines: result["lines"].as_u64().unwrap_or(0),
            running: result["running"].as_bool().unwrap_or(false),
            truncated: result["truncated"].as_bool().unwrap_or(false),
            size: result["size"].as_u64(),
            more: result["more"].as_bool().unwrap_or(false),
        }
    }

    /// 接上带 `after` 读到的一段：字接在后面，行数照换行加（上一段末尾没换行的那一行接着写，不再加一），只留最后
    /// `keep` 行、最多 128 KiB；`size`、`more`、`running` 照新的。
    pub fn append(&mut self, next: &JobOutput, keep: usize) {
        let open = |t: &str| u64::from(!t.is_empty() && !t.ends_with('\n'));
        if !next.text.is_empty() {
            let newlines = next.text.matches('\n').count() as u64;
            self.lines = self.lines - open(&self.text) + newlines + open(&next.text);
            self.text.push_str(&next.text);
        }
        let mut cut = 0;
        // 从后往前数到第 `keep` 个整行的开头（最后一段没换行的算一行）。
        let body = self.text.trim_end_matches('\n');
        if let Some((at, _)) = body.rmatch_indices('\n').nth(keep.saturating_sub(1)) {
            cut = at + 1;
        }
        if self.text.len() - cut > KEEP_BYTES {
            cut = self.text.len() - KEEP_BYTES;
            while !self.text.is_char_boundary(cut) {
                cut += 1;
            }
        }
        if cut > 0 {
            self.text.drain(..cut);
            self.truncated = true;
        }
        self.size = next.size;
        self.more = next.more;
        self.running = next.running;
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::JobOutput;

    #[test]
    fn the_answer_of_job_output_is_read() {
        let result =
            json!({"lines": 2, "output": "building\nhalf\n", "running": true, "truncated": false});
        assert_eq!(
            JobOutput::read(&result),
            JobOutput {
                text: "building\nhalf\n".into(),
                lines: 2,
                running: true,
                truncated: false,
                size: None,
                more: false,
            }
        );
    }

    #[test]
    fn a_tail_answer_carries_the_size_to_go_on_from() {
        // 核心 V-6 上补：带 `tail` 的也交 `size`；带 `after` 的交 `more`。
        let tail =
            json!({"lines": 1, "output": "a\n", "running": true, "size": 2, "truncated": false});
        assert_eq!(JobOutput::read(&tail).size, Some(2));
        let after = json!({"more": true, "output": "b", "running": true, "size": 3});
        let read = JobOutput::read(&after);
        assert_eq!((read.size, read.more), (Some(3), true));
    }

    #[test]
    fn appended_pieces_join_lines_once_and_keep_only_the_last_rows() {
        let piece = |text: &str, size| JobOutput {
            text: text.into(),
            running: true,
            size: Some(size),
            ..JobOutput::default()
        };
        let mut had = JobOutput {
            text: "line1\nhal".into(),
            lines: 2,
            running: true,
            size: Some(10),
            ..JobOutput::default()
        };
        // 上一段末尾没换行的那一行接着写，不再加一行。
        had.append(&piece("f\nline3\n", 18), 10);
        assert_eq!(had.text, "line1\nhalf\nline3\n");
        assert_eq!((had.lines, had.size), (3, Some(18)));
        assert!(!had.truncated);
        // 空的一段（命令结束了）：字不动，跑没跑照新的。
        let mut ended = piece("", 18);
        ended.running = false;
        had.append(&ended, 10);
        assert_eq!((had.lines, had.running), (3, false));
        // 只留最后几行，前面去掉的记成还有没交的。
        had.append(&piece("line4\nline5\n", 30), 2);
        assert_eq!(had.text, "line4\nline5\n");
        assert_eq!(had.lines, 5, "行数照一共的算");
        assert!(had.truncated);
    }
}
