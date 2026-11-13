//! 整屏归它的几页（对话不画）：扩展的整页（蓝图「界面块」第 2 条）、「文件修改」那一页（「文件修改」第 2 条，check 9）。

use ratatui::Frame;

use crate::app::App;

/// 有一页开着就画它、交回 `true`。
pub(super) fn draw(frame: &mut Frame, app: &mut App) -> bool {
    if let Some(view) = app.boxes.widget_page.clone() {
        let (seen, screen, _) = app.parts();
        super::widget::page(frame, &view, &seen, screen);
        return true;
    }
    if let Some(mut page) = app.boxes.files_page.take() {
        let (seen, screen, _) = app.parts();
        super::files_page::draw(frame, &mut page, &seen, &mut screen.files_page);
        app.boxes.files_page = Some(page);
        return true;
    }
    false
}
