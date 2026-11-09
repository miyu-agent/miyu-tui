//! 输入框上面那一块的覆盖层（蓝图「斜杠命令列表」第 1、3 条）：命令列表、`@` 文件列表、输入历史列表、后台面板（`/help`
//! 这些框）占同一个地方，开哪个照 `app/frame.rs`；盖在待办、运行状态行、长期目标那一行上面（后画）。

use ratatui::Frame;
use ratatui::text::Line;

use super::panel::Chrome;
use super::{Areas, background, history, mention, menu, skill_list};
use crate::app::{App, Lists};

/// 画开着的那一个：`history`、`panel` 是先排好的行（占几行由它们定，`ui::draw`）。
pub(super) fn draw(
    frame: &mut Frame,
    app: &App,
    areas: Areas,
    lists: &Lists,
    (history_chrome, history_rows): (Chrome, Vec<history::Row>),
    (panel_chrome, panel_lines): (Chrome, Vec<Line<'static>>),
) {
    if let Some(matches) = &lists.matches {
        let rows = menu::rows(app.config.layout.menu_rows, areas.menu_text.height);
        let (chrome, lines) = menu::lines(
            matches,
            app.typing.menu.selected,
            app.typing.menu.pinned,
            rows,
            areas.menu_text.width,
            &app.config.text.menu,
        );
        menu::draw(frame, areas.menu, areas.menu_text, chrome, lines);
    }
    if let Some(found) = &lists.mentions {
        let rows = menu::rows(app.config.layout.menu_rows, areas.menu_text.height);
        let (chrome, lines) = mention::lines(
            found,
            app.typing.mention.selected,
            app.typing.mention.pinned,
            rows,
            areas.menu_text.width,
            (
                &app.config.text.mention,
                mention::hint(
                    &app.config.text.mention,
                    app.config.layout.vim,
                    &app.typing.mention.mode,
                ),
            ),
        );
        menu::draw(frame, areas.menu, areas.menu_text, chrome, lines);
    }
    if let Some(found) = &lists.skills {
        let rows = menu::rows(app.config.layout.menu_rows, areas.menu_text.height);
        let picker = &app.typing.skill_word;
        let words = &app.config.text.skill_word;
        let hint = skill_list::hint(words, app.config.layout.vim, &picker.mode);
        let (chrome, lines) = skill_list::lines(
            found,
            (picker.selected, picker.pinned),
            rows,
            areas.menu_text.width,
            (words, hint),
        );
        menu::draw(frame, areas.menu, areas.menu_text, chrome, lines);
    }
    if app.typing.history.open {
        history::draw(
            frame,
            areas.menu,
            areas.menu_text,
            history_chrome,
            history_rows,
        );
    }
    background::draw(
        frame,
        areas.menu,
        areas.menu_text,
        panel_chrome,
        panel_lines,
    );
}
