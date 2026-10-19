## 终端无缝集成（shellhook）

状态：施工中。设计 `22-命令行.md` 第七节、O6（核心仓库；2026-10-20 项目主人定：叫 shellhook，并进终端界面这个包，程序就是 `miyu-tui`）。时间线、主题、抽屉的样子照 `tui.md`，这一页只写 shellhook 自己的；共用的写「照 `tui.md` 某节」，不抄一份。

### 是什么

在 shell 里直接和她说话：提示符上敲一句话回车，她的回答照终端界面时间线的样子印在 shell 里（不全屏、不能点开）。核心把 `miyu shellhook …` 转交成 `miyu-tui shellhook …`（清单里第二个 `[[command]]`，核心 F-13）。

### 在哪

| 代码 | 管什么 |
|---|---|
| `src/shellhook/mod.rs` | 参数（都是具名的）、分给几个子命令、退出码 |
| `src/shellhook/install.rs` | 装、删、看装没装 |
| `src/shellhook/script.rs` | shell 起来时现印的脚本 |
| `src/shellhook/shells.rs` | 认得的三种 shell：叫什么、装在哪、引号怎么写、照上一层进程认 |
| `src/shellhook/classify.rs` | 多行的一段是命令还是话（照旧版 `--shell-classify`） |
| `src/shellhook/words.rs` | 给人看的字 |
| `resources/shellhook/hook.{fish,bash,zsh}` | 三种 shell 的脚本，`@PROGRAM@` 印的时候换成装的时候记下的程序 |
| `resources/shellhook/text/<代码>.json` | 给人看的字，一种语言一份，和 `resources/text/` 一样多 |

### 命令

| 命令 | 谁用 | 做什么 |
|---|---|---|
| `install [--shell <名字>] [--program <路径>]` | 人 | 装进 shell。`--shell` 没给的照上一层进程认（Linux 读 `/proc/<ppid>/comm`），再照 `SHELL`；认不出的说一声、退出码 1 |
| `uninstall [--shell <名字>]` | 人 | 删掉 |
| `status` | 人 | 三种 shell 各一行：装没装、装在哪；记的程序不在了的说要重装；旧版那一份也在的说一声 |
| `script --shell <名字> --program <路径>` | 装进去的那一行 | 印这种 shell 的脚本 |
| `classify --shell <名字>` | 脚本 | 标准输入是多行的一段：是命令退出码 0，是话 1 |
| `say --shell <名字>` | 脚本 | 标准输入是那句话：交给她、印回答（施工中，现在说一声退出码 1） |

1. 帮助（`-h`）只列给人用的三个。用法写错了退出码 2，别的出错 1，同核心命令行（设计 22 O1）。
2. `--program`：脚本调哪个程序，没给的照这个程序旁边的 `miyu`（随 Miyu 装的界面放在它旁边，设计 32 第六节），没有的调自己。不按名字去 PATH 里找：并行期 PATH 上的 `miyu` 是旧版（设计 27）。临时部署用包装脚本的，装的时候写 `--program <包装脚本>`。

### 装在哪、装进去什么

1. fish：`$XDG_CONFIG_HOME/fish/conf.d/miyu-shellhook.fish`（没设的 `~/.config`），整个文件是自己的；bash：`~/.bashrc`；zsh：`$ZDOTDIR/.zshrc`（没设的 `~`），启动文件里一段，前后带记号（`# >>> miyu shellhook >>>`、`# <<< miyu shellhook <<<`）。
2. 装进去的只有一行：交互的 shell、程序在，就跑 `<程序> shellhook script --shell <名字> --program <程序> | source`（bash、zsh 是 `eval`）。脚本每次 shell 起来时现印，升级以后自动是新的，不另外提供 `init`（2026-10-20 项目主人定）。
3. 不动不是自己写的：fish 的那个文件第一行不是记号的，装、删都不动它，装的说一声、退出码 1。启动文件里只换、只删记号中间那一段，别的原样。
4. 再装一次换掉原来那一段，不加第二段；回「已更新」。
5. 顺着符号链接写到它指的文件，链接还是链接；先写在旁边再换过去，权限照原来的。
6. 旧版那一份（fish 的 `conf.d/miyu.fish`）也在的，装完、`status` 时说一声：两份都接管回车，一句话会交两遍。不删它。

### fish 的脚本

底子是旧版的 fish 配置（设计 22 第七节），只换了调的程序：

1. 回车：空行照常；单行的看没展开的首词（`--tokens-raw`），是普通词又不是任何命令的交给她，别的交回 fish；多行的先看首个命令在不在，不在的问 `classify`。
2. 交给她：记进 fish 的历史，清空输入行，在提示符那一行用报错色重放这句话；清空和重放包在一段同步输出里，不闪（旧版 2026-10-01 报过）。
3. Ctrl+J 换行。找不到命令时兜底交给她（`fish_command_not_found`）。
4. bash、zsh 只做找不到命令时交给她（`command_not_found_handle`、`command_not_found_handler`）。

### 还没做

`say`（照时间线的样子印、颜色每轮现取、提前敲的字交还命令行、确认和提问的抽屉）、Ctrl+V 粘图、斜杠命令、闲置换了会话的那一行、`doctor`。终端会话照核心的 `session.terminal`、`session.set_terminal`（施工中）。
