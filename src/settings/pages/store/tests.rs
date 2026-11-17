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

/// 不知道核心程序在哪时，「软件后台」`Enter` 叫人自己跑 `miyu pkg page <编号>`（发版前 check 14，2026-11-09 验收时项目主人：
/// 金色提示还是旧的 `miyu web --package`）。
#[test]
fn without_the_core_program_the_hint_names_pkg_page() {
    let texts = crate::config::Config::builtin().unwrap().text.settings;
    let line = super::keys::run_page(&texts, "im");
    assert_eq!(line, "在终端里运行 miyu pkg page im");
}

/// 2026-11-17 项目主人验收第 10 条：联网搜索的名字只有连接语言的（「联网搜索」），筛 `web` 要靠功能编号 `web-search` 才找得到；
/// `-`、`_`、`.` 当空格。
#[test]
fn the_installed_filter_also_reads_feature_ids() {
    let got = serde_json::json!({"packages": [
        {"package": "base-net", "name": "联网搜索", "summary": "", "features": [
            {"id": "web-search", "name": "网络搜索"}, {"id": "web-fetch", "name": "抓取网页"}]},
        {"package": "memes", "name": "表情包"}]});
    let mut store = Store {
        tab: Tab::Installed,
        local: Some(super::read::locals(&got)),
        ..Store::default()
    };
    let ids = |s: &Store| -> Vec<String> { s.rows().iter().map(|r| r.id().to_string()).collect() };
    for query in ["web", "WEB search", "抓取"] {
        store.query = query.into();
        assert_eq!(ids(&store), ["base-net"], "筛「{query}」");
    }
}

/// 归包的配置项的键、名字也认（联网搜索的键是 `net.*`），「下载软件」照包带的东西的名字认。
#[test]
fn the_filters_also_read_setting_keys_and_what_a_package_brings() {
    let mut store = store();
    store.tab = Tab::Installed;
    let schema = super::super::schema::Schema::read(&serde_json::json!({
        "pages": [{"id": "packages", "name": "软件包"}],
        "groups": [{"id": "b", "name": "B", "page": "packages"}],
        "items": [{"key": "net.search.source", "type": "text", "control": "select", "layers": ["system"],
                   "name": "联网服务", "page": "packages", "group": "b", "package": "b"}]}));
    store.note_keys(&schema);
    let ids = |s: &Store| -> Vec<String> { s.rows().iter().map(|r| r.id().to_string()).collect() };
    for query in ["net search", "联网服务"] {
        store.query = query.into();
        assert_eq!(ids(&store), ["b"], "筛「{query}」");
    }
    store.tab = Tab::Source;
    if let Listing::Got(list) = &mut store.source {
        list[1].provides = vec![("features".into(), vec!["web-search".into()])];
    }
    store.query = "web".into();
    assert_eq!(ids(&store), ["y"]);
}
