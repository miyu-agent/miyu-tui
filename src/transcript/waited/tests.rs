use std::time::{Duration, Instant};

use serde_json::json;

use super::Waited;
use crate::core::Asking;

fn at(t0: Instant, secs: u64) -> Instant {
    t0 + Duration::from_secs(secs)
}

#[test]
fn waiting_on_a_person_counts_from_the_ask_until_answered_or_a_result() {
    let t0 = Instant::now();
    let mut w = Waited::default();
    w.asking(&Asking::Approval(json!({"call_id": "a"})), at(t0, 2));
    assert!(w.waiting());
    assert_eq!(
        w.total(at(t0, 5)),
        Duration::from_secs(3),
        "还在等的算到现在"
    );
    w.asking(&Asking::Decided(json!({"call_id": "a"})), at(t0, 7));
    assert!(!w.waiting());
    assert_eq!(w.total(at(t0, 30)), Duration::from_secs(5), "定了就不再算");
    // 提问没等到答，那一次调用先有了结果（别处答的、超时的）。
    w.asking(&Asking::Asked(json!({"call_id": "q"})), at(t0, 40));
    w.end("q", at(t0, 41));
    assert_eq!(w.total(at(t0, 50)), Duration::from_secs(6));
}

#[test]
fn overlapping_waits_count_once() {
    let t0 = Instant::now();
    let mut w = Waited::default();
    w.asking(&Asking::Approval(json!({"call_id": "a"})), at(t0, 0));
    w.asking(&Asking::Approval(json!({"call_id": "b"})), at(t0, 2));
    w.asking(&Asking::Decided(json!({"call_id": "a"})), at(t0, 4));
    assert!(w.waiting(), "b 还在等");
    w.asking(&Asking::Decided(json!({"call_id": "b"})), at(t0, 6));
    assert_eq!(w.total(at(t0, 9)), Duration::from_secs(6));
    // 不认识的调用（这一轮开始前问的）有了结果：不动。
    w.end("old", at(t0, 10));
    assert_eq!(w.total(at(t0, 10)), Duration::from_secs(6));
    w.reset();
    assert_eq!(w.total(at(t0, 11)), Duration::ZERO, "新的一轮从零算");
}
