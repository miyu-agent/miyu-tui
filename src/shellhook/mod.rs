//! 终端无缝集成（shellhook，设计 22 第七节，图纸 `docs/blueprint/shellhook.md`）：`miyu-tui shellhook …`，核心把
//! `miyu shellhook …` 转交过来。在 shell 里直接和她说话，时间线照终端界面那一套画、配置项照 `tui.*` 那一套读。
//!
//! 给人用的是 `install`、`uninstall`、`status`；`script`、`classify`、`say` 是 shell 脚本调的，帮助里不列。

mod classify;
mod install;
mod say;
mod script;
mod shells;
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

Options:
  --shell <name>     fish, bash or zsh
  --program <path>   the program the hook calls (default: the `miyu` next to this program)
  -h, --help         print this help";

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
    let options = match Options::parse(options) {
        Ok(options) => options,
        Err(e) => {
            eprintln!("{e}\n\n{USAGE}");
            return ExitCode::from(BAD_USAGE);
        }
    };
    let result = match command {
        "install" => with_shell(&options, &words, |shell| {
            install::install(shell, options.program.clone(), &words)
        }),
        "uninstall" => with_shell(&options, &words, |shell| install::uninstall(shell, &words)),
        "status" => install::status(&words),
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
        "say" => return say(options.leftover, &words),
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

/// 把标准输入的那句话交给她，这一轮印在 shell 里（`say/`）。
fn say(leftover: Option<PathBuf>, words: &Words) -> ExitCode {
    let mut text = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut text) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    match say::run(say::Ask { text, leftover }, words) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests;
