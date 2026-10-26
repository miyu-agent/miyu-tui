## Miyu 终端界面

全屏的终端界面：程序 `miyu-tui`，连上 Miyu 的核心跟 AI 说话。核心在 [miyu-agent/miyu](https://github.com/miyu-agent/miyu)，这里照 `Cargo.toml` 的 git 依赖钉住核心的一个提交，只经门面 `miyu-client` 用核心的东西（核心仓库的设计 `32-仓库拆分.md`）。现在长什么样，见蓝图 `docs/blueprint/tui.md`。

### 怎么跑

先在核心仓库的检出里编一次核心（`cargo build -p miyu`），下面的 `<核心检出>` 换成它的路径。

第一次用这个 `MIYU_HOME` 时，在这个仓库的根目录装上终端自己的软件包 `tui/`（核心 S-5 起不再自带它：`tui.*` 配置项、放输入历史的状态目录都照它认；编号照文件夹名，发版时核心照 `heads.toml` 放进 `packages/tui/`）：

```sh
DEEPSEEK_API_KEY=<key> MIYU_HOME=$HOME/.cache/miyu-tui-home MIYU_RESOURCES=<核心检出>/resources <核心检出>/target/debug/miyu pkg install ./tui --yes
```

`miyu pkg install` 会把核心拉起来、留着跑，下面的 `cargo run` 连的就是它：key 这一步也要给上。改了 `tui/` 里的清单的，再装一次。

然后跑：

```sh
DEEPSEEK_API_KEY=<key> MIYU_HOME=$HOME/.cache/miyu-tui-home MIYU_RESOURCES=<核心检出>/resources MIYU_CORE_BIN=<核心检出>/target/debug/miyu cargo run
```

- 变量写在命令前面，不要 `export`：旧版 Miyu 也认 `MIYU_HOME`。
- `MIYU_HOME` 放在持久目录里，别放 `/tmp`：`/tmp` 是 tmpfs，一重启模型配置、会话记录全没（2026-10-02 撞见过一次）。
- 拉起核心只认 `MIYU_CORE_BIN`，不去 PATH 里找 `miyu`（PATH 上的可能是旧版）。核心已经在跑的，不给也能连上。

### 按键、鼠标、版式

都写在蓝图 `docs/blueprint/tui.md` 里（「对外的样子」「怎么走」「样子」），这里不再抄一份。

### herdr 中恢复会话

`miyu-tui --resume <完整会话 ID>` 恢复指定会话，优先于 `tui.startup=recent`。在 herdr 中创建或切换主会话后自动上报恢复命令；服务重启后每个窗格恢复各自的会话。`/new` 清掉旧绑定。只退出 herdr 客户端时原进程继续运行。

恢复程序 `miyu-tui` 必须在恢复 shell 的 PATH 中，MIYU_HOME、MIYU_RESOURCES、MIYU_CORE_BIN 也须可用；临时写在原启动命令前的环境变量不会被 herdr 保存。试用时可以写一个固定这些环境的启动脚本，放到 PATH 中；独立命名 herdr 会话先试，不停止默认服务。

Linux 隔离验收（需要 herdr）：`cargo test --test herdr_resume -- --ignored`。使用临时数据根、假模型和独立 XDG 配置，验证两窗格各自恢复、/new 清掉旧绑定，不读写真实会话。

### 许可证

这个仓库自己写的代码和资源用 AGPL-3.0-or-later，见 `LICENSE`：改过的终端界面拿去给别人用，包括只在网上提供服务，要把改过的源码交给用它的人。

不是这个仓库自己写的东西照它们原来的许可证，AGPL 只管这里自己写的部分：Rust 依赖照各自的 `license`（`Cargo.lock` 里列着，`cargo metadata` 查得到）；用到的第三方资源照各自的许可证，发行时随包附上原文。
