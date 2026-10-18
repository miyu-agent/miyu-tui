use std::time::{Duration, Instant};

use super::Trim;

const AFTER: Duration = Duration::from_secs(2);

#[test]
fn a_big_drop_releases_once_after_a_pause() {
    let t0 = Instant::now();
    let mut trim = Trim::default();
    trim.rows(100, t0, 200, AFTER);
    trim.rows(900, t0, 200, AFTER);
    assert_eq!(trim.deadline(), None, "只是变多不还");
    trim.rows(150, t0, 200, AFTER);
    assert_eq!(trim.deadline(), Some(t0 + AFTER), "少了一大截：两秒后还");
    assert!(!trim.take(t0 + Duration::from_secs(1)), "还没到点");
    assert!(trim.take(t0 + AFTER));
    assert!(!trim.take(t0 + AFTER * 2), "只还一次");
    assert_eq!(trim.deadline(), None);
}

#[test]
fn a_small_drop_does_nothing() {
    let t0 = Instant::now();
    let mut trim = Trim::default();
    trim.rows(500, t0, 200, AFTER);
    trim.rows(350, t0, 200, AFTER);
    assert_eq!(trim.deadline(), None, "少得不多不还");
    trim.rows(290, t0, 200, AFTER);
    assert_eq!(trim.deadline(), Some(t0 + AFTER), "从最多时算起少了 210 行");
}
