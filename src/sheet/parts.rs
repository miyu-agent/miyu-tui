//! 主题那张表的各组（蓝图 `tui.md`「主题」第 1 条）：键名同 `packages/theme-default/theme.json`。

use ratatui::style::Modifier;
use ratatui::symbols::{border, line};
use serde::Deserialize;

/// 整张表（`layout`、`timeline` 两节读配置时盖，不在这里）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Sheet {
    /// 框线。
    pub frame: Frames,
    /// 记号。
    pub marks: Marks,
    /// 修饰。
    pub text: Text,
    /// 选中的样子。
    pub pick: Pick,
}

/// 选中的样子。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pick {
    /// 整行铺底色（出厂）。
    Bar,
    /// 不铺底，只照记号和强调色认。
    Mark,
}

/// 框线的样子。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Line {
    /// 圆角。
    Rounded,
    /// 直角。
    Plain,
    /// 双线。
    Double,
    /// 粗线。
    Thick,
    /// 只用 ASCII：`+` `-` `|`。
    Ascii,
    /// 不画线，只留出位置。
    None,
}

/// ASCII 的框线。
const ASCII: border::Set<'static> = border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

/// ASCII 的线（表格的交叉也是 `+`）。
const ASCII_LINES: line::Set<'static> = line::Set {
    vertical: "|",
    horizontal: "-",
    top_right: "+",
    top_left: "+",
    bottom_right: "+",
    bottom_left: "+",
    vertical_left: "+",
    vertical_right: "+",
    horizontal_down: "+",
    horizontal_up: "+",
    cross: "+",
};

/// 不画线：都是空格。
const NO_LINES: line::Set<'static> = line::Set {
    vertical: " ",
    horizontal: " ",
    top_right: " ",
    top_left: " ",
    bottom_right: " ",
    bottom_left: " ",
    vertical_left: " ",
    vertical_right: " ",
    horizontal_down: " ",
    horizontal_up: " ",
    cross: " ",
};

impl Line {
    /// 画表格、拼线用的那一套字（带交叉）。
    pub fn lines(self) -> line::Set<'static> {
        match self {
            Self::Rounded => line::ROUNDED,
            Self::Plain => line::NORMAL,
            Self::Double => line::DOUBLE,
            Self::Thick => line::THICK,
            Self::Ascii => ASCII_LINES,
            Self::None => NO_LINES,
        }
    }

    /// 画框用的那一套字。
    pub fn set(self) -> border::Set<'static> {
        match self {
            Self::Rounded => border::ROUNDED,
            Self::Plain => border::PLAIN,
            Self::Double => border::DOUBLE,
            Self::Thick => border::THICK,
            Self::Ascii => ASCII,
            Self::None => border::EMPTY,
        }
    }
}

/// 各处的框线。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frames {
    /// 输入框。
    pub input: Line,
    /// 输入框上面开的框：命令列表、`@` 文件列表、输入历史、会话列表、`/model`、`/help` 这些。
    pub boxes: Line,
    /// 弹出来的提示那个小框（「已复制」「再按一次 Esc 打断」这些）。
    pub toast: Line,
    /// 回答里代码块上下的线（只画左上角和横线）。
    pub code: Line,
    /// 回答里的表格。
    pub table: Line,
}

/// 记号（行首两格宽的要照两格写）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marks {
    /// 列表里选中的那一行前面（两格宽）。
    pub pointer: String,
    /// 正在用的、开着的、走过的。
    pub on: String,
    /// 没在用的、没到的。
    pub off: String,
    /// 正在这一步、这一页。
    pub now: String,
    /// 成了、选着。
    pub ok: String,
    /// 失败。
    pub fail: String,
    /// 放不下截掉的地方。
    pub ellipsis: String,
    /// 差异里省掉的几行。
    pub gap: String,
    /// 「下一步 →」这类动作行后面。
    pub next: String,
    /// 一行里几样东西之间（程序拼的；界面上的字里写好的 ` · ` 照那一份）。
    pub sep: String,
    /// 配置页顶上那条路径各层之间。
    pub dash: String,
    /// 横线：回答里的分隔线、分数线，配置页、界面块整页底下那条。
    pub rule: String,
    /// 回答里引用、展开的 details 每层行首。
    pub quote: String,
    /// 浮窗里选中的那一格左边的竖条。
    pub bar: String,
    /// 侧边栏和正文之间的竖线。
    pub side: String,
    /// 密钥、密码一个字一个。
    pub secret: String,
    /// 一行写下几行字时换行的地方。
    pub enter: String,
    /// 配置页、引导里画一块颜色。
    pub swatch: String,
    /// 用量热力图一格，图例也是（两格宽）。
    pub heat: String,
    /// 撤销那一行删掉了几行前面的减号。
    pub minus: String,
    /// 回答里的任务列表。
    pub task: Task,
    /// 用量柱状图一格里八分之一到八分之八高。
    pub chart: [String; 8],
    /// 用量那一页的条形。
    pub block: String,
}

/// 任务列表的两种框（两格宽）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    /// 没做。
    pub todo: String,
    /// 做完。
    pub done: String,
}

/// 一种修饰。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mod {
    /// 加粗。
    Bold,
    /// 斜体。
    Italic,
    /// 下划线。
    Underline,
    /// 变暗。
    Dim,
    /// 反色。
    Reversed,
}

impl Mod {
    /// 几种修饰合成一个。
    pub fn all(mods: &[Self]) -> Modifier {
        mods.iter().fold(Modifier::empty(), |all, m| {
            all | match m {
                Self::Bold => Modifier::BOLD,
                Self::Italic => Modifier::ITALIC,
                Self::Underline => Modifier::UNDERLINED,
                Self::Dim => Modifier::DIM,
                Self::Reversed => Modifier::REVERSED,
            }
        })
    }
}

/// 各处的修饰。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Text {
    /// 回答里的标题（配置页弹窗的分组标题也是）。
    pub heading: Vec<Mod>,
    /// 回答里卡片、图的标题。
    pub title: Vec<Mod>,
    /// 回答里的粗体。
    pub strong: Vec<Mod>,
    /// 回答里的斜体。
    pub emphasis: Vec<Mod>,
    /// 回答里的链接标题。
    pub link: Vec<Mod>,
    /// 回答里的引用。
    pub quote: Vec<Mod>,
    /// 回答里的表头。
    pub table_head: Vec<Mod>,
    /// 命令列表这类选中的那一条。
    pub picked: Vec<Mod>,
    /// 权限级别（提示符、左下角）。
    pub level: Vec<Mod>,
    /// 底栏的模型名。
    pub model: Vec<Mod>,
    /// 选区。
    pub selected: Vec<Mod>,
}
