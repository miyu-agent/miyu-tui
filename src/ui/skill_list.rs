//! `$` 点名技能的列表（蓝图 `tui.md`「`$` 点名技能」，发版前 check 26）：贴在输入框上面，和 `@` 文件列表、斜杠命令列表一个
//! 框（`panel.rs`）；一个技能一行：名字，后面暗色接它的说明，对上的字强调色加下划线。上边框写打的字和条数，下边框写按键。

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::panel::{self, Chrome};
use crate::config::SkillWordTexts;
use crate::core::CoreSkill;
use crate::mention::Mode;
use crate::skill_word::Found;
use crate::theme;

/// 列表的框和露出来的那一段，最多 `rows` 条。`at` 是选中的、鼠标钉住的那一段。
pub fn lines(
    found: &Found,
    (selected, pinned): (usize, Option<usize>),
    rows: usize,
    width: u16,
    (words, hint): (&SkillWordTexts, &str),
) -> (Chrome, Vec<Line<'static>>) {
    let title = words.title.replace("{query}", &found.query);
    let count = words
        .count
        .replace("{count}", &found.items.len().to_string());
    let chrome = Chrome::new(&title, vec![Span::styled(count, theme::dim())]).hint(hint);
    if found.items.is_empty() {
        let empty = vec![Span::styled(words.empty.clone(), theme::faint())];
        return (chrome, vec![panel::item(false, empty, None, width)]);
    }
    let top = crate::menu::top(selected, pinned, found.items.len(), rows);
    let out = found
        .items
        .iter()
        .enumerate()
        .skip(top)
        .take(rows)
        .map(|(i, (skill, hits))| {
            let picked = i == selected;
            panel::item(picked, spans(skill, hits, picked), None, width)
        })
        .collect();
    (chrome, out)
}

/// 一条的几段：名字（选中的加粗，颜色由底色定），对上的字强调色加下划线；说明暗色接在后面。
fn spans(skill: &CoreSkill, hits: &[usize], picked: bool) -> Vec<Span<'static>> {
    let base = if picked {
        Style::new().add_modifier(Modifier::BOLD)
    } else {
        Style::new()
    };
    let hit = theme::accent().add_modifier(Modifier::UNDERLINED);
    let mut out: Vec<Span<'static>> = vec![Span::styled("$", base)];
    for (i, c) in skill.name.chars().enumerate() {
        let style = if hits.contains(&i) {
            base.patch(hit)
        } else {
            base
        };
        match out.last_mut() {
            Some(last) if last.style == style => last.content.to_mut().push(c),
            _ => out.push(Span::styled(c.to_string(), style)),
        }
    }
    if !skill.summary.is_empty() {
        out.push(Span::styled(format!("  {}", skill.summary), theme::dim()));
    }
    out
}

/// 下边框的按键说明：按键模式是 Vim 时照在哪一步写（同 `@` 列表，发版前 check 16）。
pub fn hint<'a>(words: &'a SkillWordTexts, vim: bool, mode: &Mode) -> &'a str {
    match (vim, mode) {
        (false, _) => &words.hints,
        (true, Mode::Typing) => &words.vim_hints,
        (true, Mode::Picking) => &words.picking,
        (true, Mode::Filtering(_)) => &words.filtering,
    }
}

#[cfg(test)]
mod tests {
    use super::lines;
    use crate::config::Config;
    use crate::core::CoreSkill;
    use crate::skill_word::Picker;
    use crate::theme;

    #[test]
    fn each_skill_shows_its_name_and_summary_with_hits_marked() {
        let _theme = theme::hold();
        let words = Config::builtin().unwrap().text.skill_word;
        let all = vec![
            CoreSkill {
                name: "review".into(),
                summary: "审一遍代码".into(),
            },
            CoreSkill {
                name: "refactor".into(),
                summary: String::new(),
            },
            CoreSkill {
                name: "pdf".into(),
                summary: String::new(),
            },
        ];
        let found = Picker::default().find("$re", 3, &all).expect("开");
        let (chrome, rows) = lines(&found, (0, None), 8, 60, (&words, &words.hints));
        let title: String = chrome.title.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(title, "$ re · 2 个");
        assert!(
            rows[0].to_string().contains("$review  审一遍代码"),
            "{}",
            rows[0]
        );
        // 没选中的那条（选中的字色由底色定）：对上的字强调色。
        let hit = rows[1]
            .spans
            .iter()
            .find(|s| s.content == "re")
            .expect("对上的字单独一段");
        assert_eq!(hit.style.fg, theme::accent().fg);
    }
}
