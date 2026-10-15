//! 「软件包」那一页的包列表（蓝图「配置页」第 43a 条，2026-10-16 项目主人定）：一个包一行，名字、暗色的一句说明、右边
//! 暗色写有几项；名字照最长的对齐。

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::fit;
use crate::settings::Texts;
use crate::settings::pages::packages::Listed;
use crate::theme;

/// 排成的行：字、是不是选中的那一行。
pub(super) fn lines(
    list: &[Listed],
    at: usize,
    texts: &Texts,
    width: u16,
) -> Vec<(Line<'static>, bool)> {
    let words = &texts.more;
    if list.is_empty() {
        return vec![(Line::styled(words.no_packages.clone(), theme::dim()), false)];
    }
    let name_w = list.iter().map(|p| p.name.width()).max().unwrap_or(0) + 2;
    let counts: Vec<String> = list
        .iter()
        .map(|p| words.package_items.replace("{n}", &p.count.to_string()))
        .collect();
    let count_w = counts.iter().map(|c| c.width()).max().unwrap_or(0);
    list.iter()
        .zip(&counts)
        .enumerate()
        .map(|(i, (p, count))| {
            // 左边空两格画选中的竖条（`paint`）；说明占名字和几项中间剩下的。
            let room = usize::from(width).saturating_sub(2 + name_w + count_w + 2);
            let summary = fit(p.summary.as_deref().unwrap_or_default(), room);
            let pad = " ".repeat(room.saturating_sub(summary.width()) + 2);
            let spans = vec![
                Span::raw(format!("{}{}", p.name, " ".repeat(name_w - p.name.width()))),
                Span::styled(summary, theme::dim()),
                Span::raw(pad),
                // 右对齐照显示宽度补空格（`{:>w$}` 照字数补，中文的「项」会顶出去）。
                Span::raw(" ".repeat(count_w - count.width())),
                Span::styled(count.clone(), theme::dim()),
            ];
            (Line::from(spans), i == at)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::lines;
    use crate::config::Config;
    use crate::settings::pages::packages::Listed;

    #[test]
    fn a_row_is_name_summary_and_the_count_flush_right() {
        let config = Config::builtin().unwrap();
        let list = [
            Listed {
                id: "memory".into(),
                name: "人格记忆".into(),
                summary: Some("记住你说过的事".into()),
                count: 5,
            },
            Listed {
                id: "onebot".into(),
                name: "接入QQ".into(),
                summary: None,
                count: 12,
            },
        ];
        let width = 60;
        let rows = lines(&list, 1, &config.text.settings, width);
        let text: Vec<String> = rows.iter().map(|(l, _)| l.to_string()).collect();
        assert!(text[0].starts_with("人格记忆  记住你说过的事"), "{text:?}");
        for row in &text {
            assert_eq!(
                unicode_width::UnicodeWidthStr::width(row.as_str()),
                usize::from(width) - 2,
                "右边的几项贴着右边、不顶出去：{row:?}"
            );
        }
        assert!(
            text[0].ends_with(" 5 项") && text[1].ends_with("12 项"),
            "{text:?}"
        );
        assert!(rows[1].1 && !rows[0].1, "选中的是第二行");
    }
}
