//! 能打字搜的列表（会话列表、`/model`、引导里「更多供应商…」）里搜索那一截的按键（蓝图 `tui.md`「按键」表「按键模式」
//! 那一行，2026-10-21 项目主人：「搜索用 vim 的方式，左斜杠键」）。列表自己的键（勾选、置顶、删除）先认，剩下的交这里；
//! 交回 [`Step::Pass`] 的（挪、`Enter` 选、`Esc` 关）再归列表。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// 搜索那一截认了这个键以后，列表该做什么。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// 搜的字换成这个：列表照它重筛、回到第一个。
    Query(String),
    /// 归搜索，列表别的不做（进出打字、没在搜时打的字）。
    Kept,
    /// 不归搜索：列表自己办（挪、`Enter` 选、`Esc` 关）。
    Pass,
}

/// 按下 `key`：`vim` 是 按键模式是不是 Vim，`typing` 是按了 `/` 正在打搜索的字（只在按键模式是 Vim 时有），`query` 是现在搜的字。
///
/// 关着：打字就是搜，退格删一个。开着：没在打字时 `/` 清空了进打字，`Esc` 先去掉留着的筛，别的字不进搜索；打字时
/// `Enter` 收起（筛的留着），`Esc` 不搜了，删光了再退格也收起。
pub fn key(vim: bool, typing: &mut bool, query: &str, key: KeyEvent) -> Step {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let pushed = |c: char| Step::Query(format!("{query}{c}"));
    let popped = || {
        let mut query = query.to_string();
        query.pop();
        Step::Query(query)
    };
    if !vim {
        return match key.code {
            KeyCode::Char(c) if !ctrl => pushed(c),
            KeyCode::Backspace => popped(),
            _ => Step::Pass,
        };
    }
    if *typing {
        return match key.code {
            KeyCode::Enter => {
                *typing = false;
                Step::Kept
            }
            KeyCode::Esc => {
                *typing = false;
                Step::Query(String::new())
            }
            KeyCode::Backspace if query.is_empty() => {
                *typing = false;
                Step::Kept
            }
            KeyCode::Backspace => popped(),
            KeyCode::Char(c) if !ctrl => pushed(c),
            _ => Step::Pass,
        };
    }
    match key.code {
        KeyCode::Char('/') if !ctrl => {
            *typing = true;
            Step::Query(String::new())
        }
        KeyCode::Esc if !query.is_empty() => Step::Query(String::new()),
        KeyCode::Char(_) if !ctrl => Step::Kept,
        KeyCode::Backspace => Step::Kept,
        _ => Step::Pass,
    }
}

/// 下边框的按键提示：关着的 `plain`（`↑/↓`），开着的 `vim`（`/ 搜索 · j/k`），打字时的 `searching`。
pub fn hint<'a>(
    vim: bool,
    typing: bool,
    plain: &'a str,
    vim_hint: &'a str,
    searching: &'a str,
) -> &'a str {
    match (vim, typing) {
        (false, _) => plain,
        (true, false) => vim_hint,
        (true, true) => searching,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{Step, key};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn with_vim_keys_off_typing_searches_right_away() {
        let mut typing = false;
        assert_eq!(
            key(false, &mut typing, "op", press(KeyCode::Char('j'))),
            Step::Query("opj".into())
        );
        assert_eq!(
            key(false, &mut typing, "op", press(KeyCode::Char('/'))),
            Step::Query("op/".into())
        );
        assert_eq!(
            key(false, &mut typing, "op", press(KeyCode::Backspace)),
            Step::Query("o".into())
        );
        assert_eq!(
            key(false, &mut typing, "op", press(KeyCode::Enter)),
            Step::Pass
        );
        assert_eq!(
            key(false, &mut typing, "op", press(KeyCode::Esc)),
            Step::Pass,
            "关框归列表"
        );
        assert!(!typing);
    }

    #[test]
    fn with_vim_keys_on_a_slash_starts_a_fresh_search() {
        // 2026-10-21 项目主人：「下次再 / 就清空搜索结果后再次进入搜索模式」。
        let mut typing = false;
        assert_eq!(
            key(true, &mut typing, "", press(KeyCode::Char('x'))),
            Step::Kept,
            "没在搜时字不进搜索"
        );
        assert_eq!(
            key(true, &mut typing, "old", press(KeyCode::Char('/'))),
            Step::Query(String::new())
        );
        assert!(typing);
        assert_eq!(
            key(true, &mut typing, "", press(KeyCode::Char('j'))),
            Step::Query("j".into()),
            "打字时 j 是字"
        );
        assert_eq!(
            key(true, &mut typing, "j", press(KeyCode::Down)),
            Step::Pass,
            "挪归列表"
        );
        let ctrl_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL);
        assert_eq!(key(true, &mut typing, "j", ctrl_j), Step::Pass);
    }

    #[test]
    fn enter_keeps_the_filter_and_esc_drops_it() {
        let mut typing = true;
        assert_eq!(
            key(true, &mut typing, "cl", press(KeyCode::Enter)),
            Step::Kept
        );
        assert!(!typing, "Enter 收起打字");
        assert_eq!(
            key(true, &mut typing, "cl", press(KeyCode::Enter)),
            Step::Pass,
            "再 Enter 是选"
        );
        assert_eq!(
            key(true, &mut typing, "cl", press(KeyCode::Esc)),
            Step::Query(String::new()),
            "Esc 先去掉筛的"
        );
        assert_eq!(
            key(true, &mut typing, "", press(KeyCode::Esc)),
            Step::Pass,
            "再按关框"
        );
        typing = true;
        assert_eq!(
            key(true, &mut typing, "cl", press(KeyCode::Esc)),
            Step::Query(String::new())
        );
        assert!(!typing);
        typing = true;
        assert_eq!(
            key(true, &mut typing, "", press(KeyCode::Backspace)),
            Step::Kept
        );
        assert!(!typing, "删光了再退格收起");
    }
}
