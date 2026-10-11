//! 输入框上面那个框里画什么（蓝图「后台命令、子代理和侧边栏」第 3 条、`/help`、`/model`、`/sessions`、`/effort`、
//! `/language`、`/usage`）：照开着的是哪一个排成行，交回框（标题、提示）、行、每行是第几项（点中用）。

use std::time::Instant;

use ratatui::text::Line;

use super::panel::Chrome;
use super::{
    background, effort_list, help, languages, model_list, persona_list, session_list, spin_frame,
    usage,
};
use crate::app::Seen;

/// 开着的框排成的行，最多 `max` 行。
pub(super) fn content(
    seen: &Seen,
    width: u16,
    now: Instant,
    max: usize,
) -> (Chrome, Vec<Line<'static>>, Vec<Option<usize>>) {
    match seen.boxes.panel {
        Some(crate::app::Panel::Help { scroll }) => {
            let (chrome, lines) = help::lines(seen.config, width, scroll, max);
            let map = vec![None; lines.len()];
            (chrome, lines, map)
        }
        Some(crate::app::Panel::Models) => match &seen.boxes.model_list {
            Some(list) => model_list::lines(
                list,
                seen.talk.transcript.model_ref(),
                seen.config,
                width,
                max,
            ),
            None => Default::default(),
        },
        Some(crate::app::Panel::Sessions) => match &seen.boxes.session_list {
            Some(list) => session_list::lines(
                list,
                seen.talk.main_session().as_deref(),
                seen.cwd,
                seen.config,
                width,
                max,
                spin_frame(seen),
            ),
            None => Default::default(),
        },
        Some(crate::app::Panel::Effort { selected }) => effort_list::lines(
            seen.boxes.efforts.as_ref(),
            selected,
            seen.config,
            width,
            max,
        ),
        Some(crate::app::Panel::Language { selected }) => languages::lines(
            seen.config,
            &seen.link.system_language,
            selected,
            width,
            max,
        ),
        Some(crate::app::Panel::Usage { tab, scroll, money }) => {
            let (chrome, lines) = usage::lines(seen, (tab, scroll, money), width, max);
            let map = vec![None; lines.len()];
            (chrome, lines, map)
        }
        Some(crate::app::Panel::Pick { kind, selected }) => {
            let (list, default) = seen.pick_list(kind);
            persona_list::lines(
                list,
                default,
                kind,
                selected,
                &seen.config.text.persona_box,
                width,
                max,
            )
        }
        _ => background::lines(
            seen.boxes.panel,
            &seen.talk.board,
            seen.config,
            width,
            now,
            max,
        ),
    }
}
