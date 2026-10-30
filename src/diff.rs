//! 编辑、写入点开以后的差异（蓝图 `tui.md`「编辑、写入点开的差异」）：照工具调用的参数比，按行排。
//!
//! 头不读核心的存储（`01-架构.md` 第五节第 1 条），所以只照参数：写入的全部是加上的，带行号；编辑的每一处
//! 按行比，不知道在文件的第几行，不带行号。完整的差异等核心经协议交出来（蓝图「还没有的」）。

mod cache;
pub mod unified;

use std::rc::Rc;
use std::time::Duration;

use serde::Deserialize;
use similar::{ChangeTag, TextDiff};

/// 每处改动前后带几行不变的。
const CONTEXT: usize = 3;

/// 比一份最多用多久（发布前审查 T2-1）：整块重写大文件时 Myers 按改动数的平方走，4000 行比一遍两百多毫秒；到了就退成粗的
/// 差异（多算几行改动），不让界面卡住。
const DEADLINE: Duration = Duration::from_millis(30);

/// 写入的参数里要的那一格。
#[derive(Deserialize)]
struct Write {
    content: String,
}

/// 编辑的参数里要的那一格。
#[derive(Deserialize)]
struct Edit {
    edits: Vec<Piece>,
}

/// 编辑的一处。
#[derive(Deserialize)]
struct Piece {
    old_string: String,
    new_string: String,
}

/// 差异里一行是什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// 没变的。
    Keep,
    /// 删掉的。
    Removed,
    /// 加上的。
    Added,
    /// 两处改动隔得远，中间略过的。
    Gap,
}

/// 差异里的一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// 是什么。
    pub mark: Mark,
    /// 行号：删掉的是原来的，别的是改后的；略过的那一行没有。
    pub number: Option<usize>,
    /// 这一行的字，不带行尾的换行。
    pub text: String,
}

/// 一个文件的差异。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diff {
    /// 排好的行。
    pub lines: Vec<Line>,
    /// 加了几行。
    pub added: usize,
    /// 删了几行。
    pub removed: usize,
}

/// 比两份内容。新建的文件，`before` 传空的。
pub fn compare(before: &str, after: &str) -> Diff {
    let diff = TextDiff::configure()
        .timeout(DEADLINE)
        .diff_lines(before, after);
    let mut out = Diff::default();
    for (i, group) in diff.grouped_ops(CONTEXT).iter().enumerate() {
        if i > 0 {
            out.lines.push(Line {
                mark: Mark::Gap,
                number: None,
                text: String::new(),
            });
        }
        for op in group {
            for change in diff.iter_changes(op) {
                let (mark, number) = match change.tag() {
                    ChangeTag::Equal => (Mark::Keep, change.new_index()),
                    ChangeTag::Delete => (Mark::Removed, change.old_index()),
                    ChangeTag::Insert => (Mark::Added, change.new_index()),
                };
                match mark {
                    Mark::Added => out.added += 1,
                    Mark::Removed => out.removed += 1,
                    _ => {}
                }
                let text = change.value().trim_end_matches(['\n', '\r']).to_string();
                out.lines.push(Line {
                    mark,
                    number: number.map(|n| n + 1),
                    text,
                });
            }
        }
    }
    out
}

/// 照工具调用的参数排差异：有 `content` 的是写入，有 `edits` 的是编辑；都不是的是 `None`。同样的参数比过的直接拿
/// （`cache.rs`）：标题、合计、点开的差异共用一份，每帧不再重比。
pub fn from_args(args: &serde_json::Value) -> Option<Rc<Diff>> {
    cache::get_or(cache::key(args), || build(args))
}

/// 照参数比一遍。
fn build(args: &serde_json::Value) -> Option<Diff> {
    if let Ok(write) = Write::deserialize(args) {
        return Some(compare("", &write.content));
    }
    let edit = Edit::deserialize(args).ok()?;
    let mut out = Diff::default();
    for (i, piece) in edit.edits.iter().enumerate() {
        if i > 0 {
            out.lines.push(Line {
                mark: Mark::Gap,
                number: None,
                text: String::new(),
            });
        }
        let one = compare(&piece.old_string, &piece.new_string);
        out.added += one.added;
        out.removed += one.removed;
        // 片段里的行号不是文件里的行号，不写。
        out.lines
            .extend(one.lines.into_iter().map(|l| Line { number: None, ..l }));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{DEADLINE, Mark, compare, from_args};

    fn marks(before: &str, after: &str) -> Vec<(Mark, Option<usize>, String)> {
        compare(before, after)
            .lines
            .into_iter()
            .map(|l| (l.mark, l.number, l.text))
            .collect()
    }

    #[test]
    fn removed_lines_keep_old_numbers_and_added_ones_new() {
        let before = "a\nb\nc\n";
        let after = "a\nB\nc\nd\n";
        assert_eq!(
            marks(before, after),
            vec![
                (Mark::Keep, Some(1), "a".into()),
                (Mark::Removed, Some(2), "b".into()),
                (Mark::Added, Some(2), "B".into()),
                (Mark::Keep, Some(3), "c".into()),
                (Mark::Added, Some(4), "d".into()),
            ]
        );
    }

    #[test]
    fn far_apart_changes_get_a_gap_and_three_lines_of_context() {
        let before: String = (1..=20).map(|n| format!("line {n}\n")).collect();
        let after = before
            .replace("line 2\n", "line two\n")
            .replace("line 19\n", "line nineteen\n");
        let diff = compare(&before, &after);
        let gaps = diff.lines.iter().filter(|l| l.mark == Mark::Gap).count();
        assert_eq!(gaps, 1);
        // 第一处：第 1 行、改掉的第 2 行、加上的第 2 行、第 3 到 5 行。
        assert_eq!(
            diff.lines[..6]
                .iter()
                .filter(|l| l.mark == Mark::Keep)
                .count(),
            4
        );
        assert_eq!((diff.added, diff.removed), (2, 2));
    }

    #[test]
    fn edits_from_args_have_no_numbers_and_a_gap_between() {
        let args = serde_json::json!({"file_path": "a", "edits": [
            {"old_string": "x\ny", "new_string": "x\nY"},
            {"old_string": "z", "new_string": ""}]});
        let diff = super::from_args(&args).unwrap();
        assert!(diff.lines.iter().all(|l| l.number.is_none()));
        assert_eq!(diff.lines.iter().filter(|l| l.mark == Mark::Gap).count(), 1);
        assert_eq!((diff.added, diff.removed), (1, 2));
        let write =
            super::from_args(&serde_json::json!({"file_path": "a", "content": "1\n2\n"})).unwrap();
        assert_eq!(write.lines.last().map(|l| l.number), Some(Some(2)));
    }

    /// 量尺（不断言耗时，AGENTS 规矩）：`cargo test --bin miyu-tui a_large_rewrite -- --ignored --nocapture` 打出比一遍
    /// 用了多久。
    #[test]
    #[ignore = "量尺"]
    fn a_large_rewrite_is_compared_within_a_frame() {
        // 发布前审查（2026-10-30）：还在进行的那一段每帧重排（`row_cache::fingerprint` 交 `None`），编辑这一步的标题
        // `+N -N`、收起那一行的合计、点开的差异每帧各照参数重比一遍（`from_args`）。整块重写的编辑两边几乎没有相同的
        // 行，Myers 不设期限时按改动数的平方走，这一轮剩下的时间里每一帧都卡在这里。
        let old: String = (0..4000).map(|n| format!("old line {n}\n")).collect();
        let new: String = (0..4000).map(|n| format!("new line {n}\n")).collect();
        let start = std::time::Instant::now();
        let diff = compare(&old, &new);
        let spent = start.elapsed();
        assert_eq!((diff.added, diff.removed), (4000, 4000));
        eprintln!("4000 行整块重写比一次用了 {spent:?}（期限 {DEADLINE:?}）");
    }

    /// 同样的参数只比一次（发布前审查 T2-1）：标题、合计、点开的差异拿的是同一份；参数不一样的另比。
    #[test]
    fn the_same_arguments_are_compared_once() {
        let args = serde_json::json!({"edits": [{"old_string": "a\nb", "new_string": "a\nc"}]});
        let one = from_args(&args).unwrap();
        let again = from_args(&args.clone()).unwrap();
        assert!(std::rc::Rc::ptr_eq(&one, &again), "比过的直接拿");
        assert_eq!((one.added, one.removed), (1, 1));
        let other = serde_json::json!({"edits": [{"old_string": "a", "new_string": "b"}]});
        assert!(!std::rc::Rc::ptr_eq(&one, &from_args(&other).unwrap()));
        let write = serde_json::json!({"content": "a\nc"});
        assert_eq!(
            from_args(&write).map(|d| d.added),
            Some(2),
            "写入和编辑不混"
        );
        assert!(from_args(&serde_json::json!({"path": "x"})).is_none());
    }

    #[test]
    fn a_new_file_is_all_added() {
        let diff = compare("", "x\ny\n");
        assert!(diff.lines.iter().all(|l| l.mark == Mark::Added));
        assert_eq!((diff.added, diff.removed), (2, 0));
    }
}
