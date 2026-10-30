use super::read::{Local, Remote};
use super::{Listing, Store, Tab};

fn local(id: &str, page: bool) -> Local {
    Local {
        id: id.into(),
        name: id.into(),
        on: true,
        page,
        ..Local::default()
    }
}

fn store() -> Store {
    let remote = |id: &str| Remote {
        id: id.into(),
        name: id.into(),
        ..Remote::default()
    };
    Store {
        source: Listing::Got(vec![remote("x"), remote("y")]),
        local: Some(vec![local("a", false), local("b", true), local("c", true)]),
        ..Store::default()
    }
}

#[test]
fn each_page_lists_its_own_and_turning_stops_at_the_ends() {
    let mut store = store();
    let ids = |s: &Store| -> Vec<String> { s.rows().iter().map(|r| r.id().to_string()).collect() };
    assert_eq!(ids(&store), ["x", "y"]);
    store.turn(false);
    assert_eq!(store.tab, Tab::Source, "最左边再往左停着");
    store.turn(true);
    assert_eq!(ids(&store), ["a", "b", "c"], "已安装列全部");
    store.turn(true);
    assert_eq!(ids(&store), ["b", "c"], "软件后台只列带后台页的");
    store.turn(true);
    assert_eq!(store.tab, Tab::Backstage, "最右边再往右停着");
}

#[test]
fn the_cursor_stays_in_the_list_and_comes_back_to_the_same_package() {
    let mut store = store();
    store.tab = Tab::Installed;
    store.go(9);
    assert_eq!(store.cursor(), 2);
    store.keep = Some((Tab::Installed, "b".into()));
    store.local = Some(vec![local("c", true), local("b", true)]);
    store.settle();
    assert_eq!(
        store.current().map(|r| r.id().to_string()).as_deref(),
        Some("b"),
        "重读以后停回它"
    );
    assert_eq!(store.keep, None);
}

#[test]
fn details_are_asked_once_per_package_and_not_for_the_backstage() {
    let mut store = store();
    assert_eq!(store.wants_info(), Some((true, "x".into())));
    assert_eq!(store.wants_info(), None, "要过的不再要");
    store.tab = Tab::Installed;
    assert_eq!(store.wants_info(), Some((false, "a".into())));
    store.tab = Tab::Backstage;
    assert_eq!(store.wants_info(), None);
}
