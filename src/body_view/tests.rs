use super::{BodyView, slice};
use crate::ui::rows::Row;
use ratatui::layout::Rect;
use ratatui::text::Line;

fn row(plain: &str) -> Row {
    Row {
        line: Line::default(),
        target: None,
        shade: false,
        plain: plain.into(),
        copy_blocks: Vec::new(),
        content_x: 4,
        joined: false,
        links: Vec::new(),
        copy: true,
        figure: None,
        icon: None,
        figure_pending: false,
        blank: None,
    }
}

fn view_of(rows: Vec<Row>) -> BodyView {
    BodyView {
        rows: rows.into(),
        area: Rect::new(0, 0, 40, 10),
        ..BodyView::default()
    }
}

fn view() -> BodyView {
    view_of(vec![row("你好世界"), row("second line")])
}

#[test]
fn slicing_counts_display_columns() {
    assert_eq!(slice("你好世界", 2, 6), "好世");
    assert_eq!(slice("abc", 1, 99), "bc");
    // review C2（2026-10-11）：排版、画面照字素簇算宽，`⚠️`（U+26A0 加 FE0F）两列；原来逐字符算成一列，后面的全错位。
    assert_eq!(slice("⚠️ Note: foo", 9, 12), "foo");
    assert_eq!(slice("👩‍💻 ok", 3, 5), "ok", "带 ZWJ 的一个字两列");
}

#[test]
fn copying_skips_the_gutter_and_spans_rows() {
    let mut v = view();
    // 从第 0 行的第 6 列（「好」）拖到第 1 行的第 9 列（"second"）。
    v.select = Some(((0, 6), (1, 9)));
    assert_eq!(v.selected_text(), "好世界\nsecond");
    // 从槽里开始选的，也不带槽：末列算在里面，选到内容的第 1 列。
    v.select = Some(((1, 0), (1, 5)));
    assert_eq!(v.selected_text(), "se");
}

#[test]
fn releasing_a_drag_copies_and_keeps_the_selection() {
    use ratatui::crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    let mut v = view();
    let at = |kind, column| MouseEvent {
        kind,
        column,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    v.mouse(at(MouseEventKind::Down(MouseButton::Left), 4));
    v.mouse(at(MouseEventKind::Drag(MouseButton::Left), 7));
    let up = v.mouse(at(MouseEventKind::Up(MouseButton::Left), 7));
    assert_eq!(up, super::BodyAction::Copy("你好".into()));
    assert_eq!(v.selected_text(), "你好");
}

#[test]
fn folded_rows_copy_back_as_one_line() {
    // 第二行是第一行折下来的：复制出来是一行，折行处的空格留着。
    let mut folded = row("world");
    folded.joined = true;
    let mut v = view_of(vec![row("hello "), folded]);
    v.select = Some(((0, 4), (1, 20)));
    assert_eq!(v.selected_text(), "hello world");
}

#[test]
fn clicking_the_blank_of_an_open_timeline_folds_the_whole_segment() {
    // 2026-10-19 项目主人：展开的时间线，点没铺底色的空白收起整段；点开的一步铺了底色的空白收起那一步；字、标题照旧。
    use super::BodyAction;
    use crate::ui::rows::Target;
    use ratatui::crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    let segment = Target::Segment(3);
    let step = Target::Step(3, 0);
    let tagged = |plain: &str, target: Option<Target>, blank: Target| Row {
        target,
        blank: Some(blank),
        ..row(plain)
    };
    let mut v = view_of(vec![
        tagged("Used 2 tools · 2s", Some(segment), segment),
        tagged("", None, segment),
        tagged("读取 · a.rs", Some(step), segment),
        tagged("fn main() {}", Some(step), step),
    ]);
    let mut click = |row: u16, column: u16| {
        let at = |kind| MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        };
        v.mouse(at(MouseEventKind::Down(MouseButton::Left)));
        v.mouse(at(MouseEventKind::Up(MouseButton::Left)))
    };
    assert_eq!(click(1, 4), BodyAction::Toggle(segment), "步与步之间的竖线");
    assert_eq!(click(2, 30), BodyAction::Toggle(segment), "标题右边的空白");
    assert_eq!(
        click(2, 5),
        BodyAction::Toggle(step),
        "标题的字照旧开关那一步"
    );
    assert_eq!(
        click(3, 30),
        BodyAction::Toggle(step),
        "点开那一块的空白收起那一步"
    );
}
