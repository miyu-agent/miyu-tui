//! 在伪终端里起装了钩子的真 shell（bash、zsh、fish）测脚本自己那一半：回车判话还是命令、找不到命令交不交给她、提前敲的
//! 字放不放得回命令行。钩子调的程序换成一个记账的替身（[`STUB`]），不起核心、不拉起终端界面：替身把每次调用记进 `calls`，
//! 照测试目录里的几个开关文件办（交回什么字、放回什么字、信箱里有没有回报）。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use super::{Tui, WAIT};

/// 替身：脚本调它是 `<程序> shellhook --hook <记号> <命令> [选项]`。`say` 读标准输入那句话；`/edit` 照真程序把「touch edited」写进
/// `--leftover`、旁边放 `.redo`；有 `answer` 的把它印到标准输出（找不到命令那条路的输出进了 `$(…)` 就会被执行）；有 `typed`
/// 的把它写进 `--leftover`（像她答话时你敲了这几个字）；有 `wake` 的往信箱放一件、给 shell 发 SIGUSR1（像她答话时来了后台
/// 回报）。`report` 照 `mail` 办：有的印一行、`typed` 写进 `--leftover`；`--pending` 只问有没有。
const STUB: &str = r#"#!/bin/sh
dir=@DIR@
# shellhook --hook <记号>
shift 3
cmd=$1
shift
args="$*"
left=
pid=
pending=
while [ $# -gt 0 ]; do
    case $1 in
        --leftover) left=$2; shift ;;
        --shell-pid) pid=$2; shift ;;
        --pending) pending=1 ;;
    esac
    shift
done
case $cmd in
    say)
        said=$(cat)
        printf '%s\n' "say [$args] $said" >> "$dir/calls"
        if [ "$said" = /edit ] && [ -n "$left" ]; then
            printf 'touch edited' > "$left"
            : > "$left.redo"
            exit 0
        fi
        [ -e "$dir/answer" ] && cat "$dir/answer"
        if [ -e "$dir/wake" ]; then
            rm -f "$dir/wake"
            : > "$dir/mail"
            kill -USR1 "$pid"
            sleep 0.5
        fi
        printf 'stub answered\n' > /dev/tty
        [ -e "$dir/typed" ] && [ -n "$left" ] && cat "$dir/typed" > "$left"
        exit 0
        ;;
    report)
        [ -e "$dir/mail" ] || exit 1
        [ -n "$pending" ] && exit 0
        rm -f "$dir/mail"
        printf '%s\n' "report [$args]" >> "$dir/calls"
        printf 'stub reported\n' > /dev/tty
        [ -e "$dir/typed" ] && [ -n "$left" ] && cat "$dir/typed" > "$left"
        exit 0
        ;;
    classify) exit 0 ;;
    *) exit 1 ;;
esac
"#;

/// 测的三种 shell（机器上没装的跳过，[`installed`]）。
pub const SHELLS: [&str; 3] = ["bash", "zsh", "fish"];

/// 测试的 shell 都用这个提示符。
pub const PROMPT: &str = "P$ ";

/// 一个装了钩子的 shell，开在伪终端里；用完删掉测试目录。
pub struct Hooked {
    /// 伪终端那一头。
    pub tui: Tui,
    /// 测试目录：替身的记账和开关文件在这里。
    pub dir: PathBuf,
    /// shell 起在这里（`touch` 这类命令建出来的文件在这里）。
    pub work: PathBuf,
}

/// 这台机器上装没装着能用的 `shell`：bash 要 4.4 以上（钩子用 `${PS1@P}`），没装的测试跳过这一种。
pub fn installed(shell: &str) -> bool {
    let probe = match shell {
        "bash" => "((BASH_VERSINFO[0] * 100 + BASH_VERSINFO[1] >= 404))",
        _ => "true",
    };
    std::process::Command::new(shell)
        .args(["-c", probe])
        .output()
        .is_ok_and(|out| out.status.success())
}

impl Hooked {
    /// 起 `shell`：启动文件里先装钩子、设提示符，再接上 `extra`（测试要的别的启动内容，比如找不到的命令）。等提示符出来。
    pub fn start(shell: &str, extra: &str) -> Hooked {
        Hooked::start_with(shell, extra, &[])
    }

    /// 同 [`Hooked::start`]，起 shell 以前先打开几个开关文件（[`Hooked::set`]）：启动文件里就用得着的。
    pub fn start_with(shell: &str, extra: &str, switches: &[(&str, &str)]) -> Hooked {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("miyu-tui-sh-{}-{n}", std::process::id()));
        let (dir, work, home) = (base.join("t"), base.join("work"), base.join("home"));
        for each in [&dir, &work, &home] {
            std::fs::create_dir_all(each).expect("建得了测试目录");
        }
        let base = std::fs::canonicalize(&base).expect("在");
        let (dir, work, home) = (base.join("t"), base.join("work"), base.join("home"));
        let stub = dir.join("stub");
        write_program(&stub, &STUB.replace("@DIR@", &quote(&dir)));
        for (name, text) in switches {
            std::fs::write(dir.join(name), text).expect("写得进去");
        }
        let hook = dir.join(format!("hook.{shell}"));
        std::fs::write(&hook, render(shell, &stub, &home)).expect("写得进去");
        let hook = quote(&hook);
        let (cache, home_text) = (home.join("cache"), home.display().to_string());
        let cache_text = cache.display().to_string();
        let mut env = vec![
            ("HOME", home_text.as_str()),
            ("XDG_CACHE_HOME", cache_text.as_str()),
        ];
        let rc = base.join("rc");
        std::fs::create_dir_all(&rc).expect("建得了");
        let rc_text = rc.display().to_string();
        let (args, rc_file): (Vec<&str>, PathBuf) = match shell {
            "bash" => (vec!["--rcfile", "", "-i"], rc.join("bashrc")),
            "zsh" => {
                env.push(("ZDOTDIR", rc_text.as_str()));
                (vec!["-i"], rc.join(".zshrc"))
            }
            _ => {
                env.push(("XDG_CONFIG_HOME", rc_text.as_str()));
                std::fs::create_dir_all(rc.join("fish")).expect("建得了");
                (vec!["-i"], rc.join("fish/config.fish"))
            }
        };
        let prompt = match shell {
            "bash" => "PS1='P$ '\nHISTFILE=/dev/null",
            "zsh" => "PROMPT='P$ '\nunset HISTFILE",
            _ => "set -g fish_greeting\nfunction fish_prompt; printf 'P$ '; end",
        };
        std::fs::write(&rc_file, format!("source {hook}\n{prompt}\n{extra}\n")).expect("写得进去");
        let rc_file = rc_file.display().to_string();
        let args: Vec<&str> = args
            .into_iter()
            .map(|a| if a.is_empty() { rc_file.as_str() } else { a })
            .collect();
        let tui = Tui::program(&home, &work, "C.UTF-8", &env, &args, shell);
        let mut hooked = Hooked { tui, dir, work };
        hooked.wait_line("提示符", |line| line == PROMPT.trim_end());
        hooked
    }

    /// 打开一个开关文件（[`STUB`] 照它办），内容是 `text`。
    pub fn set(&self, name: &str, text: &str) {
        std::fs::write(self.dir.join(name), text).expect("写得进去");
    }

    /// 替身记下的调用，一次一行。
    pub fn calls(&self) -> Vec<String> {
        std::fs::read_to_string(self.dir.join("calls"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// 等到替身记下的调用合 `check`，最多 [`WAIT`]；交回那时的调用。
    pub fn wait_calls(&mut self, what: &str, check: impl Fn(&[String]) -> bool) -> Vec<String> {
        let end = Instant::now() + WAIT;
        loop {
            let calls = self.calls();
            if check(&calls) {
                return calls;
            }
            assert!(
                Instant::now() < end,
                "等不到{what}：调用是 {calls:?}，屏幕是：\n{}",
                self.tui.lines().join("\n")
            );
            self.tui.pump(Duration::from_millis(50));
        }
    }

    /// 等到屏幕上最后一行有字的合 `check`，再安静一会儿；交回那一行。
    pub fn wait_line(&mut self, what: &str, check: impl Fn(&str) -> bool) -> String {
        let last = |lines: &[String]| {
            lines
                .iter()
                .rev()
                .find(|l| !l.trim().is_empty())
                .cloned()
                .unwrap_or_default()
        };
        self.tui.wait_until(what, |lines| check(&last(lines)));
        self.tui.pump(Duration::from_millis(300));
        last(&self.tui.lines())
    }

    /// 等回到提示符、命令行上是 `text`（空的就是什么都没有）。
    pub fn wait_command_line(&mut self, text: &str) {
        let want = format!("{PROMPT}{text}");
        let want = want.trim_end().to_string();
        self.wait_line(&format!("命令行上是「{text}」"), |line| line == want);
    }

    /// 给 shell 发 SIGUSR1（盯后台的进程叫它印回报那样）。
    pub fn signal_report(&mut self) {
        let pid = self.tui.pid().expect("shell 有进程号");
        let sent = std::process::Command::new("kill")
            .args(["-USR1", &pid.to_string()])
            .status()
            .expect("跑得了 kill");
        assert!(sent.success());
        self.tui.pump(Duration::from_millis(300));
    }
}

impl Drop for Hooked {
    fn drop(&mut self) {
        if let Some(base) = self.dir.parent() {
            drop(std::fs::remove_dir_all(base));
        }
    }
}

/// 照真程序印 `shell` 的钩子脚本，调的程序是 `program`；缓存目录（信箱）照 shell 的那一个。
fn render(shell: &str, program: &Path, home: &Path) -> String {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_miyu-tui"))
        .args(["shellhook", "script", "--shell", shell, "--program"])
        .arg(program)
        .env("HOME", home)
        .env("XDG_CACHE_HOME", home.join("cache"))
        .output()
        .expect("跑得了 shellhook script");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("脚本是 UTF-8")
}

/// 写一个可执行的脚本。
fn write_program(path: &Path, text: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, text).expect("写得进去");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("可执行");
}

/// 单引号包起来（测试目录里没有单引号）。
fn quote(path: &Path) -> String {
    format!("'{}'", path.display())
}
