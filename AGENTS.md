## Miyu 终端界面

Miyu 的终端界面（头）：一个程序 `miyu-tui`，经本机套接字说 Miyu 的核心协议。核心在 `miyu-agent/miyu`，这里照 `Cargo.toml` 里的 git 依赖钉住核心的一个提交（设计 `docs/designs/32-仓库拆分.md`，在核心仓库）。

### 规矩

- 只经 `miyu-client` 用核心的代码（设计 32 第二节）；缺的告诉核心的主会话加进门面，不直接依赖核心里别的 crate。
- 协议、事件的形状以核心仓库的 `docs/blueprint/protocol.md`、`docs/designs/samples/` 为准；改到协议的事先找核心。
- 升级核心：改 `Cargo.toml` 里钉住的提交，过三台 CI。
- 每个 `.rs` 文件不超过 500 行，测试也算；clippy `-D warnings`；不吞错误（不写 `let _ =`、`.ok()` 扔错误）。
- 给人看的字照三种语言（`resources/text/`）；界面上只放人要的，说法照通用软件的短句。
- 图纸在 `docs/blueprint/tui.md`；改了界面先改图纸。
- 本机测：沙箱数据根（`MIYU_HOME` 指到临时目录），不碰 `~/.miyu`，不重启别人在用的核心。
