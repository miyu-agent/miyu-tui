use super::{Media, Scope, View};
use crate::config::Config;
use crate::human::Human;
use crate::transcript::{Entry, JobMark, Kind, Segment, Step, StepKind, Transcript};

fn said(transcript: &mut Transcript, kind: Kind, turn: Option<u64>, text: &str) {
    let id = transcript.entries.len() as u64 + 1;
    let mut entry = Entry::new(id, kind, text.into());
    entry.turn = turn;
    transcript.entries.push(entry);
}

fn plain(scope: Scope, transcript: &Transcript) -> String {
    let config = Config::builtin().unwrap();
    let human = Human::default();
    let media = Media::new(&config);
    let board = crate::jobs::Board::default();
    let view = View {
        config: &config,
        human: &human,
        media: &media,
        board: &board,
        width: 80,
        height: 40,
        frame: 0,
    };
    view.rows_held(transcript, scope)
        .0
        .iter()
        .map(|r| r.plain.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn only_the_turns_asked_for_are_laid_out() {
    // 后台回报接上叫醒她的那一轮：后面你说话开的那一轮不印（2026-10-20 验收：bash 里印了两遍）。
    let mut transcript = Transcript::default();
    said(&mut transcript, Kind::Reply, Some(5), "叫醒的那一轮");
    transcript.job(JobMark::Done, "任务结束的那一行".into(), String::new());
    said(&mut transcript, Kind::User, None, "进度如何了");
    said(&mut transcript, Kind::Reply, Some(9), "你说话开的那一轮");
    let woken = plain(Scope::Turns(vec![5]), &transcript);
    assert!(woken.contains("叫醒的那一轮"));
    assert!(woken.contains("任务结束的那一行"), "不带回合号的照样排");
    assert!(!woken.contains("你说话开的那一轮"));
    let both = plain(Scope::Turns(vec![5, 9]), &transcript);
    assert!(both.contains("你说话开的那一轮"));
    assert!(plain(Scope::Turns(Vec::new()), &transcript).is_empty());
}

#[test]
fn lines_from_an_unsettled_timeline_on_are_held_back_from_the_scrollback() {
    // 「怎么印」第 2 条：没收好的时间线还会收成一行，它和它下面的不能先印进终端的滚动历史。
    let config = Config::builtin().unwrap();
    let human = Human::default();
    let media = Media::new(&config);
    let board = crate::jobs::Board::default();
    let view = View {
        config: &config,
        human: &human,
        media: &media,
        board: &board,
        width: 80,
        height: 40,
        frame: 0,
    };
    let mut transcript = Transcript::default();
    said(&mut transcript, Kind::Reply, Some(5), "前面那段");
    let mut steps = Entry::new(9, Kind::Steps, String::new());
    steps.turn = Some(5);
    let mut segment = Segment::new();
    segment.steps.push(Step::new(StepKind::Thought {
        text: "想一想".into(),
    }));
    steps.segment = Some(segment);
    transcript.entries.push(steps);
    said(&mut transcript, Kind::Reply, Some(5), "后面那段");
    let text = |lines: &[ratatui::text::Line]| {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let (lines, held) = view.lines_held(&transcript, Scope::Turns(vec![5]), false);
    assert!(text(&lines[..held]).contains("前面那段"));
    assert!(!text(&lines[..held]).contains("后面那段"));
    assert!(text(&lines).contains("后面那段"));
    if let Some(segment) = &mut transcript.entries[1].segment {
        segment.finish_at(std::time::Instant::now());
    }
    let (lines, held) = view.lines_held(&transcript, Scope::Turns(vec![5]), false);
    assert_eq!(held, lines.len(), "收好了的都定下来了");
}
