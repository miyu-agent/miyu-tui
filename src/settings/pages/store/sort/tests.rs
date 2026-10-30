use super::super::read::Remote;
use super::{Sort, available, next, shown};

fn remote(id: &str, published: Option<&str>, downloads: Option<u64>, week: Option<u64>) -> Remote {
    Remote {
        id: id.into(),
        name: id.into(),
        published: published.map(str::to_string),
        downloads,
        week,
        ..Remote::default()
    }
}

/// 核心设计 54 第四节：从大到小，没有这一格的排在最后、再照编号。
#[test]
fn each_sort_puts_the_biggest_first_and_the_missing_last_by_id() {
    let list = [
        remote("b", Some("2026-10-20T00:00:00Z"), Some(10), None),
        remote("a", None, Some(300), Some(5)),
        remote("c", Some("2026-10-31T00:00:00Z"), None, Some(40)),
        remote("d", None, None, None),
    ];
    let ids = |sort| -> Vec<&str> {
        shown(&list, sort, "")
            .into_iter()
            .map(|i| list[i].id.as_str())
            .collect()
    };
    assert_eq!(ids(Some(Sort::Recent)), ["c", "b", "a", "d"]);
    assert_eq!(ids(Some(Sort::Downloads)), ["a", "b", "c", "d"]);
    assert_eq!(ids(Some(Sort::Trend)), ["c", "a", "b", "d"]);
    assert_eq!(ids(None), ["b", "a", "c", "d"], "不排的照软件源给的先后");
}

/// 一个包都没有某一格的那一种不给；`s` 在能用的里面轮换。
#[test]
fn a_sort_nobody_has_is_skipped() {
    let list = [remote("a", Some("2026-10-01T00:00:00Z"), None, Some(3))];
    assert_eq!(available(&list), [Sort::Recent, Sort::Trend]);
    assert_eq!(next(Some(Sort::Recent), &list), Some(Sort::Trend));
    assert_eq!(next(Some(Sort::Trend), &list), Some(Sort::Recent));
    assert_eq!(
        next(Some(Sort::Downloads), &list),
        Some(Sort::Recent),
        "不能用的从第一种起"
    );
    assert_eq!(
        next(None, &[remote("old", None, None, None)]),
        None,
        "旧索引一种都没有"
    );
}

#[test]
fn every_word_must_appear_in_the_name_id_or_summary() {
    let mut weather = remote("miyu-scripts-weather", None, None, None);
    weather.name = "天气".into();
    weather.summary = "查城市天气和预报".into();
    let mut tarot = remote("miyu-scripts-tarot", None, None, None);
    tarot.summary = "抽牌".into();
    let list = [weather, tarot];
    let hit = |q: &str| shown(&list, None, q).len();
    assert_eq!(hit("天气 预报"), 1);
    assert_eq!(hit("SCRIPTS"), 2, "不分大小写，编号也算");
    assert_eq!(hit("天气 抽牌"), 0, "每个词都得有");
    assert_eq!(hit("  "), 2);
}
