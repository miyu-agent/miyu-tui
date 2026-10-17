//! 引导的几步和先后（蓝图 `tui.md`「第一次打开的引导」）。

/// 第几步，照先后。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// 欢迎页。
    Welcome,
    /// 界面语言。
    Language,
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
    /// 沙盒：新会话怎么开局（2026-10-13 项目主人定）。
    Sandbox,
    /// 开不开内置的语义模型，只在装了它的包时出现（2026-10-18 项目主人定）。
    Semantic,
    /// 好了。
    Done,
}

const ORDER: [Step; 11] = [
    Step::Welcome,
    Step::Language,
    Step::Icons,
    Step::Model,
    Step::Persona,
    Step::Theme,
    Step::Profile,
    Step::Preset,
    Step::Sandbox,
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

    /// 在进度那一行里是第几个；欢迎、好了不画进度（第 6 条）。
    pub fn bar(self) -> Option<usize> {
        matches!(
            self,
            Step::Language
                | Step::Icons
                | Step::Model
                | Step::Persona
                | Step::Theme
                | Step::Profile
                | Step::Preset
                | Step::Sandbox
        )
        .then(|| self.at() - 1)
    }
}
