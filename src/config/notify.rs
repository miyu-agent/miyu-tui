//! 系统通知的开关和清单（`resources/notify.json`）、字（`text/zh.json` 的 `notify`），蓝图 `tui.md`「系统通知」。

/// 系统通知怎么弹、怎么响。
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(serde::Deserialize, PartialEq))]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct NotifyLook {
    /// 关掉整个不弹（第 5 条）。
    pub enabled: bool,
    /// 关掉就不响。
    pub sound: bool,
    /// kitty 的通知上写的程序名。
    pub app: String,
    /// `TERM_PROGRAM` 是这几个的用 OSC 9（第 4 条）。
    pub osc9_programs: Vec<String>,
    /// 放提示音的程序，照先后试，哪个拉得起来用哪个；每一个是程序和它的参数，音的文件接在最后。
    pub players: Players,
}

crate::from_json::record! { NotifyLook, deny { enabled, sound, app, osc9_programs, players } }

/// 各个系统上放提示音的程序。
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(serde::Deserialize, PartialEq))]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct Players {
    /// Linux 上的。
    pub linux: Vec<Vec<String>>,
    /// macOS 上的。
    pub macos: Vec<Vec<String>>,
    /// Windows 上的（2026-11-15 三平台摸底：原来照 Linux 那份，一个都没有，不响）。参数里写 `{file}` 的那一格换成音的文件。
    #[cfg_attr(test, serde(default))]
    pub windows: Vec<Vec<String>>,
}

crate::from_json::record! { Players, deny { linux, macos, windows(default) } }

impl Players {
    /// 这台机器上照哪一份。
    pub fn here(&self) -> &[Vec<String>] {
        if cfg!(target_os = "macos") {
            &self.macos
        } else if cfg!(windows) {
            &self.windows
        } else {
            &self.linux
        }
    }
}

/// 通知上的字（第 3 条）。
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(serde::Deserialize, PartialEq))]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct NotifyTexts {
    /// 标题。
    pub title: String,
    /// 一轮回答完了。
    pub replied: String,
    /// 出错了。
    pub failed: String,
    /// 在等你确认。
    pub approving: String,
    /// 在等你回答。
    pub asking: String,
    /// 每次新开会话的定时跑完了、没跑完（出错、被打断，核心 J-3 中）。
    pub schedule_ran: String,
    pub schedule_failed: String,
    /// 唤醒表坏了换了一份空的：原来的定时都没了。
    pub schedules_reset: String,
    /// 系统通知上点开的那个按钮（`notify-send` 的默认动作）。
    pub open: String,
    /// 标题写人格名以后，正文前面接会话标题的写法（`{title}`、`{body}`，「系统通知」第 3 条，2026-11-10）。
    pub titled: String,
}

crate::from_json::record! {
    NotifyTexts, deny {
        title, replied, failed, approving, asking, schedule_ran, schedule_failed, schedules_reset,
        open, titled,
    }
}
