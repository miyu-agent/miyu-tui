//! 图标（`resources/icons/*.json`，蓝图 `tui.md`「图标」）：一套一个文件，时间线的图标、暂存着东西时的提示符
//! 都从当前这一套取。包的工具没登记自己的图标时，照包清单 `[package] icon` 写的 Lucide 名在这一套的 `lucide` 里换成
//! 字形（核心定的规矩，2026-10-16 项目主人要表情包有自己的图标）；这一套没有这个名字的、包没写的照旧用 `tool`。出厂 `nerd`（Nerd Font 的字形）和 `plain`（普通字符）；用哪套照 `layout.json` 的 `icons`，
//! `/icons` 轮换。以后第一次启动时问看不看得到 Nerd Font 的图标，选好的记进设置。

use std::collections::HashMap;

use serde::Deserialize;

/// 一套图标。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Icons {
    /// 这一套叫什么（文件名，不写在文件里）。
    #[serde(skip)]
    pub name: String,
    /// 每件工具的图标；没登记的照它的包的图标，再没有的用 `tool`。
    pub tools: HashMap<String, String>,
    /// Lucide 图标名到这一套的字形：包清单 `[package] icon` 写的是 Lucide 名。普通字符那一套不写（一律 `tool`）。
    #[serde(default)]
    pub lucide: HashMap<String, String>,
    /// 没登记的工具的图标。
    pub tool: String,
    /// 思考的图标。
    pub think: String,
    /// 出错时顶替图标的叉。
    pub error: String,
    /// 会话列表里置顶的记号，连同它后面的空格（蓝图「会话列表」第 1 条）。
    pub pin: String,
    /// 暂存着东西时的提示符，连同它后面的空格。
    pub stash: String,
    /// 视频卡片封面下的播放标记（蓝图「链接卡片」第 3 条）。
    pub video: String,
    /// 要人留意的一句前面的记号，连同它后面的空格（人格、预设框里「切换前会先压缩」，2026-10-23 项目主人定黄字加 ⚠）。
    #[serde(default = "warn_mark")]
    pub warn: String,
    /// fork 出来的子代理名字前面的分叉记号，连同它后面的空格（2026-10-26 项目主人）；没写的套件不画。
    #[serde(default)]
    pub fork: String,
    /// 展示的卡片前面的种类图标：`html`、`markdown`、`code`、`pdf`、`text`、`file`（蓝图「展示」第 4 条，2026-10-17 项目主人
    /// 要的）；没写的这一套不画。
    #[serde(default)]
    pub files: HashMap<String, String>,
}

impl Icons {
    /// 这件工具的图标。
    pub fn tool(&self, name: &str) -> &str {
        self.tool_in(name, None)
    }

    /// 这件工具的图标，`package` 是它的包在清单里写的 Lucide 图标名：先照工具自己登记的，再照包的，都没有的用 `tool`。
    pub fn tool_in(&self, name: &str, package: Option<&str>) -> &str {
        self.tools
            .get(name)
            .or_else(|| package.and_then(|icon| self.lucide.get(icon)))
            .unwrap_or(&self.tool)
    }
}

/// 出厂的几套：名字和 JSON，照登记的先后。
const BUILTIN: [(&str, &str); 2] = [
    ("nerd", include_str!("../../resources/icons/nerd.json")),
    ("plain", include_str!("../../resources/icons/plain.json")),
];

/// 读出厂的全部几套，照登记的先后。
///
/// # Errors
///
/// 哪一套写坏了，说是哪一套。
pub fn builtin() -> Result<Vec<Icons>, String> {
    BUILTIN
        .iter()
        .map(|(name, json)| {
            serde_json::from_str::<Icons>(json)
                .map(|icons| Icons {
                    name: (*name).to_string(),
                    ..icons
                })
                .map_err(|e| format!("resources/icons/{name}.json 读不懂：{e}"))
        })
        .collect()
}

/// 照名字挑一套；没有这一套的用出厂的第一套。
pub fn pick(sets: &[Icons], name: &str) -> Option<Icons> {
    sets.iter()
        .find(|icons| icons.name == name)
        .or(sets.first())
        .cloned()
}

/// 没写 `warn` 的图标套件（以前的、包里带的）用它。
fn warn_mark() -> String {
    "! ".to_string()
}

#[cfg(test)]
mod tests {
    use unicode_width::UnicodeWidthStr;

    use super::{Icons, builtin, pick};

    /// Nerd Font 的字形都在私用区。
    fn private_use(c: char) -> bool {
        matches!(u32::from(c), 0xE000..=0xF8FF | 0xF_0000..=0x10_FFFD)
    }

    fn every_icon(icons: &Icons) -> Vec<&str> {
        let mut all: Vec<&str> = icons
            .tools
            .values()
            .chain(icons.lucide.values())
            .map(String::as_str)
            .collect();
        all.extend([
            icons.tool.as_str(),
            icons.think.as_str(),
            icons.error.as_str(),
            icons.video.as_str(),
        ]);
        all
    }

    #[test]
    fn two_sets_ship_and_the_plain_one_needs_no_nerd_font() {
        let sets = builtin().unwrap();
        let names: Vec<&str> = sets.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["nerd", "plain"]);
        let plain = &sets[1];
        let glyphs: String = every_icon(plain).concat() + &plain.stash;
        assert!(
            !glyphs.chars().any(private_use),
            "普通字符那一套没有 Nerd Font 的字形：{glyphs}"
        );
        // 普通字符那一套不凑（设计 13 第三节第 9 条）：执行命令 `$`，别的工具一律 `⚙`。
        assert_eq!(plain.tool("shell"), "$");
        assert_eq!(plain.tool("read"), "⚙");
        assert_eq!(plain.tool("没登记的"), "⚙");
    }

    #[test]
    fn every_icon_takes_one_cell_so_the_column_lines_up() {
        for set in builtin().unwrap() {
            for icon in every_icon(&set) {
                assert_eq!(icon.width(), 1, "{} 这一套的 {icon:?}", set.name);
            }
            assert_eq!(set.stash.width(), 2, "{} 的暂存提示符连空格两格", set.name);
        }
    }

    #[test]
    fn every_icon_stays_in_the_basic_plane() {
        // 2026-10-16 项目主人经 ssh 在手机上用：思考的图标 md-atom（U+F0768，第 15 平面的私用区）那边的终端认不准它占
        // 几格，转圈那一行往上推时在时间线左边留下半个字形。换回它、在画的那一层修（`glyph_guard`）也没用（10-19 复现：
        // 那边的终端把字画得比一格宽，多出的半个落在旁边的空格里，那格原样重写它不重画）。出厂的图标一律只用 BMP 的字。
        for set in builtin().unwrap() {
            let mut all = every_icon(&set);
            all.extend([set.pin.as_str(), set.stash.as_str()]);
            for icon in all {
                assert!(
                    icon.chars().all(|c| u32::from(c) <= 0xFFFF),
                    "{} 这一套的 {icon:?} 不在 BMP 里",
                    set.name
                );
            }
        }
        // 2026-10-19 项目主人：fae-atom 线条太细，换实心的 fa-atom。
        assert_eq!(builtin().unwrap()[0].think, "\u{ee99}", "思考的是 fa-atom");
    }

    #[test]
    fn every_tool_the_core_ships_has_a_nerd_icon() {
        // 2026-10-07 项目主人：工具的图标别忘了加。核心基础系统的每件工具（钉住的那份核心检出里的 `resources/software/basesystem/tools/`）在
        // Nerd Font 那一套里都要有自己的一个，核心加了新工具这里当场红。
        let dir = std::path::Path::new(miyu_client::WORKSPACE_ROOT)
            .join("resources/software/basesystem/tools");
        let nerd = &builtin().unwrap()[0];
        // 内核当场答的（`load_tools`，核心 L-1）不在那个目录里，在基础系统给人看的字里登记着显示名：一起查。改了名的旧名
        // （`agent`、`message_agent`）蓝图定了不认，不查。
        let human = std::path::Path::new(miyu_client::WORKSPACE_ROOT)
            .join("resources/software/basesystem/human/zh.json");
        let human: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(human).unwrap()).unwrap();
        let named = human["tools"]
            .as_object()
            .into_iter()
            .flat_map(|t| t.keys().cloned())
            .filter(|n| !["agent", "message_agent"].contains(&n.as_str()));
        let mut missing: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok()?.path().file_stem()?.to_str().map(str::to_string))
            .chain(named)
            .filter(|name| !nerd.tools.contains_key(name))
            .collect();
        missing.sort();
        missing.dedup();
        assert!(missing.is_empty(), "这几件工具没有图标：{missing:?}");
    }

    #[test]
    fn a_package_tool_takes_its_package_icon_in_the_nerd_set_only() {
        // 2026-10-16 项目主人验 T-260：「挑选表情包」那一行还是扳手；包清单写了 Lucide 名的，照它换。
        let sets = builtin().unwrap();
        let (nerd, plain) = (&sets[0], &sets[1]);
        assert_eq!(
            nerd.tool_in("meme_send", Some("face-slightly-smiling")),
            "\u{f118}"
        );
        assert_eq!(
            nerd.tool_in("meme_send", Some("没有这个名字")),
            nerd.tool,
            "表里没有的用扳手"
        );
        assert_eq!(nerd.tool_in("meme_send", None), nerd.tool, "包没写图标");
        assert_eq!(
            nerd.tool_in("read", Some("face-slightly-smiling")),
            nerd.tool("read"),
            "工具自己登记了的照自己的"
        );
        assert_eq!(
            plain.tool_in("meme_send", Some("face-slightly-smiling")),
            "⚙",
            "没有 Nerd Font 的照旧"
        );
        for name in [
            "face-slightly-smiling",
            "image",
            "music",
            "globe",
            "book",
            "bot",
            "message-circle",
            "calendar",
            "database",
            "terminal",
            "search",
            "file",
            // 2026-10-22 移植旧 Miyu 脚本包时清单里写了、原来表里没有的（hexagon 没有对得上的字形，照旧用扳手）。
            "dice-5",
            "tv",
            "radio",
            "scroll",
            "banknote",
            "shield-check",
            "download",
            "book-marked",
            "book-heart",
            "shopping-bag",
        ] {
            assert!(nerd.lucide.contains_key(name), "常用的 {name} 要有");
        }
    }

    #[test]
    fn a_lucide_name_missing_from_the_table_falls_back_to_the_tool_icon() {
        let nerd = pick(&builtin().unwrap(), "nerd").unwrap();
        assert_eq!(nerd.tool_in("meme_roll", Some("hexagon")), nerd.tool);
        assert_eq!(nerd.tool_in("meme_roll", Some("dice-5")), "\u{eded}");
    }

    #[test]
    fn an_unknown_name_falls_back_to_the_first_set() {
        let sets = builtin().unwrap();
        assert_eq!(pick(&sets, "plain").unwrap().name, "plain");
        assert_eq!(pick(&sets, "没有这套").unwrap().name, "nerd");
    }
}
