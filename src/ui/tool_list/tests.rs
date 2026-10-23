//! `/tool-list` 框排出来的样子（蓝图「工具 `/tool-list`」）。

use serde_json::json;

use super::{read, rows};
use crate::config::Config;

#[test]
fn tools_are_grouped_by_source_in_kind_order_and_packed_by_name() {
    // 核心 S-T 的样本，打乱种类的先后：内置在前、MCP、脚本在后；同一组照核心给的先后。
    let got = json!({"tools": [
        {"name": "edit", "source": {"kind": "builtin", "id": "basesystem", "name": "基础系统"}},
        {"name": "load_tools", "source": {"kind": "builtin", "id": "basesystem", "name": "基础系统"}},
        {"name": "mcp__notes__search", "source": {"kind": "mcp", "id": "notes", "name": "notes"}, "loaded": false},
        {"name": "read", "source": {"kind": "builtin", "id": "basesystem", "name": "基础系统"}},
        {"name": "weather", "source": {"kind": "script", "id": "weather", "name": "天气"}, "summary": "查天气"},
        {"name": "", "source": {"kind": "builtin", "id": "basesystem"}},
        {"name": "meme_send", "source": {"kind": "package", "id": "memes"}}
    ]});
    let tools = read(&got);
    assert_eq!(tools.len(), 6, "没名字的不要");
    let config = Config::builtin().unwrap();
    let texts = &config.text.tool_list;
    let text: Vec<String> = rows(&tools, texts, 80)
        .iter()
        .map(|l| l.to_string().trim_end().to_string())
        .collect();
    assert_eq!(
        text,
        [
            "基础系统  3 件",
            "  edit  load_tools  read",
            "",
            "memes  1 件",
            "  meme_send",
            "",
            "notes  1 件",
            "  mcp__notes__search",
            "",
            "天气  1 件",
            "  weather"
        ],
        "没有来源名字的写编号"
    );
    // 窄了折行：一行放不下的折下去，名字不截断。
    let narrow: Vec<String> = rows(&tools, texts, 16)
        .iter()
        .map(|l| l.to_string().trim_end().to_string())
        .collect();
    assert_eq!(&narrow[1..3], ["  edit", "  load_tools"]);
}
