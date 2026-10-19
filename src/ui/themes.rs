//! 配色的框（`/theme`，蓝图 `tui.md`「主题」第 6 条，2026-10-15 项目主人：出列表选，不再轮换）：和界面语言的框一个位置、
//! 一个样子。一种配色一行，写配置页里的那个名字；打开时的那一种右边暗色写「当前」；选中的照别的列表铺底。

use ratatui::text::{Line, Span};
use serde::Deserialize;

use super::panel::{self, Chrome, Row};
use crate::config::Config;

/// 框里的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Texts {
    /// 上边框写的。
    pub title: String,
    /// 下边框的按键提示。
    pub hint: String,
}

/// 框（标题、提示）、排好的行、每一行是第几种；`options` 是列出来的几种配色（配置项的值），`current` 是打开时用着的那种，
/// 选中第 `selected` 行，放不下 `max` 行时从离它最远的起少露。
pub fn lines(
    config: &Config,
    (options, current): (&[&str], &str),
    selected: usize,
    width: u16,
    max: usize,
) -> (Chrome, Vec<Line<'static>>, Vec<Option<usize>>) {
    let texts = &config.text.themes;
    let chrome = Chrome::new(&texts.title, Vec::new()).hint(&texts.hint);
    let names = config.text.settings.more.choice_names.get("tui.theme");
    let rows: Vec<Row> = options
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let name = names
                .and_then(|n| n.get(*value))
                .cloned()
                .unwrap_or_else(|| (*value).to_string());
            let lead = panel::current_lead(&config.layout.current_mark, *value == current);
            (
                Some(i),
                panel::item_with(i == selected, lead, vec![Span::raw(name)], None, width),
            )
        })
        .collect();
    let rows = panel::fit(rows, Some(selected), max);
    let (map, lines) = rows.into_iter().unzip();
    (chrome, lines, map)
}

#[cfg(test)]
mod tests {
    use super::lines;
    use crate::config::Config;

    #[test]
    fn each_option_has_its_settings_name_and_the_current_one_is_marked() {
        let config = Config::builtin().unwrap();
        let options = ["persona", "tokyonight", "terminal"];
        let (_, rows, map) = lines(&config, (&options, "tokyonight"), 2, 40, 10);
        let rows: Vec<String> = rows
            .iter()
            .map(|l| l.to_string().trim_end().to_string())
            .collect();
        assert_eq!(rows[0], "  跟随头像");
        assert_eq!(rows[1], "● Tokyo Night", "用着的那种行首写 ●：{rows:?}");
        assert_eq!(rows[2], "❯ 终端配色", "选中的写 ❯");
        assert_eq!(map, [Some(0), Some(1), Some(2)]);
    }
}
