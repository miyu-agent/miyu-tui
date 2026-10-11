//! 照显示宽度截断、折行（review，2026-10-11）：一律照字素簇算列，和排版（`linebreak.rs`）、画面一样。原来几处各写一份、
//! 逐字符算，`⚠️`（U+26A0 加 FE0F）、带 ZWJ 的 emoji 算窄了：放不下的照样放、行尾被画面截掉一个字，一个字拆到两行。

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// 放进 `width` 列：放不下的截掉后面、结尾写 `…`（`…` 也算在里面）。
pub fn clip(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    let mut out = String::new();
    let mut used = 0;
    for g in text.graphemes(true) {
        let w = g.width();
        if used + w + 1 > width {
            break;
        }
        used += w;
        out.push_str(g);
    }
    out.push('…');
    out
}

/// 放进 `width` 列：放不下的留后面那截（看得到最新写的），不加记号。
pub fn tail(text: &str, width: usize) -> String {
    let (kept, _) = last_cols(text, width);
    kept
}

/// 放进 `width` 列：放不下的从前面截掉、打头写 `…`（路径的末段最有用）。
pub fn tail_marked(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    let (kept, _) = last_cols(text, width.saturating_sub(1));
    format!("…{kept}")
}

/// 照显示宽度硬折：一行放满了另起一行，一个字放不下一行的也独占一行。
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut used = 0;
    for g in text.graphemes(true) {
        let w = g.width();
        if used + w > width && used > 0 {
            out.push(String::new());
            used = 0;
        }
        if let Some(last) = out.last_mut() {
            last.push_str(g);
        }
        used += w;
    }
    out
}

/// 从后往前、放得进 `width` 列的那一截，和它占几列。
fn last_cols(text: &str, width: usize) -> (String, usize) {
    let mut kept = Vec::new();
    let mut used = 0;
    for g in text.graphemes(true).rev() {
        let w = g.width();
        if used + w > width {
            break;
        }
        used += w;
        kept.push(g);
    }
    kept.reverse();
    (kept.concat(), used)
}

#[cfg(test)]
mod tests;
