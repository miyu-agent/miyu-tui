//! 认得的三种 shell：叫什么、装在哪、调它的那一行怎么引号。

use std::path::{Path, PathBuf};

/// 认得的 shell。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    /// fish：整行判断是命令还是话（设计 22 第七节）。
    Fish,
    /// bash：只做找不到命令时交给她。
    Bash,
    /// zsh：同 bash。
    Zsh,
}

/// 三种都在这：`status` 照这个顺序一行一种。
pub const ALL: [Shell; 3] = [Shell::Fish, Shell::Bash, Shell::Zsh];

impl Shell {
    /// 照名字认：`fish`、`bash`、`zsh`，也认带路径的（`/usr/bin/fish`）和登录 shell 的 `-fish`。
    pub fn named(name: &str) -> Option<Self> {
        let base = Path::new(name).file_name()?.to_str()?;
        match base.trim_start_matches('-') {
            "fish" => Some(Self::Fish),
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            _ => None,
        }
    }

    /// 名字。
    pub fn name(self) -> &'static str {
        match self {
            Self::Fish => "fish",
            Self::Bash => "bash",
            Self::Zsh => "zsh",
        }
    }

    /// 装在哪：fish 是 conf.d 下自己的一个文件；bash、zsh 是启动文件里的一段。`env` 读环境变量（测试里换掉）。
    pub fn target(self, env: &impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
        let home = env("HOME").filter(|h| !h.is_empty()).map(PathBuf::from);
        match self {
            Self::Fish => {
                let config = env("XDG_CONFIG_HOME")
                    .filter(|d| !d.is_empty())
                    .map(PathBuf::from)
                    .or_else(|| home.map(|h| h.join(".config")))?;
                Some(config.join("fish/conf.d/miyu-shellhook.fish"))
            }
            Self::Bash => Some(home?.join(".bashrc")),
            Self::Zsh => {
                let dir = env("ZDOTDIR")
                    .filter(|d| !d.is_empty())
                    .map(PathBuf::from)
                    .or(home)?;
                Some(dir.join(".zshrc"))
            }
        }
    }

    /// 旧版装的那一份（并行期两份都在，一句话会交两遍）：只有 fish 是单独的文件，认得出来。
    pub fn old_hook(self, env: &impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
        match self {
            Self::Fish => Some(self.target(env)?.with_file_name("miyu.fish")),
            Self::Bash | Self::Zsh => None,
        }
    }

    /// 把一个路径写成这种 shell 里的一个词：单引号包起来。
    pub fn quote(self, path: &Path) -> String {
        let text = path.to_string_lossy();
        match self {
            // fish 的单引号里只有 \' 和 \\ 是转义。
            Self::Fish => format!("'{}'", text.replace('\\', "\\\\").replace('\'', "\\'")),
            // POSIX 的单引号里什么都不转义：引号先关上，转义的引号，再打开。
            Self::Bash | Self::Zsh => format!("'{}'", text.replace('\'', "'\\''")),
        }
    }
}

/// 照上一层进程认是哪种 shell（`miyu shellhook install` 是在 shell 里敲的；核心转交时换成这个程序，上一层还是那个 shell），
/// 认不出的照 `SHELL`。
pub fn detect() -> Option<Shell> {
    parent_name().as_deref().and_then(Shell::named).or_else(|| {
        std::env::var("SHELL")
            .ok()
            .as_deref()
            .and_then(Shell::named)
    })
}

/// 上一层进程叫什么：Linux 上读 `/proc`，别的平台不认。
fn parent_name() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let ppid = std::os::unix::process::parent_id();
        let comm = std::fs::read_to_string(format!("/proc/{ppid}/comm")).ok()?;
        Some(comm.trim().to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}
