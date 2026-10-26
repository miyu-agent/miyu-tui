//! 引导的几步和先后（蓝图 `tui.md`「第一次打开的引导」）。

/// 第几步，照先后。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// 欢迎页。
    Welcome,
    /// 界面语言。
    Language,
    /// 注册机主：只在还没有机主时有（第 5a 条，2026-10-24 项目主人定，放在语言后面）。
    Register,
    /// 你喜欢的操作方式：方向键、Vim（2026-10-21 项目主人定，放在语言后面）。
    Keys,
    /// 图标。
    Icons,
    /// 接模型。
    Model,
    /// 建人格。
    Persona,
    /// 主题：明暗、配色（2026-10-13 项目主人定，放在人格后面：配色跟着头像）。
    Theme,
    /// AI 怎么认识你（2026-10-13 项目主人定）。
    Profile,
    /// 选预设。
    Preset,
    /// 开不开内置的语义模型，只在装了它的包时出现（2026-10-18 项目主人定）。
    Semantic,
    /// 好了。
    Done,
}

const ORDER: [Step; 12] = [
    Step::Welcome,
    Step::Language,
    Step::Register,
    Step::Keys,
    Step::Icons,
    Step::Model,
    Step::Persona,
    Step::Theme,
    Step::Profile,
    Step::Preset,
    Step::Semantic,
    Step::Done,
];

impl Step {
    fn at(self) -> usize {
        ORDER.iter().position(|s| *s == self).unwrap_or(0)
    }

    /// 下一步（好了以后没有）。
    pub fn next(self) -> Self {
        ORDER.get(self.at() + 1).copied().unwrap_or(self)
    }

    /// 上一步（欢迎页前面没有）。
    pub fn previous(self) -> Self {
        ORDER[self.at().saturating_sub(1)]
    }

    /// 进度那一行画哪几步（字表 `oobe.steps` 里第几个名字）、这一步是其中第几个；欢迎、好了不画进度（第 6 条）。
    /// `register`：还要注册机主，「注册」那一步画进去（第 5a 条）。
    pub fn bar(self, register: bool) -> Option<(Vec<usize>, usize)> {
        let shown: Vec<usize> = (0..BAR.len())
            .filter(|i| register || BAR[*i] != Step::Register)
            .collect();
        let now = shown.iter().position(|i| BAR[*i] == self)?;
        Some((shown, now))
    }
}

/// 进度那一行的几步，照先后；和字表 `oobe.steps` 的名字一一对着。
const BAR: [Step; 9] = [
    Step::Language,
    Step::Register,
    Step::Keys,
    Step::Icons,
    Step::Model,
    Step::Persona,
    Step::Theme,
    Step::Profile,
    Step::Preset,
];
