//! 主循环为放掉缓冲醒来的点：画完过了 `screen_park_ms` 没再画才放；中间又画了顺延；放了以后不再醒；配成 0 不放。

use std::cell::RefCell;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use super::Parking;
use crate::screen::Screen;
use crate::screen::wire::{Wire, frame};

/// 画过一帧（留着缓冲）的画面。
fn drawn() -> Screen<Wire> {
    let mut screen = Screen::new(Wire::new(10, 3));
    frame(&mut screen, |_| None);
    assert!(screen.active());
    screen
}

/// 照 `script` 一次次回话的等法：`Some` 是来了一件事，`None` 是等满了给的时长（真的睡那么久）；记下每次要等多久。
fn waiter<'a>(
    script: &'a RefCell<Vec<Option<u32>>>,
    asked: &'a RefCell<Vec<Duration>>,
) -> impl FnMut(Duration) -> Result<u32, RecvTimeoutError> + 'a {
    move |wait| {
        asked.borrow_mut().push(wait);
        match script.borrow_mut().remove(0) {
            Some(message) => Ok(message),
            None => {
                std::thread::sleep(wait);
                Err(RecvTimeoutError::Timeout)
            }
        }
    }
}

#[test]
fn it_parks_after_the_quiet_time_and_keeps_waiting_without_a_frame() {
    let mut screen = drawn();
    let mut parking = Parking::new(30);
    parking.drawn(Instant::now());
    let script = RefCell::new(vec![None, Some(7)]);
    let asked = RefCell::new(Vec::new());
    let got = parking.wait(&mut screen, None, waiter(&script, &asked));
    assert_eq!(got, Ok(Some(7)), "放掉以后接着等，等到了事情才交回");
    assert!(!screen.active(), "放掉了");
    let asked = asked.borrow();
    assert!(
        asked[0] <= Duration::from_millis(30),
        "头一回只等到放的点：{asked:?}"
    );
    assert_eq!(asked[1], Duration::from_secs(3600), "放了以后不再为它醒");
    assert_eq!(parking.due(), None);
}

#[test]
fn drawing_again_moves_the_point_later() {
    let mut parking = Parking::new(2000);
    let start = Instant::now();
    parking.drawn(start);
    parking.drawn(start + Duration::from_millis(500));
    assert_eq!(parking.due(), Some(start + Duration::from_millis(2500)));
}

#[test]
fn the_main_loop_point_coming_first_returns_without_parking() {
    let mut screen = drawn();
    let mut parking = Parking::new(10_000);
    parking.drawn(Instant::now());
    let script = RefCell::new(vec![None]);
    let asked = RefCell::new(Vec::new());
    let soon = Instant::now() + Duration::from_millis(5);
    let got = parking.wait(&mut screen, Some(soon), waiter(&script, &asked));
    assert_eq!(got, Ok(None), "主循环自己的点到了，交回去画");
    assert!(screen.active(), "还没到放的点");
    assert!(parking.due().is_some());
}

#[test]
fn zero_never_parks() {
    let mut screen = drawn();
    let mut parking = Parking::new(0);
    parking.drawn(Instant::now());
    assert_eq!(parking.due(), None);
    let script = RefCell::new(vec![Some(1)]);
    let asked = RefCell::new(Vec::new());
    assert_eq!(
        parking.wait(&mut screen, None, waiter(&script, &asked)),
        Ok(Some(1))
    );
    assert_eq!(asked.borrow()[0], Duration::from_secs(3600));
    assert!(screen.active());
}

#[test]
fn a_closed_channel_is_reported() {
    let mut screen = drawn();
    let mut parking = Parking::new(100);
    let got: Result<Option<u32>, _> =
        parking.wait(&mut screen, None, |_| Err(RecvTimeoutError::Disconnected));
    assert_eq!(got, Err(RecvTimeoutError::Disconnected));
}
