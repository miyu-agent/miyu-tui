# 总状态 App 拆分（review E1）

状态：2026-10-12 项目主人看过，照这份做；同一天八步都做完（PR #10–#13），`App` 的字段 80 → 12（5 个字段加 7 块），画面层拿整个 `App` 的文件 20 → 1（顶层 `ui::draw`）。

## 一、为什么拆

2026-10-12 量的：

| 量 | 现在 |
|---|---|
| `App` 的字段 | 80 个 |
| `src/app/` 里改它的文件 | 40 个 |
| 画面层拿着整个 `App` 的文件 | 20 个，其中 10 个拿的是能改的 `&mut App` |
| 被最多文件碰的字段 | `config` 50 个、`transcript` 33、`core` 27、`panel` 19、`input` 16、`view` 16 |
| 只有一个文件碰的字段 | 24 个 |

问题：
1. 任何一处都能改任何一样，改一个功能要读的面很大；review 里 B 批那几条（切会话、断线、确认归属）都出在这几样互相牵扯的地方。
2. 画的时候顺手改状态：各块位置、每一行对应第几项、视口、吉祥物的动画都在画的函数里写回。画两次结果会不一样，画面没法单独测。
3. 很多字段只有一两个地方用，挂在总状态上谁都看得见。

## 二、拆成什么样

`App` 只留协调用的几样，其余按「谁一起变」分成几块，每块一个结构体、自己的方法写在块上。块与块之间不互相拿，要两块一起动的事由 `App` 上的方法办（现在 `src/app/*.rs` 里那些方法大多就是干这个的）。

| 块 | 管什么 | 字段（现在的名字） |
|---|---|---|
| `App` 本身 | 协调、退出、挂起 | `config`、`notifier`、`quit`、`suspend`、`cwd` |
| `link` 连接 | 和核心说话、核心给的资料 | `core`、`abilities`、`human`、`currency`、`default_chat`、`system_language` |
| `talk` 会话 | 正在看的和停放着的会话、切换、确认和提问 | `transcript`、`board`、`feed`、`parked`、`home`、`left`、`opening`、`sessions_seen`、`unsent`、`takeback`、`asks`、`drawers`、`todo_full`、`compaction_announced`、`workspace_pending`、`persona`、`preset`、`demo_drawers` |
| `typing` 输入 | 输入框和它上面的列表、编辑器 | `input`、`menu`、`mention`、`history`、`history_placed`、`old_history`、`focus`、`esc_at`、`composing`、`staging`、`editor`、`edit`、`ime`、`tips`、`notice` |
| `boxes` 开着的框和整屏页 | 面板、各种框、配置页、引导、用量 | `panel`、`session_list`、`model_list`、`efforts`、`usage`、`settings`、`settings_prompt`、`oobe`、`oobe_editing`、`welcome_checked`、`welcome_known` |
| `media` 要画的东西的账 | 排好的行、图、卡片、头像、吉祥物包 | `md_cache`、`row_cache`、`figures`、`cards`、`diagrams`、`avatars`、`mascot_pick` |
| `mascot` 吉祥物 | 视线、待机动作、嘴、落脚点、运行状态的词 | `gaze`、`idle`、`attention`、`perch`、`mouth`、`pulse` |
| `screen` 上一帧画出来的 | 鼠标、点击要照它认的 | `areas`、`view`、`caret`、`drawer_rows`、`panel_rows`、`agents_hover`、`pointer`、`grab`、`side_select`、`theme`、`started`、`rng` |

拆完 `App` 剩 5 个字段加 7 块。

## 三、画面只读

2026-10-12 动手时调整（技术定）：试着把画面层全改成只拿 `&App` 编一次，编译器指出四十来处在写，除了位置、行表、视口这些排版结果，
输入框、抽屉、配置页、引导弹窗都把「滚到哪」记在自己身上、画的时候照光标挪（ratatui 有状态控件的惯例），吉祥物的动画要照这一帧
排出来的位置推一步。硬改成完全只读得把这些全搬出来，面太大。改成：

1. 画的时候能改的只有「画面状态」：`screen` 块（各块位置、抽屉和面板每一行是第几项、侧边栏能选的行、光标、视口）、各控件的滚动
   位置、`mascot` 块（动画推一步）。
2. 会话、任务表、输入的字、框开没开这些「内容状态」画的时候只读。
3. 缓存（排好的行、做好的图、链接卡片的账）照旧是 `RefCell`：它们是「算过的记住」，画两次结果一样。
4. 这一条靠函数签名守：画的函数拿内容各块的 `&`，加 `&mut Screen`、`&mut MascotState`，不拿整个 `App`。所以等各块都搬好了
   最后做（第四节第 8 步）。

2026-10-12 第 8 步做完，落地的样子：

- `Seen`（`app/seen.rs`）：配置和内容各块（`talk`、`typing`、`boxes`、`link`、`media`）的只读借用，加终端所在的目录、启动时刻；
  跨块的只读查询（焦点、花了多少、现在的工作区、选的人格和预设、头像、引导走没走过……）挂在它上面，`App` 里要用的写
  `self.seen().查询()`。
- `App::parts()` 一次拆出 `(Seen, &mut Screen, &mut MascotState)`；正文、首页、侧边栏、运行状态行、吉祥物、头像拿它，`/usage`、
  面板、底栏、子代理行、回到底部的按钮只拿 `&Seen`（要读上一帧画面的另拿 `&Screen`）。
- 画对话那一帧以前先走 `App::before_frame()`（`app/frame.rs`）：换提示、同步斜杠命令列表、`@` 文件列表的词变了问核心，
  交回两个列表；原来这几样在画的函数里边画边改。
- 自带滚动位置的控件在签名里单写可写：输入框拿 `&mut InputBox`、开着的抽屉 `Option<&mut Drawer>`，读的几样先算好成
  `input_box::Facts`；引导拿 `oobe::Parts`（配置、图的账只读，引导自己、上一帧画面、吉祥物可写）；配置页照旧拿
  `&mut Settings`。
- 画面层只剩顶层 `ui::draw` 拿 `&mut App`：它是协调的那一层，照上面几样拆借用、往下交。

## 四、怎么搬

每一步只搬家、不改行为，一步一个提交；整套测试（现在 935 条，含伪终端测试）每步都跑，过了才走下一步。两三步一个 PR。

| 步 | 做什么 | 碰的面 |
|---|---|---|
| 1 | `mascot`：最小、最独立，先把「块 + 方法挂在块上」的样子立起来 | `mascot*`、`home`、`status`、`oobe/figure` |
| 2 | `screen` | 鼠标、点击、`ui/` |
| 3 | `media` | `ui/rows`、`cards.rs`、`diagrams.rs`、`lost.rs` |
| 4 | `boxes` | 各个框、配置页、引导 |
| 5 | `typing` | `keys.rs`、`compose.rs`、输入框 |
| 6 | `link` | `core` 那 27 处 |
| 7 | `talk` | 最大的一块，最后搬：`sessions`、`switch`、`asking`、`takeback`、`pages` |
| 8 | 画面层改拿块：`ui/` 里的函数照用到的几块收窄参数，内容各块 `&`、`screen` 和 `mascot` `&mut`（第三节；2026-10-12 做完） | `ui/` |

1. 每一步完了量一次：`App` 还剩几个字段、画面层几个文件拿着整个 `App`，写进 PR。
2. 文件超了 500 行的顺着块拆（`src/app/` 下按块分子目录）。
3. 不顺手改行为、不改界面上的字；搬的时候看到的 bug 记下来，另起提交修。

## 五、风险

- `talk` 那一块和别的块牵得最多（切会话要动视口、框、输入框）：放最后，前面几块先把「两块一起动的事放在 App 上」的写法练熟。
- 搬的时候借用规则会逼着把一些「顺手改别的」的地方拆开，那正是要的；碰到确实要两块同时可变的，写成 `App` 上的方法，先借这块再借那块。
- 不改协议、不改核心，两个头的约定不受影响。
