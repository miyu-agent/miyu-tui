//! 注册机主那一步（蓝图 `tui.md`「第一次打开的引导」第 5a 条，2026-10-24 项目主人定，核心 Y-12 中上 `account.setup`）：
//! 用户名、密码、再输一遍，下面「注册」。只在还没有机主（握手回应 `setup: true`）时有。交之前自己先查一遍（账号编号的
//! 写法、密码长短、两次一样），不合的不发请求。注册成了核心断开这条连接、照机主重起：等重连上、握手不再要注册
//! （[`RegisterStep::registered`]）再往下走。回应和断开挨着到，回应被断开抢先的也一样：只认重连后的握手。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::Move;
use super::asker::{Asker, Waiting};
use super::field::{Edit, Field};
use super::nav::{self, Nav};
use crate::core::Refusal;

/// 光标停的几格。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Slot {
    /// 用户名。
    #[default]
    User,
    /// 密码。
    Password,
    /// 再输一遍。
    Again,
    /// 「注册」。
    Submit,
}

/// 照画的先后。
pub const SLOTS: [Slot; 4] = [Slot::User, Slot::Password, Slot::Again, Slot::Submit];

/// 不合的地方：查出来的，或者核心拒的。照字表 `oobe.register.problems` 写。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// 用户名不合账号编号的写法。
    Username,
    /// 密码不到 8 个字节。
    Short,
    /// 密码全是空白。
    Blank,
    /// 两次不一样。
    Mismatch,
    /// 用户名已被使用（`username_taken`）。
    Taken,
    /// 核心拒了别的：它的原话。
    Said(String),
}

impl Problem {
    /// 写在哪一格下面。
    pub fn slot(&self) -> Slot {
        match self {
            Problem::Short | Problem::Blank => Slot::Password,
            Problem::Mismatch => Slot::Again,
            Problem::Username | Problem::Taken | Problem::Said(_) => Slot::User,
        }
    }
}

/// 密码最短几个字节、最长几个字节（核心 `login.rs` 的 `SHORTEST`、`LONGEST`）。
const PASSWORD_BYTES: std::ops::RangeInclusive<usize> = 8..=1024;
/// 用户名最长几个字（核心账号编号的写法）。
const USERNAME_CHARS: usize = 32;

/// 用户名合不合账号编号的写法（核心 `id.rs` 的 `check_name`）：小写字母开头，只有小写字母、数字、`-`、`_`，最多 32 个字。
/// `admin`、已有的账号、包的编号这些占着的，交给核心说。
pub fn good_username(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name.len() <= USERNAME_CHARS
        && name
            .chars()
            .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '-' | '_'))
}

/// 这一步。
#[derive(Debug)]
pub struct RegisterStep {
    /// 还没有机主：有这一步（握手回应 `setup: true`）。
    pub needed: bool,
    /// 注册成了，在等核心重起、重连上。
    pub done: bool,
    /// 用户名。
    pub user: Field,
    /// 密码。
    pub password: Field,
    /// 再输一遍。
    pub again: Field,
    /// 光标在哪一格。
    pub focus: Slot,
    /// 交了在等回应、等重连。
    pub busy: bool,
    /// 不合的地方。
    pub problem: Option<Problem>,
}

impl Default for RegisterStep {
    fn default() -> Self {
        Self {
            needed: false,
            done: false,
            user: Field::line(),
            password: Field::secret(),
            again: Field::secret(),
            focus: Slot::User,
            busy: false,
            problem: None,
        }
    }
}

impl RegisterStep {
    /// 还要走这一步：要注册、还没注册成。
    pub fn pending(&self) -> bool {
        self.needed && !self.done
    }

    /// 正在编辑的那一格。
    fn editing(&mut self) -> Option<&mut Field> {
        [&mut self.user, &mut self.password, &mut self.again]
            .into_iter()
            .find(|f| f.editing())
    }

    /// 光标那一格（「注册」那一行没有）。
    fn field(&mut self, slot: Slot) -> Option<&mut Field> {
        match slot {
            Slot::User => Some(&mut self.user),
            Slot::Password => Some(&mut self.password),
            Slot::Again => Some(&mut self.again),
            Slot::Submit => None,
        }
    }

    /// 有一格在编辑。
    pub fn typing(&self) -> bool {
        self.user.editing() || self.password.editing() || self.again.editing()
    }

    /// 按了一个键。
    pub fn key(&mut self, key: KeyEvent, asker: &mut Asker) -> Move {
        if self.busy {
            return Move::Stay;
        }
        // 格子 `Enter` 才开始编辑，编辑时 `Enter` 写好、`Esc` 不改（第 7 条）。
        if let Some(field) = self.editing() {
            if field.edit(key) == Edit::Done {
                self.problem = None;
            }
            return Move::Stay;
        }
        let at = SLOTS.iter().position(|s| *s == self.focus).unwrap_or(0);
        let moved = nav::plain(key);
        match key.code {
            KeyCode::Esc => return Move::Back,
            _ if moved == Some(Nav::Up) => self.focus = SLOTS[at.saturating_sub(1)],
            _ if moved == Some(Nav::Down) => self.focus = SLOTS[(at + 1).min(SLOTS.len() - 1)],
            KeyCode::Enter => match self.field(self.focus) {
                Some(field) => field.begin(),
                None => self.submit(asker),
            },
            _ => {}
        }
        Move::Stay
    }

    /// 先自己查一遍：不合的写在那一格下面、光标挪过去；都合的交 `account.setup`。
    fn submit(&mut self, asker: &mut Asker) {
        let user = self.user.trimmed();
        let password = self.password.text().to_string();
        let problem = if !good_username(&user) {
            Some(Problem::Username)
        } else if password.trim().is_empty() {
            Some(Problem::Blank)
        } else if !PASSWORD_BYTES.contains(&password.len()) {
            Some(Problem::Short)
        } else if self.again.text() != password {
            Some(Problem::Mismatch)
        } else {
            None
        };
        if let Some(problem) = problem {
            self.focus = problem.slot();
            self.problem = Some(problem);
            return;
        }
        self.problem = None;
        self.busy = true;
        asker.ask(
            "account.setup",
            json!({"username": user, "password": password}),
            Waiting::Register,
        );
    }

    /// 回应：成了的等重连（核心答完就断这条连接）；拒了的写在用户名那一格下面、表单留着（断开了没等到回应的照被拒办，
    /// 重连上握手说有机主了的照样往下走）。
    pub fn answer(&mut self, result: Result<Value, Refusal>) -> Move {
        match result {
            Ok(_) => self.done = true,
            Err(refusal) => {
                self.busy = false;
                let problem = match refusal.reason.as_deref() {
                    Some("username_taken") => Problem::Taken,
                    _ => Problem::Said(crate::settings::pages::target::said(&refusal)),
                };
                self.focus = problem.slot();
                self.problem = Some(problem);
            }
        }
        Move::Stay
    }

    /// 握手说已经有机主了：原来要注册的这一步不用再走（交回 `true`）。不管是这里注册成的，还是别处（网页）注册的。
    pub fn registered(&mut self) -> bool {
        if !self.needed || self.done && !self.busy {
            return false;
        }
        self.done = true;
        self.busy = false;
        self.problem = None;
        // 密码不留在内存里。
        self.password = Field::secret();
        self.again = Field::secret();
        true
    }

    /// 粘贴：光标那一格在编辑的进那一格；没在编辑的先开始编辑。
    pub fn paste(&mut self, text: &str) {
        if self.busy {
            return;
        }
        if let Some(field) = self.editing() {
            field.paste(text);
            return;
        }
        if let Some(field) = self.field(self.focus) {
            field.begin();
            field.paste(text);
        }
    }
}
