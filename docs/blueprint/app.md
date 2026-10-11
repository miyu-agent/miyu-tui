# 总状态 App 拆分（review E1）

状态：2026-10-12 项目主人看过，照这份做。

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

1. 画之前先走一步动画：`App::animate(now)` 推吉祥物的视线、嘴、落脚点、运行状态的词（现在在 `mascot_view`、`home`、`oobe/figure`、`status` 里边画边推）。
2. 画：`ui::draw(frame, &App) -> Drawn`，只读。排版的结果收进 `Drawn` 交回：各块位置、抽屉和面板每一行是第几项、侧边栏能选的行、光标、视口这一帧的第一行和行表、要的头像。
3. 画完：`App::drawn(Drawn)` 写回 `screen`。
4. 缓存（排好的行、做好的图、链接卡片的账）照旧是 `RefCell`：它们是「算过的记住」，不改看得见的状态，画两次结果一样。
5. 画面层的函数拿它要的那几块（例如 `&Talk`、`&Screen`、`&Config`），不拿整个 `App`。

## 四、怎么搬

每一步只搬家、不改行为，一步一个提交；整套测试（现在 935 条，含伪终端测试）每步都跑，过了才走下一步。两三步一个 PR。

| 步 | 做什么 | 碰的面 |
|---|---|---|
| 1 | `mascot`：最小、最独立，先把「块 + 方法挂在块上」的样子立起来 | `mascot*`、`home`、`status`、`oobe/figure` |
| 2 | 画面只读：`animate`、`Drawn`、`drawn`，画的函数改拿 `&App` | `ui/` 那 10 个 `&mut App` |
| 3 | `screen` | 鼠标、点击、`ui/` |
| 4 | `media` | `ui/rows`、`cards.rs`、`diagrams.rs`、`lost.rs` |
| 5 | `boxes` | 各个框、配置页、引导 |
| 6 | `typing` | `keys.rs`、`compose.rs`、输入框 |
| 7 | `link` | `core` 那 27 处 |
| 8 | `talk` | 最大的一块，最后搬：`sessions`、`switch`、`asking`、`takeback`、`pages` |
| 9 | 画面层改拿块：`ui/` 里的函数照用到的几块收窄参数 | `ui/` |

1. 每一步完了量一次：`App` 还剩几个字段、画面层几个文件拿着整个 `App`，写进 PR。
2. 文件超了 500 行的顺着块拆（`src/app/` 下按块分子目录）。
3. 不顺手改行为、不改界面上的字；搬的时候看到的 bug 记下来，另起提交修。

## 五、风险

- `talk` 那一块和别的块牵得最多（切会话要动视口、框、输入框）：放最后，前面几块先把「两块一起动的事放在 App 上」的写法练熟。
- 搬的时候借用规则会逼着把一些「顺手改别的」的地方拆开，那正是要的；碰到确实要两块同时可变的，写成 `App` 上的方法，先借这块再借那块。
- 不改协议、不改核心，两个头的约定不受影响。
