//! 离视口远的条目只留行数（施工 LW-22，蓝图「正文」第 8 条）：照 App 的做法画，和全部留着（`keep_screens` 是 0）的
//! 同一个会话一帧帧比——滚、翻页、跳远、点开收起、拖选复制、改宽度以后，视口里画出来的逐格相同。

use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use serde_json::json;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::app::App;
use crate::core::{Command, ToolStatus};
use crate::transcript::{Kind, Segment, Step, StepKind, ToolState};
use crate::ui::rows::Target;

/// 一个画在测试终端里的界面。
struct Side {
    app: App,
    term: Terminal<TestBackend>,
    _sent: UnboundedReceiver<Command>,
}

/// 做完了的一段：想一下、跑一条命令。
fn segment() -> Segment {
    let t0 = Instant::now();
    let mut thought = Step::new(StepKind::Thought {
        text: "先看看目录\n再想想".into(),
    });
    thought.started = t0;
    thought.took = Some(Duration::from_secs(1));
    let mut shell = Step::new(StepKind::Tool {
        name: "shell".into(),
        args: String::new(),
        parsed: json!({"command": "ls", "description": "列目录"}),
        state: ToolState::Done(ToolStatus::Ok),
        output: "a.txt\nb.txt".into(),
        said: None,
    });
    shell.started = t0;
    shell.took = Some(Duration::from_secs(1));
    let mut seg = Segment::new();
    seg.steps = vec![thought, shell];
    seg.finished = true;
    seg
}

/// `keep_screens` 屏以外放掉、`turns` 轮的会话，画在 `width`×30 的终端里（还没画）。每一轮：你说的话、一段时间线、
/// 长短不一的回答。
fn side(keep_screens: usize, turns: usize, width: u16) -> Side {
    let (mut app, sent) = crate::app::test_support::app();
    app.boxes.welcome_known = true;
    app.boxes.personas_known = true;
    app.config.layout.keep_screens = keep_screens;
    let t = &mut app.talk.transcript;
    for k in 0..turns {
        t.note(Kind::User, format!("第 {k} 句：帮我看看这个目录"));
        t.note(Kind::Steps, String::new());
        t.entries.last_mut().unwrap().segment = Some(segment());
        let body = format!("第 {k} 轮的一段回答，稍微长一点，好在窄的窗口里折几行。\n\n");
        t.note(
            Kind::Reply,
            format!("## 第 {k} 轮\n\n{}", body.repeat(1 + k % 5)),
        );
    }
    let term = Terminal::new(TestBackend::new(width, 30)).unwrap();
    Side {
        app,
        term,
        _sent: sent,
    }
}

impl Side {
    fn draw(&mut self) {
        let app = &mut self.app;
        self.term.draw(|f| crate::ui::draw(f, app)).unwrap();
    }

    /// 正文区画出来的每一行。
    fn body(&self) -> Vec<String> {
        let a = self.app.screen.areas.body;
        let buf = self.term.backend().buffer();
        (a.y..a.bottom())
            .map(|y| (a.x..a.right()).map(|x| buf[(x, y)].symbol()).collect())
            .collect()
    }

    fn mouse(&mut self, kind: MouseEventKind, column: u16, row: u16) {
        let event = MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        };
        self.app.handle(Event::Mouse(event));
    }

    /// 滚轮在正文当中滚一格。
    fn wheel(&mut self, up: bool) {
        let a = self.app.screen.areas.body;
        let kind = if up {
            MouseEventKind::ScrollUp
        } else {
            MouseEventKind::ScrollDown
        };
        self.mouse(kind, a.x + 2, a.y + a.height / 2);
    }

    /// 按在滚动条第 `y` 行上（滑块中间跳过去），再松开。
    fn bar(&mut self, y: u16) {
        let x = self.app.screen.view.bar_column();
        let a = self.app.screen.areas.body;
        self.mouse(MouseEventKind::Down(MouseButton::Left), x, a.y + y);
        self.mouse(MouseEventKind::Up(MouseButton::Left), x, a.y + y);
    }

    /// `/jump` 跳到第 `index` 条：照上一帧的行找它从第几行起，顶到视口第一行（`app/jump.rs`）。
    fn jump(&mut self, index: usize) {
        let view = &mut self.app.screen.view;
        let row = view.rows.start_of(index).unwrap();
        let id = self.app.talk.transcript.entries[index].id;
        view.jump_to(row, id, Instant::now());
    }

    /// 这一帧视口里的每一行都有内容。
    fn viewport_has_rows(&self) -> bool {
        let view = &self.app.screen.view;
        let height = usize::from(view.area.height);
        (view.first..(view.first + height).min(view.rows.len())).all(|i| view.rows.get(i).is_some())
    }

    /// 每一条从第几行到第几行。
    fn spans(&self) -> Vec<(Option<usize>, Option<usize>)> {
        let rows = &self.app.screen.view.rows;
        (0..self.app.talk.transcript.entries.len())
            .map(|i| (rows.start_of(i), rows.end_of(i)))
            .collect()
    }
}

/// 一样的会话一个放掉远处的（`keep_screens` 4），一个全部留着。
fn pair(turns: usize) -> (Side, Side) {
    (side(4, turns, 70), side(0, turns, 70))
}

/// 两边各做一样的事、各画一帧，视口里画出来的逐格相同、第一行相同；放掉的那一边视口里每一行都有内容。
fn both(keep: &mut Side, all: &mut Side, what: &str, act: impl Fn(&mut Side)) {
    act(keep);
    act(all);
    keep.draw();
    all.draw();
    assert_eq!(
        keep.app.screen.view.first, all.app.screen.view.first,
        "{what}：第一行"
    );
    assert_eq!(keep.body(), all.body(), "{what}：视口里画的");
    assert!(keep.viewport_has_rows(), "{what}：视口里每一行都有内容");
}

#[test]
fn scrolling_far_and_back_draws_the_same_as_keeping_everything() {
    let (mut keep, mut all) = pair(40);
    both(&mut keep, &mut all, "头一帧", |_| {});
    let height = usize::from(keep.app.screen.view.area.height);
    assert!(
        keep.app.screen.view.rows.len() > 12 * height,
        "会话长过留着的范围"
    );
    for _ in 0..5 {
        both(&mut keep, &mut all, "滚轮往上", |s| s.wheel(true));
    }
    for _ in 0..4 {
        both(&mut keep, &mut all, "往上翻页", |s| {
            s.app.screen.view.page(false)
        });
    }
    let paged = keep.app.screen.view.first;
    both(&mut keep, &mut all, "跳到中间", |s| s.jump(60));
    assert_ne!(keep.app.screen.view.first, paged, "跳动了");
    both(&mut keep, &mut all, "滚轮往下", |s| s.wheel(false));
    both(&mut keep, &mut all, "拖滚动条到开头", |s| s.bar(0));
    for _ in 0..3 {
        both(&mut keep, &mut all, "开头往下翻页", |s| {
            s.app.screen.view.page(true)
        });
    }
    both(&mut keep, &mut all, "回到底部", |s| {
        s.app.screen.view.follow()
    });
    both(&mut keep, &mut all, "再跳到中间", |s| s.jump(45));
    both(&mut keep, &mut all, "滚轮往上", |s| s.wheel(true));
    both(&mut keep, &mut all, "拖滚动条到中间", |s| s.bar(14));
}

#[test]
fn far_entries_keep_only_their_length_and_the_geometry_is_unchanged() {
    let (mut keep, mut all) = pair(40);
    keep.draw();
    all.draw();
    let rows = &keep.app.screen.view.rows;
    assert_eq!(rows.kept(0), Some(false), "最上面那一条离视口远，放掉了");
    let last = keep.app.talk.transcript.entries.len() - 1;
    assert_eq!(rows.kept(last), Some(true), "最新的那一条留着");
    assert_eq!(rows.len(), all.app.screen.view.rows.len(), "一共几行一样");
    assert_eq!(keep.spans(), all.spans(), "每一条从第几行到第几行一样");
    let kept = keep.app.media.row_cache.borrow().kept_rows().count();
    let every = all.app.media.row_cache.borrow().kept_rows().count();
    assert!(kept < every / 2, "留着行的条目少了一大半：{kept} / {every}");
    // 全部留着的一条都不放。
    assert!((0..=last).all(|i| all.app.screen.view.rows.kept(i) != Some(false)));
}

#[test]
fn a_far_jump_has_every_viewport_row_in_the_same_frame() {
    let (mut keep, mut all) = pair(40);
    both(&mut keep, &mut all, "头一帧", |_| {});
    assert_eq!(keep.app.screen.view.rows.kept(0), Some(false));
    both(&mut keep, &mut all, "拖滚动条一下到开头", |s| {
        s.bar(0)
    });
    assert_eq!(keep.app.screen.view.first, 0);
    both(&mut keep, &mut all, "回到底部", |s| {
        s.app.screen.view.follow()
    });
    // 再跳之前开头那几条已经又放掉了（离底部超过 4 屏）。
    assert_eq!(keep.app.screen.view.rows.kept(0), Some(false));
    both(&mut keep, &mut all, "/jump 跳到最上面那一句", |s| {
        s.jump(0)
    });
    assert_eq!(keep.app.screen.view.first, 0);
}

#[test]
fn toggling_a_block_in_the_middle_keeps_its_first_row_still() {
    let (mut keep, mut all) = pair(40);
    both(&mut keep, &mut all, "头一帧", |_| {});
    // 跳到第 20 轮你说的话（第 60 条，露着就不钉顶上那一行），它下面第 61 条是一段时间线：点它点开，再点收起。
    both(&mut keep, &mut all, "跳到中间", |s| s.jump(60));
    let at = |s: &Side| {
        let view = &s.app.screen.view;
        view.rows
            .row_of(Target::Segment(61))
            .map(|r| r - view.first)
    };
    let before = at(&keep).expect("时间线那一行在视口里");
    let a = keep.app.screen.areas.body;
    let y = a.y + u16::try_from(before).unwrap();
    for what in ["点开", "收起"] {
        both(&mut keep, &mut all, what, |s| {
            s.mouse(MouseEventKind::Down(MouseButton::Left), a.x + 4, y);
            s.mouse(MouseEventKind::Up(MouseButton::Left), a.x + 4, y);
        });
        assert_eq!(at(&keep), Some(before), "{what}以后那一块的第一行不动");
    }
    assert!(
        keep.app.screen.view.rows.kept(0) == Some(false),
        "远处的照旧放着"
    );
}

/// 拖着选：从视口第 `from` 行按下，拖出正文区的下边 `lines` 次（每次跟着滚一行、画一帧），在下边外面松开；交回松开时
/// 复制的字。
fn select_across(s: &mut Side, from: u16, lines: usize) -> String {
    let a = s.app.screen.areas.body;
    let event = |kind, row| MouseEvent {
        kind,
        column: a.x + 3,
        row,
        modifiers: KeyModifiers::NONE,
    };
    let below = a.bottom();
    s.app
        .screen
        .view
        .mouse(event(MouseEventKind::Down(MouseButton::Left), a.y + from));
    for _ in 0..lines {
        s.app
            .screen
            .view
            .mouse(event(MouseEventKind::Drag(MouseButton::Left), below));
        s.draw();
    }
    match s
        .app
        .screen
        .view
        .mouse(event(MouseEventKind::Up(MouseButton::Left), below))
    {
        crate::body_view::BodyAction::Copy(text) => text,
        other => panic!("松开该复制：{other:?}"),
    }
}

#[test]
fn copying_a_selection_across_shed_rows_gives_the_same_text() {
    let (mut keep, mut all) = pair(40);
    both(&mut keep, &mut all, "头一帧", |_| {});
    both(&mut keep, &mut all, "拖滚动条到开头", |s| s.bar(0));
    let height = usize::from(keep.app.screen.areas.body.height);
    let middle = keep.app.screen.view.rows.entry_at(3 * height).unwrap();
    assert_eq!(
        keep.app.screen.view.rows.kept(middle),
        Some(false),
        "中间那几屏原来放掉了"
    );
    // 从开头选起，拖出下边自动滚六屏：中间那几屏原来放掉了，滚过去的时候才排出来。
    let lines = 6 * height;
    let copied = select_across(&mut keep, 0, lines);
    assert_eq!(copied, select_across(&mut all, 0, lines));
    assert!(copied.contains("第 0 句"), "从开头选起：{copied}");
    assert!(copied.lines().count() > 20, "跨了好几屏：{copied}");
    // 选好以后滚到最底下（离选区远过留着的范围）再复制：选区盖到的条目一直留着。
    both(&mut keep, &mut all, "回到底部", |s| {
        s.app.screen.view.follow()
    });
    for _ in 0..3 {
        both(&mut keep, &mut all, "底部再画几帧", |_| {});
    }
    assert_eq!(
        keep.app.screen.view.rows.kept(0),
        Some(true),
        "选区盖到的不放"
    );
    assert_eq!(keep.app.screen.view.selected_text(), copied);
    assert_eq!(all.app.screen.view.selected_text(), copied);
    // 放开选区以后，远处的照常放掉。
    keep.app.screen.view.select = None;
    keep.draw();
    assert_eq!(keep.app.screen.view.rows.kept(0), Some(false));
}

#[test]
fn after_a_width_change_settles_lengths_and_the_viewport_match_keeping_everything() {
    let (mut keep, mut all) = pair(40);
    // 每帧只排一条：分帧重排要好多帧才排完。
    for s in [&mut keep, &mut all] {
        s.app.config.layout.relayout_budget_ms = 0;
    }
    both(&mut keep, &mut all, "头一帧", |_| {});
    both(&mut keep, &mut all, "跳到中间", |s| s.jump(60));
    for s in [&mut keep, &mut all] {
        s.term.backend_mut().resize(50, 30);
    }
    let mut frames = 0;
    loop {
        keep.draw();
        all.draw();
        assert!(keep.viewport_has_rows(), "分帧重排的时候视口里也都有行");
        frames += 1;
        let settled = |s: &Side| s.app.media.row_cache.borrow().stale == 0;
        if settled(&keep) && settled(&all) {
            break;
        }
        assert!(frames < 400, "排得完");
    }
    // 排完了再画一帧：底边、视口照排好的定下来。
    both(&mut keep, &mut all, "排完以后", |_| {});
    assert_eq!(keep.spans(), all.spans(), "每一条的行数和全部留着的一样");
    // 和一开始就在这个宽度、整份排一遍的一样。
    let mut fresh = side(0, 40, 50);
    fresh.draw();
    assert_eq!(
        keep.spans(),
        fresh.spans(),
        "每一条的行数和重新整份排的一样"
    );
}
