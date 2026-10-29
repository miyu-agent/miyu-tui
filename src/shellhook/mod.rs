//! 终端无缝集成（shellhook，设计 22 第七节，图纸 `docs/blueprint/shellhook.md`）：`miyu-tui shellhook …`，核心把
//! `miyu shellhook …` 转交过来。在 shell 里直接和她说话，时间线照终端界面那一套画、配置项照 `tui.*` 那一套读。
//!
//! 给人用的是 `install`、`uninstall`、`status`；`script`、`classify`、`say` 是 shell 脚本调的，帮助里不列。

mod classify;
mod doctor;
mod install;
mod paste;
mod say;
// 终端界面的第一次引导接上以前没人调（`setup.rs`）。
mod script;
#[allow(dead_code)]
pub mod setup;
mod shells;
mod slash;
mod words;

use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use shells::Shell;
use words::Words;

/// `miyu shellhook -h` 印的用法。
pub const USAGE: &str = "Usage: miyu shellhook <command> [options]

Talk to Miyu from your shell: type a sentence at the prompt and press Enter.

Commands:
  install [--shell <fish|bash|zsh>] [--program <path>]
                 hook into the shell (detected from the parent process unless --shell is given)
  uninstall [--shell <fish|bash|zsh>]
                 remove the hook
  status         show which shells have the hook
  doctor         check the hook, the core, the terminal's colors and the clipboard

Options:
  --shell <name>     fish, bash or zsh
  --program <path>   the program the hook calls (default: the `miyu` next to this program)
  -h, --help         print this help";

/// 自己的子命令：不当斜杠命令认。
const INTERNAL: [&str; 10] = [
    "install",
    "uninstall",
    "status",
    "doctor",
    "script",
    "classify",
    "say",
    "follow",
    "report",
    "paste",
];

/// 用法写错了：退出码 2，同核心命令行（设计 22 O1）。
const BAD_USAGE: u8 = 2;

/// 照参数办：`args` 是 `shellhook` 后面的那些。
pub fn main(args: Vec<String>) -> ExitCode {
    let words = match Words::detect() {
        Ok(words) => words,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let (hook, args) = hook(args);
    // 开着的 shell 里是升级以前印的脚本（`script.rs`）：说话时、旧脚本调的参数这一版不认时说一声重开终端。
    let stale = hook.is_some_and(|hook| hook != script::stamp());
    let code = run(args, &words, stale);
    if stale && code == ExitCode::from(BAD_USAGE) {
        eprintln!("{}", words.stale_hook);
    }
    code
}

/// 脚本调的头上带的 `--hook <记号>` 拿出来，剩下的照原样。
fn hook(mut args: Vec<String>) -> (Option<String>, Vec<String>) {
    if args.len() >= 2 && args[0] == "--hook" {
        let stamp = args.remove(1);
        args.remove(0);
        return (Some(stamp), args);
    }
    (None, args)
}

fn run(args: Vec<String>, words: &Words, stale: bool) -> ExitCode {
    let (command, options) = match args.split_first() {
        None => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Some((first, _)) if first == "-h" || first == "--help" => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Some((command, rest)) => (command.as_str(), rest),
    };
    // 斜杠命令（`miyu shellhook models`）：名字后面的照参数（`compact 要求`），不是选项。
    if !INTERNAL.contains(&command) {
        let line = std::iter::once(format!("/{command}"))
            .chain(options.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");
        match slash::run(&line, words, None, None) {
            Some(Ok(code)) => return code,
            Some(Err(e)) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
            None => {}
        }
    }
    let options = match Options::parse(options) {
        Ok(options) => options,
        Err(e) => {
            eprintln!("{e}\n\n{USAGE}");
            return ExitCode::from(BAD_USAGE);
        }
    };
    let result = match command {
        "install" => with_shell(&options, words, |shell| {
            install::install(shell, options.program.clone(), words)
        }),
        "uninstall" => with_shell(&options, words, |shell| install::uninstall(shell, words)),
        "status" => install::status(words),
        "doctor" => {
            return match doctor::run(words) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            };
        }
        "script" => match (options.shell, &options.program) {
            (Some(shell), Some(program)) => {
                print!("{}", script::render(shell, program));
                Ok(())
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "script needs --shell and --program",
            )),
        },
        "classify" => return classify_stdin(options.shell.unwrap_or(Shell::Fish)),
        "say" => {
            return say(
                options.input.as_deref(),
                options.leftover,
                options.shell_pid,
                options.redo,
                stale,
                words,
            );
        }
        "follow" => match &options.watch {
            Some(path) => return say::follow::run(path),
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "follow needs --watch",
            )),
        },
        // `Ctrl+V`：读剪贴板，块上的字印出来给脚本插进命令行（`paste.rs`）。
        "paste" => match options.shell_pid {
            Some(pid) => match paste::run(pid) {
                Ok(code) => return code,
                Err(e) => Err(e),
            },
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "paste needs --shell-pid",
            )),
        },
        "report" => match options.shell_pid {
            // 只问信箱里有没有要印的（zsh 停在提示符上：有的才让出提示符）。
            Some(pid) if options.pending => {
                return if say::report::waiting(pid) {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                };
            }
            Some(pid) => match say::report::run(pid, options.leftover.as_deref(), options.above) {
                Ok(code) => return code,
                Err(e) => Err(e),
            },
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "report needs --shell-pid",
            )),
        },
        other => {
            eprintln!("unknown command: {other}\n\n{USAGE}");
            return ExitCode::from(BAD_USAGE);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// 认得的选项：都是具名的。
#[derive(Debug, Default, PartialEq, Eq)]
struct Options {
    shell: Option<Shell>,
    program: Option<PathBuf>,
    /// `say` 提前敲的字写在哪（脚本读了放回命令行）。
    leftover: Option<PathBuf>,
    /// `say` 那句话从这个文件读，不读标准输入（PowerShell：管道照 `$OutputEncoding` 编码，5.1 默认 ASCII）。
    input: Option<PathBuf>,
    /// 哪个 shell（`$fish_pid`）：`say` 交给盯后台的、`report` 拿哪个信箱。
    shell_pid: Option<u32>,
    /// `follow` 读的那份。
    watch: Option<PathBuf>,
    /// `report` 停在提示符上印：提示符在光标上面占几行，有要印的才擦掉它们。
    above: Option<u16>,
    /// `report` 只问有没有要印的。
    pending: bool,
    /// `say` 这一句是 `/edit` 改过的：替换上一轮重来。
    redo: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = args.iter();
        while let Some(flag) = args.next() {
            let value = || format!("{flag} needs a value");
            match flag.as_str() {
                "--shell" => {
                    let name = args.next().ok_or_else(value)?;
                    options.shell =
                        Some(Shell::named(name).ok_or_else(|| format!("unknown shell: {name}"))?);
                }
                "--program" => options.program = Some(args.next().ok_or_else(value)?.into()),
                "--leftover" => options.leftover = Some(args.next().ok_or_else(value)?.into()),
                "--input" => options.input = Some(args.next().ok_or_else(value)?.into()),
                "--watch" => options.watch = Some(args.next().ok_or_else(value)?.into()),
                "--pending" => options.pending = true,
                "--redo" => options.redo = true,
                "--above" => {
                    let rows = args.next().ok_or_else(value)?;
                    options.above = Some(rows.parse().map_err(|_| format!("bad rows: {rows}"))?);
                }
                "--shell-pid" => {
                    let pid = args.next().ok_or_else(value)?;
                    options.shell_pid = Some(pid.parse().map_err(|_| format!("bad pid: {pid}"))?);
                }
                other => return Err(format!("unknown option: {other}")),
            }
        }
        Ok(options)
    }
}

/// 给了 `--shell` 的照它，没给的照上一层进程认；都认不出说一声。
fn with_shell(
    options: &Options,
    words: &Words,
    then: impl FnOnce(Shell) -> io::Result<()>,
) -> io::Result<()> {
    match options.shell.or_else(shells::detect) {
        Some(shell) => then(shell),
        None => Err(io::Error::other(words.unknown_shell.clone())),
    }
}

/// 多行的一段交给它判：是命令退出码 0，是一句话 1。
fn classify_stdin(shell: Shell) -> ExitCode {
    let mut text = String::new();
    if io::stdin().read_to_string(&mut text).is_err() {
        return ExitCode::from(BAD_USAGE);
    }
    if classify::is_shell_command(&text, shell) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// 把那句话（`input` 文件里的，没给的读标准输入）交给她，这一轮印在 shell 里（`say/`）。
fn say(
    input: Option<&std::path::Path>,
    leftover: Option<PathBuf>,
    shell: Option<u32>,
    redo: bool,
    stale: bool,
    words: &Words,
) -> ExitCode {
    let read = match input {
        Some(path) => std::fs::read_to_string(path),
        None => {
            let mut text = String::new();
            io::stdin().read_to_string(&mut text).map(|_| text)
        }
    };
    let text = match read {
        Ok(text) => text,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    // 那句话是从管道交来的：读完以后标准输入接回终端，问终端颜色、读按键才读得到（2026-10-21 项目主人：「TUI 里是真色彩。
    // 终端里不是」——问颜色的看标准输入不是终端就不问，退回 16 色）。接不上的照旧，只是问不到颜色。
    #[cfg(unix)]
    if let Err(e) = stdin_to_tty() {
        eprintln!("{e}");
    }
    match say::run(
        say::Ask {
            text,
            leftover,
            shell,
            redo,
            stale,
        },
        words,
    ) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// 标准输入不是终端（管道交来的）的接回终端（`/dev/tty`）。
#[cfg(unix)]
fn stdin_to_tty() -> io::Result<()> {
    use std::io::IsTerminal;
    if io::stdin().is_terminal() {
        return Ok(());
    }
    let tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")?;
    rustix::stdio::dup2_stdin(&tty)?;
    Ok(())
}

#[cfg(test)]
mod tests;
