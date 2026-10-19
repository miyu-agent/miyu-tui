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
| `src/shellhook/say/mod.rs` | `say`：连核心、主循环、每一帧排什么、收尾 |
| `src/shellhook/say/turn.rs` | 这一轮的状态：问终端会话、接上、发话、认出这句话开的那一轮、按键 |
| `src/shellhook/say/terminal.rs` | 读 `session.terminal` 的回应；隔了多久的写法 |
| `src/shellhook/say/view.rs` | 这一轮排成哪几行（照 `ui/rows.rs` 的 `entry_rows`） |
| `src/shellhook/say/paint.rs` | 印在 shell 里：画布收拾成带颜色的字、活动区原地刷新 |
| `src/shellhook/say/asking.rs` | 确认和提问的抽屉（照 `drawer/`、`ui/drawer/`） |
| `src/shellhook/say/session.rs` | 在 shell 里画一轮的那一套（`say`、`report` 共用）：连核心、原始模式、终端颜色、读按键、系统通知、主循环、收尾 |
| `src/shellhook/say/look.rs` | 配色：照 `tui.theme*`、壁纸、人格的主题色挑 |
| `src/shellhook/say/follow.rs` | 盯后台：订阅终端会话、等 shell 退出，任务结束和叫醒的那一轮记进信箱、给 shell 发信号 |
| `src/shellhook/say/spool.rs` | 盯后台的和 shell 之间的信箱（一个 shell 一个文件） |
| `src/shellhook/say/report.rs` | `report`：shell 回到提示符时印信箱里的 |
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
| `follow --watch <路径>` | `say` 起的 | 盯后台（下面「后台回报」） |
| `report --shell-pid <编号> [--leftover <路径>]` | 脚本 | 印这个 shell 信箱里的后台回报 |
| `say --shell <名字> [--shell-pid <编号>] [--leftover <路径>]` | 脚本 | 标准输入是那句话：交给她、这一轮印在 shell 里（下面「say」）。退出码：正常 0，按了 Ctrl+C 3（同 `miyu ask`），连不上核心 1 |

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

### say

1. **说到哪个会话**：先问核心 `session.terminal {cwd}`（核心 8-33）：哪个算终端会话、闲置多久换新的都由核心定（设计 22 第七节，2026-10-20 项目主人：头去建会话是向上依赖）。问到的照 `Open` 接上，等最新一页读回来（`Paged`，读不成的 `Older`）再说那句话；问不到的（核心旧）照新会话开。先读完再发：补发的那一页里有同样一句话的时候，不会把以前那一轮认成这一轮（真模型实测撞见）。
2. **在哪个目录说**：每句带 shell 所在的目录（`session.send` 的 `cwd`，核心 05eb8159）；全屏不带。
3. **印哪几条**：只印这句话开的那一轮：发出去以后，核心把这句话落盘推回来记下序号（这条连接自己发的推回来是只带序号的 `user_message`，别处看到的是带字的 `said`，认第一个），`turn.started` 的 `trigger` 是它的那一轮就是；接上时补发的最新一页不印。你说的那句 fish 已经重放过，不印。排法照全屏（`ui/rows.rs` 的 `entry_rows`，一条之间空一行），最前面空一行和提示符那一行隔开，等第一个字时末尾转着。不印运行状态行（词、用时）：shell 里只要转圈（2026-10-20 项目主人验收）。展示的卡片点不开：「点击打开」那一行换成文件的路径（同一天）。图（图片、公式、mermaid）还不显示。
3a. **待办**：发出去以后推来的待办照全屏的任务表记，在活动区末尾照侧边栏的样子画（`ui::todo_lines`）；这一轮变过的，收尾时把最后的样子一起印出来。
4. **换了会话**：回应带 `renewed` 的，这一轮前面先印一行暗色的「上次说话是 3 小时前，开了新会话」（单位取最大的一级：分钟、小时、天）。
5. **配置**：时间线那几项（`tui.timeline_*`）照全屏读、照全屏用，做完收起照 `tui.timeline_fold`。
6. **收工**：这一轮结束（`running` 回到没有）就收；还没开轮就被拒的、连不上的当场收，原因写在标准错误。
7. **按键**：读按键时终端在原始模式。Ctrl+C 打断这一轮（`session.interrupt`，排着的退回来）；还没开轮、已经按过一下的不等了，当场收。别的字记下，收工时写进 `--leftover` 的文件，脚本放回命令行。收工时先让读按键的线程停下，再把它读走还没处理的按键收下（答得快时最后几个字会丢，实测撞见）。

### 怎么印

1. 光标下面一块活动区，每一帧先擦掉上一帧（回到它的第一行、往下全清）再画，包在同步输出里（设计 22 第七节：旧版每帧先擦再写，一闪一闪）。
2. 活动区最多占终端高减一行；长过的只露最新的那一截。这一轮结束时擦掉活动区，整段照最后的样子印一遍，留在滚动区：做完收起的照配置收起。
3. 每一帧先画进一块和终端一样宽的画布，照全屏那一套收拾（宽字后面那格、`theme::paint`、照色深降色 `theme::degrade`），再一行行写成带颜色的字：宽字只写一次，行尾没颜色的空格不写。
4. 颜色：起来时问终端的字色、底色和 16 色；照终端界面的配置项 `tui.theme*`、跟随壁纸找到的、终端会话那位人格的主题色挑（`look.rs`，算法同全屏 `theme::choice::{seeds, pick}`），每句现取；不铺底色（滚动区里铺一块底很怪）。
5. 系统通知：终端不在前台时（开着焦点报告），回答好了、出错了、新开了确认或提问的抽屉，照全屏「系统通知」弹（`notify::Notifier`）。

### 确认和提问

1. 终端会话推来的确认、提问开抽屉，数据、按键、画法都照全屏（`drawer/`、`ui/drawer/`，蓝图 `tui.md`「确认和提问的抽屉」）。画在活动区最下面一个暗色圆角框里，最多占终端一半高。
2. 抽屉开着时按键先归它，带 Ctrl 的照旧归外面（Ctrl+C 打断），编辑时的 Ctrl+J 换行除外；两下 Esc 取消，取消的顺带打断这一轮。
3. 交了的经 `session.answer` 交给核心，结果等核心推回来再写进正文，挂到「提问」那一步下面；不允许的末尾一行红 ✗（同全屏第 6 条）。
4. 编辑「其他答案」时还不显示光标。

### 后台回报

设计 22 第七节「后台任务和回报」（2026-10-20 项目主人定，核心 b3777b13）。

1. **谁盯**：`say` 记下这一轮派出去的后台任务（`job.started`，前台跑的子代理不算）；收工时还有没结束的，写一份交接（终端会话、任务、`$fish_pid`）进信箱目录，起 `miyu-tui shellhook follow`：自己一个进程组、不接终端。
2. **怎么盯**：订阅终端会话（`Open`，补发的那一页不算），只认交接来的任务编号：结束了（`job.reported`、`child.reported`）记一件「结束了」；结束了都会叫醒她（子代理、后台命令都是，真模型实测），接下来不是你说的话开的那一轮记一件「那一轮」；这一轮里又派出去的接着盯。都结束了、叫醒的那几轮也跑完了，收工。shell 退出了（Linux 上 `pidfd`，等事件不轮询；macOS 的 kqueue 还没做）当场收工，不再回报。
3. **信箱**：缓存目录 `shellhook/<shell 的编号>.jsonl`，一件一行，追加；`report` 先换名再读，拿的时候追加的留给下一次。记一件给 shell 发 `SIGUSR1`。
4. **fish 那边**：收到信号时停在提示符上的（`status current-command` 是 `fish`）当场印：回到提示符第一行往下清掉，跑 `report`，补「提示符行数 - 1」个换行再 `commandline -f repaint`（fish 照记着的位置往上回这么多行重画，不补会盖掉刚印的，实测）。前台在跑命令的（vim、一行里的几条）先记着，回到提示符（`fish_prompt` 事件）再印。敲到一半的字 fish 自己留着；印的时候敲的接在后面。
5. **印什么**：「结束了」照全屏写那一行（`jobs::announce`，从全屏挪来共用：绿点、红叉）；「那一轮」照 `Open` 接上、补完最新一页后照 `say` 一样画，要确认、要回答的照常弹，画的时候按键归它。

### 还没做

拿回焦点再问终端颜色；bash、zsh 的后台回报；`/dream` 转后台；提前敲的字边答边显示；Ctrl+V 粘图；斜杠命令（`/models`、`/sessions`……，`set_terminal`）；图；`doctor`。
