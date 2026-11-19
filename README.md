## Miyu 终端界面

这个仓库是 Miyu 的终端界面：程序 `miyu-tui` 是一个全屏的终端程序，它连接 Miyu 的核心，让你在终端里和 AI 对话。核心在 [miyu-agent/miyu](https://github.com/miyu-agent/miyu)。这个仓库在 `Cargo.toml` 里用 git 依赖固定核心的一个提交，并且只通过核心提供的 `miyu-client` 使用核心的功能。

使用方法（启动参数、按键、斜杠命令、设置）见手册的 [终端界面](https://github.com/miyu-agent/miyu/blob/main/docs/zh/guide/heads/tui.md)。

### 运行

运行之前，需要先在核心仓库的检出里编译一次核心：

```sh
cargo build -p miyu
```

下面命令里的 `<核心检出>` 要换成核心仓库检出的路径。模型的配置用核心的 `cargo xtask dev-home` 生成（见 [贡献代码](https://github.com/miyu-agent/miyu/blob/main/docs/zh/dev/contributing.md)），它写进配置的供应商地址和密钥分别引用环境变量 `MIYU_DEV_BASE_URL` 和 `MIYU_DEV_API_KEY`。

第一次使用一个新的 `MIYU_HOME` 时，要先在这个仓库的根目录安装终端界面自己的软件包 `tui/`。核心不自带这个软件包，而 `tui.*` 配置项和存放输入历史的状态目录都由它登记：

```sh
MIYU_DEV_BASE_URL=<地址，写到 /v1> MIYU_DEV_API_KEY=<key> MIYU_HOME=$HOME/.cache/miyu-tui-home MIYU_RESOURCES=<核心检出>/resources <核心检出>/target/debug/miyu pkg install ./tui --yes
```

`miyu pkg install` 会启动核心并让它保持运行，后面的 `cargo run` 连接的就是这个核心，所以这一步也要提供地址和密钥。如果修改了 `tui/` 里的清单，需要重新安装一次。

> 软件包的编号取自文件夹的名字，配置键的前缀由清单里的 `prefix = "tui"` 决定。发版时，核心按照 `heads.toml` 把这个软件包放进 `packages/tui/`。

然后运行终端界面：

```sh
MIYU_DEV_BASE_URL=<地址，写到 /v1> MIYU_DEV_API_KEY=<key> MIYU_HOME=$HOME/.cache/miyu-tui-home MIYU_RESOURCES=<核心检出>/resources MIYU_CORE_BIN=<核心检出>/target/debug/miyu cargo run
```

运行时需要注意：

- 环境变量写在命令前面，不要 `export`，因为旧版 Miyu 也会读取 `MIYU_HOME`。
- `MIYU_HOME` 要放在不会被清空的目录里，不要放在 `/tmp`。`/tmp` 是 tmpfs，电脑重启以后，模型配置和会话记录会全部丢失。
- 启动核心时只使用 `MIYU_CORE_BIN` 指定的程序，不会到 `PATH` 里查找 `miyu`，因为 `PATH` 上的可能是旧版。如果核心已经在运行，不设置这个变量也能连接。

### 按键、鼠标、版式

见手册的 [终端界面](https://github.com/miyu-agent/miyu/blob/main/docs/zh/guide/heads/tui.md)，这里不重复。

### 在 herdr 中恢复会话

`miyu-tui --resume <完整的会话编号>` 用来恢复指定的会话，它的优先级高于 `ui.startup = "recent"`。

在 herdr 中新建或者切换主会话以后，终端界面会自动向 herdr 上报恢复命令。herdr 的服务重启以后，每个窗格会恢复各自的会话。运行 `/new` 会清除旧的绑定。如果只是退出 herdr 的客户端，原来的进程会继续运行。

恢复时使用的程序 `miyu-tui` 必须在恢复所用 shell 的 `PATH` 里，`MIYU_HOME`、`MIYU_RESOURCES`、`MIYU_CORE_BIN` 也必须可用。临时写在原启动命令前面的环境变量不会被 herdr 保存，所以试用时可以写一个固定了这些环境变量的启动脚本，放进 `PATH`。可以先用一个单独命名的 herdr 会话试用，这样不需要停止默认的服务。

在 Linux 上可以运行隔离的验收测试（需要安装 herdr）：

```sh
cargo test --test herdr_resume -- --ignored
```

这个测试使用临时的数据根、假模型和独立的 XDG 配置，验证两个窗格各自恢复会话、`/new` 清除旧的绑定，不会读写真实的会话。

### 许可证

这个仓库自己编写的代码和资源使用 AGPL-3.0-or-later 许可证，全文见 `LICENSE`。如果你把修改过的终端界面提供给别人使用（包括只在网络上提供服务），需要把修改后的源码提供给使用它的人。

不是这个仓库自己编写的内容沿用它们原来的许可证，AGPL 只适用于这个仓库自己编写的部分：

- 编进程序的 Rust 依赖沿用各自的 `license`。
- `tokyonight` 的两套配色是按照 folke/tokyonight.nvim 的颜色移植的，许可证是 Apache-2.0，原文在 `resources/themes/LICENSE-tokyonight.txt`。

这些内容的版权声明和许可证原文合并在 `THIRD-PARTY-LICENSES.txt` 里（由 `cargo xtask notices` 生成），随发行包一起发布。
