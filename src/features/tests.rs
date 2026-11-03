//! 功能树：读 `sections`、平铺成一行行、切一行写哪几个键（「配置页」第 38 条「功能树」，核心设计 52）。

use serde_json::json;

use super::{
    Grid, Mark, Row, Tree, apply, mark, offs, read, section_rows, selectable, toggle, toggle_all,
};

/// 旧核心排成的那一组（没有组名）的一行行。
fn rows(tree: &Tree) -> Vec<Row> {
    section_rows(tree, 0)
}

/// 旧核心的写法（没有 `sections`）：文件读写（几件工具）、运行命令（一件）、人设防失忆提醒（没有工具）、没装的联网。
fn sample() -> Tree {
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

/// 核心 FG-1 的 `sections`：基础系统两个功能（文件读写带三件工具）、技能一个都没有、人格一个功能；`features` 里写了联网没装。
fn grouped() -> Tree {
    read(&json!({
    "features": [{"id": "web", "installed": false}],
    "sections": [
        {"id": "basesystem", "kind": "group", "name": "基础系统", "on": 2, "total": 2, "items": [
            {"kind": "feature", "id": "files", "name": "文件读写", "on": true, "by": "group", "tools": [
                {"id": "glob", "name": "找文件", "on": true},
                {"id": "grep", "name": "搜内容", "on": false},
                {"id": "read", "name": "读取", "on": true}]},
            {"kind": "feature", "id": "commands", "name": "运行命令", "on": false, "by": "preset"}]},
        {"id": "skills", "kind": "skills", "name": "技能", "on": 0, "total": 0, "items": []},
        {"id": "research", "kind": "group", "name": "查资料", "on": 1, "total": 1, "items": [
            {"kind": "feature", "id": "web", "name": "联网", "on": true, "by": "default"}]},
        {"id": "persona", "kind": "group", "name": "人格", "on": 1, "total": 1, "items": [
            {"kind": "feature", "id": "roleplay", "name": "人设遵循提醒", "on": true, "by": "group"}]}
    ]}))
}

#[test]
fn sections_come_in_the_cores_order_with_their_heading_and_an_empty_line() {
    let tree = grouped();
    assert_eq!(tree.sections.len(), 4);
    assert_eq!(
        (
            tree.sections[1].name.as_str(),
            tree.sections[1].kind.as_str()
        ),
        ("技能", "skills")
    );
    assert_eq!(
        tree.features[0].tools[1].name, "grep",
        "工具的编号是写键用的名字"
    );
    assert_eq!(tree.features[0].tools[1].label, "搜内容");
    assert!(
        !tree.features[2].installed,
        "装没装照 `features` 里同一个编号的"
    );
    assert_eq!(
        section_rows(&tree, 0),
        [
            Row::Section(0),
            Row::Feature(0),
            Row::Tool(0, 0),
            Row::Tool(0, 1),
            Row::Tool(0, 2),
            Row::Feature(1)
        ]
    );
    assert_eq!(section_rows(&tree, 1), [Row::Section(1), Row::Empty(1)]);
    assert!(selectable(&tree, Row::Section(0)), "组名那一行停得上");
    assert!(!selectable(&tree, Row::Section(1)), "什么都没有的组停不上");
    assert!(
        !selectable(&tree, Row::Section(2)),
        "组里只有没装的也停不上"
    );
    assert!(!selectable(&tree, Row::Empty(1)) && !selectable(&tree, Row::Blank));
    assert_eq!(super::tally(&tree, 0), (1, 2), "开着几个、一共几个");
}

#[test]
fn space_on_a_heading_flips_the_whole_section() {
    // 2026-10-30 项目主人：「可以整组，也可以 ctrl+A 切换全选」。组里有关着的就全开，都开着的就全关。
    let tree = grouped();
    let key = |k: &str, on: bool| (k.to_string(), on);
    assert_eq!(
        toggle(&tree, Row::Section(0)),
        [key("tools.grep", true), key("features.commands", true)]
    );
    assert_eq!(
        toggle(&tree, Row::Section(3)),
        [key("features.roleplay", false)]
    );
    assert!(toggle(&tree, Row::Empty(1)).is_empty() && toggle(&tree, Row::Blank).is_empty());
}

#[test]
fn sections_split_into_two_columns_where_the_lines_are_closest() {
    let tree = grouped();
    let grid = Grid::new(&tree, true);
    // 基础系统 6 行 | 技能 2 + 空 1 + 查资料 2 + 空 1 + 人格 2 = 8 行：切在第一组后面最近。
    assert_eq!(grid.cols.len(), 2);
    assert_eq!(grid.cols[0], section_rows(&tree, 0));
    assert_eq!(
        grid.cols[1],
        [
            Row::Section(1),
            Row::Empty(1),
            Row::Blank,
            Row::Section(2),
            Row::Feature(2),
            Row::Blank,
            Row::Section(3),
            Row::Feature(3)
        ]
    );
    // 窗不够宽：排一列，组和组之间空一行。
    let one = Grid::new(&tree, false);
    assert_eq!(one.cols.len(), 1);
    assert_eq!(one.cols[0].len(), 6 + 1 + 2 + 1 + 2 + 1 + 2);
    // 停得上的照先后：左列从上往下，再右列；没装的、空的组停不上。
    assert_eq!(
        grid.stops(&tree),
        [
            Row::Section(0),
            Row::Feature(0),
            Row::Tool(0, 0),
            Row::Tool(0, 1),
            Row::Tool(0, 2),
            Row::Feature(1),
            Row::Section(3),
            Row::Feature(3)
        ]
    );
}

#[test]
fn up_and_down_stay_in_a_column_and_left_right_go_to_the_nearest_line() {
    let tree = grouped();
    let grid = Grid::new(&tree, true);
    assert_eq!(grid.down(&tree, Row::Feature(1)), None, "左列到底了");
    assert_eq!(grid.up(&tree, Row::Section(0)), None, "到顶了交给外面");
    assert_eq!(
        grid.up(&tree, Row::Section(3)),
        None,
        "右列上面只有停不上的"
    );
    assert_eq!(grid.down(&tree, Row::Tool(0, 2)), Some(Row::Feature(1)));
    assert_eq!(
        grid.side(&tree, Row::Tool(0, 2), true),
        Some(Row::Section(3)),
        "右列离第 5 行最近的停得上的"
    );
    assert_eq!(
        grid.side(&tree, Row::Feature(3), false),
        Some(Row::Feature(1))
    );
    assert_eq!(grid.side(&tree, Row::Feature(0), false), None, "左边没有了");
}

/// 预设写了、这台机器没装的：核心不排进组，照旧排在最后一段，不写组名，停不上（「配置页」第 38 条）。
#[test]
fn features_written_but_not_installed_follow_in_a_last_unnamed_part() {
    let tree = read(&json!({
        "features": [{"id": "goal", "name": "长期目标", "on": true, "installed": false}],
        "sections": [{"id": "tasks", "kind": "group", "name": "任务和定时", "items": [
            {"id": "todo", "name": "待办清单", "on": true}]}]}));
    assert_eq!(tree.sections.len(), 2);
    assert_eq!(tree.features[1].name, "长期目标");
    assert!(!tree.features[1].installed);
    assert_eq!(section_rows(&tree, 1), [Row::Feature(1)], "不写组名");
    assert!(!selectable(&tree, Row::Feature(1)));
}
