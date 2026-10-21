//! 按键提示里的箭头（蓝图 `tui.md`「按键」表「按键模式」那一行）：按键模式是 Vim 时，界面上的字（`resources/text`）里写好的
//! 「↑/↓」「↑↓」写成「j/k」。箭头是那份字里的，不是画的记号，不归主题（`sheet/guard.rs` 照这个放过这一份）。

/// `vim` 时把提示里的上下箭头换成 `j/k`。
pub fn vim(hint: &str, vim: bool) -> String {
    if vim {
        hint.replace("↑/↓", "j/k").replace("↑↓", "j/k")
    } else {
        hint.to_string()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn arrows_in_a_hint_become_j_and_k_only_with_vim_keys() {
        assert_eq!(
            super::vim("↑/↓ 选择 · Enter 确定", true),
            "j/k 选择 · Enter 确定"
        );
        assert_eq!(super::vim("↑↓ 选择", true), "j/k 选择");
        assert_eq!(super::vim("↑/↓ 选择", false), "↑/↓ 选择");
    }
}
