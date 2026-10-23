//! 认得的 shell：叫什么、装在哪、调它的那一行怎么引号。

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
    /// PowerShell 7（`pwsh`，Windows、Linux、macOS 都有）：回车交给 PSReadLine 的按键处理判断（2026-10-23 项目主人）。
    Pwsh,
    /// Windows 自带的 Windows PowerShell 5.1（`powershell`）：脚本同 `pwsh`，启动文件是另一个。
    WinPs,
}

/// 都在这：`status` 照这个顺序一行一种。
pub const ALL: [Shell; 5] = [
    Shell::Fish,
    Shell::Bash,
    Shell::Zsh,
    Shell::Pwsh,
    Shell::WinPs,
];

impl Shell {
    /// 照名字认：`fish`、`bash`、`zsh`、`pwsh`、`powershell`，也认带路径的（`/usr/bin/fish`）、登录 shell 的 `-fish`
    /// 和 Windows 上带 `.exe` 的。
    pub fn named(name: &str) -> Option<Self> {
        let base = Path::new(name).file_name()?.to_str()?;
        let base = base.trim_start_matches('-');
        let base = base
            .strip_suffix(".exe")
            .or_else(|| base.strip_suffix(".EXE"))
            .unwrap_or(base);
        match base {
            "fish" => Some(Self::Fish),
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "pwsh" => Some(Self::Pwsh),
            "powershell" => Some(Self::WinPs),
            _ => None,
        }
    }

    /// 是 PowerShell（两种脚本、装法一样）。
    pub fn powershell(self) -> bool {
        matches!(self, Self::Pwsh | Self::WinPs)
    }

    /// 名字。
    pub fn name(self) -> &'static str {
        match self {
            Self::Fish => "fish",
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Pwsh => "pwsh",
            Self::WinPs => "powershell",
        }
    }

    /// 装在哪：fish 是 conf.d 下自己的一个文件；bash、zsh、PowerShell 是启动文件里的一段。`env` 读环境变量（测试里换掉）。
    /// PowerShell 的启动文件问它自己（`$PROFILE`：「文档」可能被挪到 OneDrive，自己拼会拼错），没装的是 `None`。
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
            Self::Pwsh | Self::WinPs => profile(self.name()),
        }
    }

    /// 旧版装的那一份（并行期两份都在，一句话会交两遍）：只有 fish 是单独的文件，认得出来。
    pub fn old_hook(self, env: &impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
        match self {
            Self::Fish => Some(self.target(env)?.with_file_name("miyu.fish")),
            Self::Bash | Self::Zsh | Self::Pwsh | Self::WinPs => None,
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
            // PowerShell 的单引号里只有两个单引号是一个单引号。
            Self::Pwsh | Self::WinPs => format!("'{}'", text.replace('\'', "''")),
        }
    }
}

/// 问 PowerShell 它的启动文件在哪（这个用户、这个宿主的那一个）；没装、问不出的是 `None`。
fn profile(program: &str) -> Option<PathBuf> {
    let out = std::process::Command::new(program)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); $PROFILE.CurrentUserCurrentHost",
        ])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let path = String::from_utf8(out.stdout).ok()?;
    let path = path.trim();
    (out.status.success() && !path.is_empty()).then(|| PathBuf::from(path))
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
