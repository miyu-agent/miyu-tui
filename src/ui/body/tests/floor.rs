//! 底边和空档：一轮结束收起留的空档留着，上面收起的、自己点开又收起的不算空档（蓝图「正文」第 1 条）。
use ratatui::layout::Rect;

use super::super::first_row;
use super::rows;
use crate::body_view::BodyView;
use crate::ui::rows::Row;

#[test]
fn the_gap_after_a_turn_stays_when_scrolling_away_and_back_or_opening_and_closing() {
    // 2026-10-17 项目主人：一轮结束时间线收起留的空档，展开再收起、滚上去再滚回来就没了，不符合直觉。
    let area = Rect::new(0, 0, 10, 10);
    let mut view = BodyView::default();
    assert_eq!(first_row(&rows(30), area, &mut view, true), 20);
    // 时间线收起，少了 6 行：顶上去的不掉下来，底下空 6 行。
    assert_eq!(first_row(&rows(24), area, &mut view, true), 20);
    // 往上滚三行：从空着的那一处往上数，不是从内容的末尾。
    view.top = Some(17);
    assert_eq!(first_row(&rows(24), area, &mut view, true), 17);
    // 滚回来：滚过头的停在原来那一处，接着跟着最新的，空档还在。
    view.top = Some(30);
    assert_eq!(first_row(&rows(24), area, &mut view, true), 20);
    assert!(view.top.is_none());
    assert_eq!(first_row(&rows(24), area, &mut view, true), 20);
    // 点开一步（钉在被点的那一行，展开 5 行），再收起：回到原来那一处。
    view.top = Some(20);
    assert_eq!(first_row(&rows(29), area, &mut view, true), 20);
    assert_eq!(first_row(&rows(24), area, &mut view, true), 20);
    assert!(view.top.is_none(), "收起以后接着跟着最新的");
    // 她接着说的字把空档填上。
    assert_eq!(first_row(&rows(32), area, &mut view, true), 22);
}

#[test]
fn folding_a_long_timeline_above_does_not_leave_its_rows_as_a_gap() {
    // 2026-10-19 项目主人：长时间线收着、她开了另一段在跑；把上面那段展开、滚上去、再收起，底下空出一大截，Ctrl+End
    // 也只露一点点，再展开才正常。底边记的是展开时的行号：上面那段收起少掉的几百行，不该算成空档。
    let area = Rect::new(0, 0, 10, 10);
    let mut view = BodyView::default();
    let rows = |timeline: usize| {
        crate::ui::row_cache::Rows::of_blank(&[(5, Some(0)), (timeline, Some(1)), (8, Some(2))])
    };
    let frame = |view: &mut BodyView, rows: crate::ui::row_cache::Rows| {
        let first = first_row(&rows, area, view, true);
        view.rows = rows;
        first
    };
    assert_eq!(frame(&mut view, rows(1)), 4, "上面那段收着，跟着最新的");
    assert_eq!(frame(&mut view, rows(300)), 303, "展开：跟着最新的");
    view.top = Some(50);
    assert_eq!(frame(&mut view, rows(300)), 50, "滚上去");
    // 收起：上面少了 299 行，底边跟着它下面那一条上移，不留空档。
    assert_eq!(frame(&mut view, rows(1)), 4, "收起以后不空出一大截");
    view.top = None;
    assert_eq!(
        frame(&mut view, rows(1)),
        4,
        "Ctrl+End 到底，在跑的那段整个露着"
    );
}

#[test]
fn the_last_entry_folding_still_leaves_its_gap() {
    // 10-17 那条照旧：最底下那段自己收起（一轮结束）空出的几行留着，等接下来的字填上。
    let area = Rect::new(0, 0, 10, 10);
    let mut view = BodyView::default();
    let rows = |live: usize| crate::ui::row_cache::Rows::of_blank(&[(5, Some(0)), (live, Some(1))]);
    assert_eq!(first_row(&rows(12), area, &mut view, true), 7);
    view.rows = rows(12);
    assert_eq!(
        first_row(&rows(8), area, &mut view, true),
        7,
        "顶上去的不掉下来"
    );
}

/// 2026-11-11 项目主人（发版前 check 33）：她还在跑的那一段里点开一步（展开出几十行）、在里面滚到过底、再点它收起，
/// 收起的那一行顶在最上面、下面空出一整屏。底边记的是展开着时的末尾：自己点开、收起的那几行不该算成空档，底边跟着
/// 这一步长了、短了多少一起走；原来就有的空档（一轮结束收起留的）照旧留着。
#[test]
fn opening_then_closing_a_step_in_the_live_entry_leaves_no_gap() {
    use crate::ui::rows::Target;
    let area = Rect::new(0, 0, 10, 10);
    let step = Target::Entry(9);
    // 第 1 条是在跑的那一段：3 行字、那一步（收着 1 行、展开 40 行）、后面 2 行。
    let rows = |open: usize| {
        let blank = Row::new(Vec::new(), 0, 0);
        let mut live = vec![blank.clone(); 3];
        live.extend((0..open).map(|_| {
            let mut row = blank.clone();
            row.target = Some(step);
            row
        }));
        live.extend(vec![blank.clone(); 2]);
        crate::ui::row_cache::Rows::of_owned(vec![
            (vec![blank.clone(); 20], Some(0)),
            (live, Some(1)),
        ])
    };
    let frame = |view: &mut BodyView, rows: crate::ui::row_cache::Rows| {
        let first = first_row(&rows, area, view, true);
        view.rows = rows;
        first
    };
    let mut view = BodyView::default();
    assert_eq!(
        frame(&mut view, rows(1)),
        16,
        "跟着最新的：26 行露最后 10 行"
    );
    // 点开那一步（它在第 23 行、屏上第 7 行）：比视口高，顶到最上面。
    view.anchor = Some((step, 7));
    assert_eq!(frame(&mut view, rows(40)), 23);
    // 在里面滚到底、再滚回它开头。
    view.top = Some(999);
    assert_eq!(frame(&mut view, rows(40)), 55);
    view.top = Some(23);
    assert_eq!(frame(&mut view, rows(40)), 23);
    // 点它收起：回到跟着最新的那一处，下面不空。
    view.anchor = Some((step, 0));
    assert_eq!(frame(&mut view, rows(1)), 16, "不留一整屏的空");
    assert!(view.top.is_none(), "接着跟着最新的");
}
