use super::{Media, Scope, View};
use crate::config::Config;
use crate::human::Human;
use crate::transcript::{Entry, JobMark, Kind, Transcript};

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
    view.rows(transcript, scope)
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
