## Miyu 终端界面

Miyu 的终端界面（头）：一个程序 `miyu-tui`，经本机套接字说 Miyu 的核心协议。核心在 `miyu-agent/miyu`，这里照 `Cargo.toml` 里的 git 依赖钉住核心的一个提交（设计 `docs/designs/32-仓库拆分.md`，在核心仓库）。

### 规矩

- 只经 `miyu-client` 用核心的代码（设计 32 第二节）；缺的告诉核心的主会话加进门面，不直接依赖核心里别的 crate。
- 协议、事件的形状以核心仓库的 `docs/blueprint/protocol.md`、`docs/designs/samples/` 为准；改到协议的事先找核心。
- 升级核心：改 `Cargo.toml` 里钉住的提交，过三台 CI。
- 合进 main：本机测试（`--all-targets`）、clippy、fmt、文档全过就合，三台 CI 在 main 上跑，红了当场修（2026-10-17 项目主人：「改个 tui 而已为什么要 ci」，同核心 10-16 定的）。先过三台再合的只有：升级核心；碰路径、文件系统、子进程、终端输入输出、`cfg` 分平台的写法。
- 提交和 PR 的标题照核心的写法（2026-10-22 项目主人：「commit 的习惯所有仓库一致会好一些」，核心 AGENTS.md 第 2 条）：`<类型>(<范围>): <这次改动的效果，说人话> (<编号>)`，类型是 `feat`、`fix`、`perf`、`refactor`、`docs`、`test`、`chore` 之一，例如 `feat(会话): 排队的消息点一下就撤回 (T-352)`；编号写验收清单的 T 编号，没有的不写括号。标题不写内部代号的说法和实现细节。正文三段：为什么、改了什么、怎么验。一件事一个提交：图纸、字表和代码进同一个提交；PR 照 squash 合，合并时的标题照这个写。进度和给人看的字里不写 sha，钉核心版本的那一处照旧写。已经推了的不改。
- 每个 `.rs` 文件不超过 500 行，测试也算；clippy `-D warnings`；不吞错误（不写 `let _ =`、`.ok()` 扔错误）。
- 给人看的字照三种语言（`resources/text/`）；界面上只放人要的，说法照通用软件的短句。
- 图纸在 `docs/blueprint/tui.md`；改了界面先改图纸。
- 本机测：沙箱数据根（`MIYU_HOME` 指到临时目录），不碰 `~/.miyu`，不重启别人在用的核心。
- 平台：主要照 Linux、macOS（M 系列）做和测。Windows 上要编得过（格式、clippy、文档、单元测试照跑），伪终端测试只在 Linux、macOS 上跑：ConPTY 转按键的时序不稳，Esc 这类键会偶发丢。Unix 专用的写法关在 `cfg(unix)` 里，测试照写死的 Unix 路径、`/bin/sh` 脚本的也是。
