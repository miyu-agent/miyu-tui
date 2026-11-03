use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::text::Line;

use super::{Laid, Part, stack};
use crate::config::slots::Slot;

/// 各件画在哪：（部件名，给它的那一块，露出来几行）。
type Placed = Vec<(&'static str, Rect, usize)>;

/// 照字画几行的部件；`rows` 是 0 的这一回没东西画。
struct Text(&'static str, usize);

impl Part<Placed> for Text {
    fn name(&self) -> &'static str {
        self.0
    }

    fn lay(&mut self, _: &Placed, _: u16) -> Laid {
        Laid::Lines(
            (0..self.1)
                .map(|i| Line::raw(format!("{}{i}", self.0)))
                .collect(),
        )
    }

    fn placed(&mut self, _: &mut ratatui::Frame, area: Rect, shown: usize, seen: &mut Placed) {
        seen.push((self.0, area, shown));
    }
}

/// 自己画、占几行的部件。
struct Picture(u16);

impl Part<Placed> for Picture {
    fn name(&self) -> &'static str {
        "pic"
    }

    fn lay(&mut self, _: &Placed, _: u16) -> Laid {
        Laid::Rows(self.0)
    }

    fn placed(&mut self, _: &mut ratatui::Frame, area: Rect, shown: usize, seen: &mut Placed) {
        seen.push(("pic", area, shown));
    }
}

fn slot(part: &str, hidden: bool, rows: u16) -> Slot {
    Slot {
        part: part.into(),
        hidden,
        rows,
    }
}

/// 画一帧：交回排到的下一行、照字画出来的字、各件拿到的那一块。
fn run(
    config: &[Slot],
    parts: &mut [&mut dyn Part<Placed>],
    height: u16,
) -> (u16, Vec<String>, Placed) {
    let mut term = Terminal::new(TestBackend::new(20, height)).unwrap();
    let mut placed = Vec::new();
    let mut out = None;
    term.draw(|frame| {
        let area = Rect::new(0, 1, 20, height - 1);
        out = Some(stack(frame, area, parts, (config, 1), &mut placed));
    })
    .unwrap();
    let stacked = out.unwrap();
    let words = stacked.written.into_iter().map(|(_, _, t)| t).collect();
    (stacked.next, words, placed)
}

#[test]
fn parts_go_in_configured_order_with_a_gap_and_empty_ones_take_no_room() {
    let (mut a, mut b, mut c) = (Text("a", 2), Text("b", 0), Text("c", 1));
    let (next, words, placed) = run(&[slot("c", false, 0)], &mut [&mut a, &mut b, &mut c], 20);
    assert_eq!(words, ["c0", "a0", "a1"], "写了的先，没写的照出厂接后面");
    assert_eq!(placed[0], ("c", Rect::new(0, 1, 20, 1), 1));
    assert_eq!(placed[1], ("a", Rect::new(0, 3, 20, 2), 2), "空一行");
    assert_eq!(placed.len(), 2, "没东西画的不占行");
    assert_eq!(next, 6, "最后一件下面空一行再接");
}

#[test]
fn hidden_parts_drop_and_rows_cap_ends_with_an_ellipsis() {
    let (mut a, mut b, mut p) = (Text("a", 5), Text("b", 1), Picture(6));
    let config = [
        slot("a", false, 3),
        slot("b", true, 0),
        slot("pic", false, 2),
    ];
    let (_, words, placed) = run(&config, &mut [&mut a, &mut b, &mut p], 20);
    assert_eq!(
        words,
        ["a0", "a1", "…"],
        "截到 3 行，最后一行写 …；藏起来的不画"
    );
    assert_eq!(
        placed[1],
        ("pic", Rect::new(0, 5, 20, 2), 2),
        "自己画的给截过的高"
    );
}

#[test]
fn what_does_not_fit_is_cut_and_says_how_much_showed() {
    let (mut a, mut b) = (Text("a", 3), Text("b", 3));
    let (_, words, placed) = run(&[], &mut [&mut a, &mut b], 6);
    assert_eq!(words, ["a0", "a1", "a2", "b0"], "放不下的截掉");
    assert_eq!(placed[1], ("b", Rect::new(0, 5, 20, 1), 1));
}

#[test]
fn a_column_lays_out_lines_in_order_with_caps_and_gaps() {
    use super::column;
    let lines = |prefix: &str, n: usize| -> Vec<Line<'static>> {
        (0..n).map(|i| Line::raw(format!("{prefix}{i}"))).collect()
    };
    let parts = || {
        vec![
            ("a", lines("a", 3)),
            ("b", Vec::new()),
            ("c", lines("c", 1)),
        ]
    };
    let text =
        |out: Vec<Line<'static>>| -> Vec<String> { out.iter().map(ToString::to_string).collect() };
    assert_eq!(
        text(column(parts(), (&[], 1))),
        ["a0", "a1", "a2", "", "c0"],
        "空的件不占行"
    );
    let config = [slot("c", false, 0), slot("a", false, 2)];
    assert_eq!(text(column(parts(), (&config, 1))), ["c0", "", "a0", "…"]);
    assert!(column(parts(), (&[slot("a", true, 0), slot("c", true, 0)], 1)).is_empty());
}
