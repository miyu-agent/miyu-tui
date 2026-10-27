//! 功能平铺、切一行写哪几个键（「配置页」第 38 条）。

use serde_json::json;

use super::{
    Feature, Mark, Packages, Row, apply, group_name, mark, name_packages, offs, read, rows,
    selectable, toggle, toggle_all,
};

/// 文件读写（几件工具）、运行命令（一件）、人设防失忆提醒（没有工具）、没装的联网。
fn sample() -> Vec<super::Feature> {
    read(&json!({"features": [
        {"id": "files", "name": "文件读写", "on": true, "installed": true, "tools": [
            {"name": "glob", "label": "找文件", "on": true},
            {"name": "grep", "label": "搜内容", "on": false},
            {"name": "read", "label": "读取", "on": true}]},
        {"id": "commands", "name": "运行命令", "on": false, "installed": true, "tools": [
            {"name": "shell", "label": "执行命令", "on": false}]},
        {"id": "roleplay", "name": "人设防失忆提醒", "on": true, "installed": true, "tools": []},
        {"id": "web", "name": "联网", "on": true, "installed": false, "tools": []}]}))
}

#[test]
fn features_lie_flat_with_their_tools_under_them() {
    // 2026-10-09 项目主人：「每一个工具都是一个功能，直接平铺，然后用组名做分隔线」。
    let features = sample();
    assert_eq!(
        rows(&features),
        [
            Row::Feature(0),
            Row::Tool(0, 0),
            Row::Tool(0, 1),
            Row::Tool(0, 2),
            Row::Feature(1),
            Row::Feature(2),
            Row::Feature(3)
        ],
        "只有一件、没有工具的只有功能那一行"
    );
    assert_eq!(mark(&features[0]), Mark::Some, "关了一件工具");
    assert_eq!(mark(&features[1]), Mark::Off);
    assert_eq!(mark(&features[2]), Mark::On);
    assert!(!selectable(&features, Row::Feature(3)), "没装的切不了");
}

#[test]
fn space_writes_the_feature_or_the_tool_and_ctrl_a_flips_everything() {
    let features = sample();
    assert_eq!(
        toggle(&features, Row::Tool(0, 1)),
        [("tools.grep".to_string(), true)]
    );
    assert_eq!(
        toggle(&features, Row::Feature(0)),
        [("features.files".to_string(), false)]
    );
    // 功能关着时开它的一件工具：功能打开、只开这一件，别的关上（2026-10-23 项目主人：「激活读取会激活整个文件读写」）。
    let mut off = features.clone();
    off[0].on = false;
    let key = |k: &str, on: bool| (k.to_string(), on);
    assert_eq!(
        toggle(&off, Row::Tool(0, 0)),
        [key("features.files", true), key("tools.read", false)],
        "本来关着的 grep 不用再写"
    );
    assert_eq!(
        toggle(&off, Row::Tool(0, 1)),
        [
            key("features.files", true),
            key("tools.glob", false),
            key("tools.grep", true),
            key("tools.read", false)
        ],
        "自己原来关着的照样打开"
    );
    assert!(toggle(&features, Row::Feature(3)).is_empty(), "没装的不写");
    // 有没开的：全开（关着的功能、开着的功能里关掉的工具）。
    let all_on = toggle_all(&features);
    assert_eq!(
        all_on,
        [
            ("tools.grep".to_string(), true),
            ("features.commands".to_string(), true)
        ]
    );
    let mut on = features.clone();
    apply(&mut on, &all_on);
    assert!(
        on[1].on && on[0].tools.iter().all(|t| t.on),
        "照写的键改手上的"
    );
    // 核心那边打开功能以后它的工具回到各自原来的开关：这里当都开着。
    on[1].tools[0].on = true;
    assert!(offs(&on).is_empty(), "都开着：新建什么都不用写");
    // 都开着：全关功能。
    assert_eq!(
        toggle_all(&on),
        [
            ("features.files".to_string(), false),
            ("features.commands".to_string(), false),
            ("features.roleplay".to_string(), false)
        ]
    );
    assert_eq!(
        offs(&features),
        [
            ("tools.grep".to_string(), false),
            ("features.commands".to_string(), false)
        ]
    );
}

#[test]
fn features_group_under_their_package_and_lone_packages_get_no_heading() {
    // 核心 ENV-1：功能带 `package`，开关挂在包名下面；BS-1 起基础系统拆成一包一个功能，组名和功能名重复，不写组名。
    let got = serde_json::json!({"features": [
        {"id": "env-time", "name": "时间", "on": true, "package": "env"},
        {"id": "read", "name": "读取", "on": true, "package": "read"},
        {"id": "loose", "name": "散的技能", "on": true},
        {"id": "env-cwd", "name": "工作目录", "on": true, "package": "env"},
        {"id": "mine", "name": "我的脚本", "on": true, "package": "~home"},
        {"id": "mine2", "name": "我的命令", "on": true, "package": "~home"}
    ]});
    let mut list = read(&got);
    let names = Packages {
        names: std::collections::HashMap::from([("env".to_string(), "注入环境信息".to_string())]),
        ..Packages::default()
    };
    name_packages(&mut list, &names);
    let rows = rows(&list);
    assert_eq!(
        rows,
        [
            Row::Feature(2),
            Row::Group(0),
            Row::Feature(0),
            Row::Feature(3),
            Row::Feature(1),
            Row::Group(4),
            Row::Feature(4),
            Row::Feature(5),
        ],
        "不归包的在最前；包照第一次出现的先后；只有一个功能的包不写组名"
    );
    assert_eq!(group_name(&list[0], "我的"), "注入环境信息");
    assert_eq!(group_name(&list[4], "我的"), "我的", "机主私人的");
    assert!(!selectable(&list, Row::Group(0)), "组名选不上");
    assert!(toggle(&list, Row::Group(0)).is_empty());
    // 旧核心一个都不带：照旧平铺。
    let old =
        read(&serde_json::json!({"features": [{"id": "a", "on": true}, {"id": "b", "on": true}]}));
    assert_eq!(rows_of(&old), [Row::Feature(0), Row::Feature(1)]);
}

fn rows_of(list: &[Feature]) -> Vec<Row> {
    rows(list)
}

#[test]
fn packages_in_a_group_gather_under_the_groups_name() {
    // 核心 BS-1下上：出厂的十八个包在「基础系统」组里，一包一个功能，照组收在一起、写组名；不在组里的照包。
    let got = serde_json::json!({"features": [
        {"id": "read", "name": "读取", "on": true, "package": "read"},
        {"id": "weather", "name": "天气", "on": true, "package": "weather"},
        {"id": "shell", "name": "跑命令", "on": true, "package": "shell"}
    ]});
    let mut list = read(&got);
    let group = |id: &str| {
        (
            id.to_string(),
            ("basesystem".to_string(), "基础系统".to_string()),
        )
    };
    let packages = Packages {
        names: std::collections::HashMap::from([("weather".to_string(), "天气包".to_string())]),
        groups: std::collections::HashMap::from([group("read"), group("shell")]),
    };
    name_packages(&mut list, &packages);
    assert_eq!(
        rows(&list),
        [
            Row::Group(0),
            Row::Feature(0),
            Row::Feature(2),
            Row::Feature(1)
        ],
        "同组的收在一起；只有一个功能的包照旧不写组名"
    );
    assert_eq!(group_name(&list[0], "我的"), "基础系统");
    assert_eq!(group_name(&list[1], "我的"), "天气包");
}
