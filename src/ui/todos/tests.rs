//! 待办那一块（「后台命令、子代理和侧边栏」第 4、4a 条）：收着、点开的写法。

use ratatui::style::Modifier;

use super::todo_lines;
use crate::config::Config;
use crate::jobs::{Board, Todo, TodoState};

/// 一份待办：前 `done` 项做完、下一项在做、再往后 `pending` 项没做。
fn board(done: usize, pending: usize) -> Board {
    let mut board = Board::default();
    board.todos = (0..=done + pending)
        .map(|i| Todo {
            text: format!("第 {i} 项，改时间线的连接线，让出错的那一步一路红到下一步的标题"),
            state: match i.cmp(&done) {
                std::cmp::Ordering::Less => TodoState::Done,
                std::cmp::Ordering::Equal => TodoState::Active,
                std::cmp::Ordering::Greater => TodoState::Pending,
            },
        })
        .collect();
    board
}

fn text(board: &Board, width: u16, full: bool) -> Vec<String> {
    let config = Config::builtin().unwrap();
    todo_lines(board, &config, width, full)
        .iter()
        .map(|l| l.to_string())
        .collect()
}

#[test]
fn folded_it_shows_the_one_in_progress_and_the_next_two() {
    // 2026-10-26 项目主人（反馈第 1 条）：只露进行中的一条、待进行的两条，标题写总数，底下不再接「还有」「已完成」。
    let lines = text(&board(12, 7), 30, false);
    assert_eq!(lines.len(), 4, "标题加三项：{lines:?}");
    assert_eq!(lines[0], "待办 12/20");
    assert!(lines[1].starts_with("■ 第 12 项"), "先是在做的那项");
    assert!(lines[1].ends_with('…'), "长的截掉加 …");
    assert!(lines[2].starts_with("☐ 第 13 项"));
    assert!(lines[3].starts_with("☐ 第 14 项"));
    // 剩得少的露剩下的。
    let lines = text(&board(2, 1), 30, false);
    assert_eq!(lines.len(), 3, "在做的、没做的一项，做完的不露：{lines:?}");
    // 没有在做的：露前三项没做的。
    let mut idle = board(0, 4);
    idle.todos[0].state = TodoState::Pending;
    let lines = text(&idle, 30, false);
    assert_eq!(lines.len(), 4, "{lines:?}");
    assert!(lines[1].starts_with("☐ 第 0 项"), "{lines:?}");
}

#[test]
fn opened_it_lists_everything_wrapped_with_the_done_ones_last() {
    let config = Config::builtin().unwrap();
    let board = board(1, 2);
    let lines = todo_lines(&board, &config, 12, true);
    let text: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    assert_eq!(text[0], "待办 1/4");
    let last = text.len() - 1;
    assert!(
        text[last].starts_with("  ") || text[last].starts_with("☑ 第 0 项"),
        "做完的排到末尾（2026-10-17 项目主人）：{text:?}"
    );
    let done = text
        .iter()
        .position(|t| t.starts_with("☑ 第 0 项"))
        .unwrap();
    assert!(
        lines[done].spans[1]
            .style
            .add_modifier
            .contains(Modifier::CROSSED_OUT),
        "做完的划掉"
    );
    assert!(
        text.iter().position(|t| t.starts_with("☐ ")).unwrap() < done,
        "没做完的在前：{text:?}"
    );
    let active = text.iter().position(|t| t.starts_with("■ ")).unwrap();
    assert!(text[active + 1].starts_with("  "), "折下来的和字对齐");
}

#[test]
fn nothing_or_all_done_draws_nothing() {
    assert!(text(&Board::default(), 30, false).is_empty());
    let all_done = board(4, 0);
    let mut all_done = all_done;
    all_done.todos[4].state = TodoState::Done;
    assert!(text(&all_done, 30, false).is_empty(), "全做完了：收掉");
    assert!(!text(&board(4, 0), 30, false).is_empty(), "还剩一项：还在");
}
