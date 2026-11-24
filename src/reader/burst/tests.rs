use std::time::{Duration, Instant};

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::{Burst, CHAR_GAP, STREAM_GAP};
use crate::reader::Read;

/// 按下一个键。
fn press(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// 松开一个键（Windows 上每个键都报）。
fn release(code: KeyCode) -> Event {
    let mut key = KeyEvent::new(code, KeyModifiers::NONE);
    key.kind = KeyEventKind::Release;
    Event::Key(key)
}

/// 一个字、回车、`Tab` 的按下。
fn keys(text: &str) -> Vec<Event> {
    text.chars()
        .map(|c| match c {
            '\r' => press(KeyCode::Enter),
            '\t' => press(KeyCode::Tab),
            c => press(KeyCode::Char(c)),
        })
        .collect()
}

/// 照 `gap` 一个一个喂（过了点的 `feed` 自己先收掉上一串），最后到点收掉；交回主循环拿到的全部。
fn feed_all(burst: &mut Burst, events: Vec<Event>, gap: Duration) -> Vec<Event> {
    let mut now = Instant::now();
    let mut out = Vec::new();
    for event in events {
        out.extend(burst.feed(Read::Event(event), now));
        now += gap;
    }
    out.extend(burst.expire());
    events_of(out)
}

/// 交给主循环的都是事件。
fn events_of(reads: Vec<Read>) -> Vec<Event> {
    reads
        .into_iter()
        .map(|r| match r {
            Read::Event(e) => e,
            Read::Reply(_) => panic!("不该有回话"),
        })
        .collect()
}

#[test]
fn keys_arriving_together_with_a_return_become_one_paste() {
    let mut burst = Burst::default();
    let out = feed_all(&mut burst, keys("第一行\r第二行\r第三行"), Duration::ZERO);
    assert_eq!(out, vec![Event::Paste("第一行\n第二行\n第三行".into())]);
}

#[test]
fn typing_at_human_speed_stays_keys_and_return_still_sends() {
    let mut burst = Burst::default();
    let typed = keys("hi\r");
    let out = feed_all(&mut burst, typed.clone(), Duration::from_millis(60));
    assert_eq!(out, typed, "人打字：照原样一个一个，回车还是回车");
}

#[test]
fn a_fast_run_without_a_return_stays_keys() {
    // 打得快的、输入法一次上屏的字：没有回车的一串照原样交回，和原来一样。
    let mut burst = Burst::default();
    let typed = keys("你好世界");
    assert_eq!(feed_all(&mut burst, typed.clone(), Duration::ZERO), typed);
}

#[test]
fn a_return_right_after_one_key_is_still_a_return() {
    // 一串不到三个：两个键连着按得再快，回车也照旧是发送。
    let mut burst = Burst::default();
    let typed = keys("a\r");
    assert_eq!(feed_all(&mut burst, typed.clone(), Duration::ZERO), typed);
}

#[test]
fn a_return_only_at_the_end_still_sends() {
    // 字打完紧跟一个回车（语音输入、自动输入的工具这样发一句）：照原样交回，回车照旧是发送。
    let mut burst = Burst::default();
    let typed = keys("hello\r");
    assert_eq!(feed_all(&mut burst, typed.clone(), Duration::ZERO), typed);
}

#[test]
fn a_return_with_text_right_behind_it_still_sends() {
    // 回车打头不算一串：发出去以后马上又进来的字是下一句，不和回车并成一段。
    let mut burst = Burst::default();
    let out = feed_all(&mut burst, keys("\rnext line"), Duration::ZERO);
    assert_eq!(out, keys("\rnext line"));
}

#[test]
fn releases_between_presses_do_not_break_the_run() {
    // Windows 上粘贴是按下、松开交替来的：松开照旧交出去，按下的照样连成一串。
    let mut burst = Burst::default();
    let mut events = Vec::new();
    for e in keys("ab\rcd") {
        let Event::Key(k) = &e else { unreachable!() };
        let code = k.code;
        events.push(e);
        events.push(release(code));
    }
    let out = feed_all(&mut burst, events, Duration::ZERO);
    let pastes: Vec<_> = out
        .iter()
        .filter(|e| matches!(e, Event::Paste(_)))
        .collect();
    assert_eq!(pastes, vec![&Event::Paste("ab\ncd".into())]);
    assert!(
        out.iter().all(|e| match e {
            Event::Key(k) => k.kind == KeyEventKind::Release,
            _ => true,
        }),
        "按下的都进了粘贴：{out:?}"
    );
}

#[test]
fn a_return_and_line_feed_pair_is_one_line_break() {
    let mut burst = Burst::default();
    let mut events = keys("ab\r");
    events.push(press(KeyCode::Char('\n')));
    events.extend(keys("cd"));
    let out = feed_all(&mut burst, events, Duration::ZERO);
    assert_eq!(out, vec![Event::Paste("ab\ncd".into())]);
}

#[test]
fn another_key_ends_the_run_first() {
    // 方向键这种不写字的：先把攒着的交出去，再交它。
    let mut burst = Burst::default();
    let mut events = keys("ab\rc");
    events.push(press(KeyCode::Left));
    let out = feed_all(&mut burst, events, Duration::ZERO);
    assert_eq!(
        out,
        vec![Event::Paste("ab\nc".into()), press(KeyCode::Left)]
    );
}

#[test]
fn a_pause_longer_than_the_gap_starts_a_new_run() {
    // 字打完停一下再回车：回车是发送，不并进前面那一串。
    let mut burst = Burst::default();
    let now = Instant::now();
    let mut out = Vec::new();
    for e in keys("abc") {
        out.extend(burst.feed(Read::Event(e), now));
    }
    let later = now + STREAM_GAP + CHAR_GAP + Duration::from_millis(1);
    out.extend(burst.feed(Read::Event(press(KeyCode::Enter)), later));
    out.extend(burst.expire());
    assert_eq!(events_of(out), keys("abc\r"));
}

#[test]
fn a_bracketed_paste_passes_through_untouched() {
    let mut burst = Burst::default();
    let paste = Event::Paste("x\ny".into());
    assert_eq!(
        feed_all(&mut burst, vec![paste.clone()], Duration::ZERO),
        vec![paste]
    );
}
