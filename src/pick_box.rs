//! 人格框、预设框的按键（蓝图 `tui.md`「新会话：人格、工作区」第 1 到 3 条、第 2b 条）：上下挪（跳过写错的），`Enter`
//! 选，`e` 编辑，`Esc`。这里只交回按了什么，选了写到哪由用它的一方定：终端开会话时带上、开着的会话发给核心
//! （`app/new_session.rs`），shellhook 写 `terminal.persona`、`terminal.preset`（2026-10-20 和 shellhook 会话对的）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::core::Persona;

/// 按了一个键以后。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pressed {
    /// 光标挪到第几行（挪不动的是原来那一行）。
    Moved(usize),
    /// 选了第几行。
    Chose(usize),
    /// 要编辑第几行（第 2b 条）。
    Edit(usize),
    /// 光标那一行不能编辑：「无人格」、写错了读不出来的。
    CantEdit,
    /// `Esc`。
    Escape,
    /// 别的键，和写错了选不了的那一行上按 `Enter`。
    Ignored,
}

/// 光标停在 `selected` 上按了 `key`。
pub fn press(list: &[Persona], selected: usize, key: KeyEvent) -> Pressed {
    let row = list.get(selected).filter(|p| p.problem.is_none());
    match key.code {
        KeyCode::Down => Pressed::Moved(usable(list).find(|&i| i > selected).unwrap_or(selected)),
        KeyCode::Up => Pressed::Moved(
            usable(list)
                .filter(|&i| i < selected)
                .last()
                .unwrap_or(selected),
        ),
        KeyCode::Enter if row.is_some() => Pressed::Chose(selected),
        KeyCode::Char('e' | 'E') => match row {
            Some(p) if !p.id.is_empty() => Pressed::Edit(selected),
            _ => Pressed::CantEdit,
        },
        KeyCode::Esc => Pressed::Escape,
        _ => Pressed::Ignored,
    }
}

/// 选得了的那几行。
pub fn usable(list: &[Persona]) -> impl Iterator<Item = usize> + '_ {
    (0..list.len()).filter(|&i| list[i].problem.is_none())
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{Pressed, press};
    use crate::core::Persona;

    fn row(id: &str, problem: Option<&str>) -> Persona {
        Persona {
            id: id.into(),
            problem: problem.map(str::to_string),
            ..Persona::default()
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn e_edits_a_real_row_and_not_the_none_row_or_a_broken_one() {
        // 第 2b 条：「无人格」（编号空着）、写错了读不出来的不能编辑；出厂的照样能。
        let list = [row("", None), row("miyu", None), row("broken", Some("bad"))];
        let e = key(KeyCode::Char('e'));
        assert_eq!(press(&list, 1, e), Pressed::Edit(1));
        assert_eq!(press(&list, 0, e), Pressed::CantEdit, "无人格");
        assert_eq!(press(&list, 2, e), Pressed::CantEdit, "写错了的");
        assert_eq!(press(&list, 9, e), Pressed::CantEdit, "列表变短了");
    }

    #[test]
    fn arrows_skip_broken_rows_and_enter_picks_only_usable_ones() {
        let list = [row("a", None), row("broken", Some("bad")), row("c", None)];
        assert_eq!(press(&list, 0, key(KeyCode::Down)), Pressed::Moved(2));
        assert_eq!(press(&list, 2, key(KeyCode::Down)), Pressed::Moved(2));
        assert_eq!(press(&list, 2, key(KeyCode::Up)), Pressed::Moved(0));
        assert_eq!(press(&list, 0, key(KeyCode::Enter)), Pressed::Chose(0));
        assert_eq!(press(&list, 1, key(KeyCode::Enter)), Pressed::Ignored);
        assert_eq!(press(&list, 0, key(KeyCode::Esc)), Pressed::Escape);
        assert_eq!(press(&list, 0, key(KeyCode::Char('x'))), Pressed::Ignored);
    }
}
