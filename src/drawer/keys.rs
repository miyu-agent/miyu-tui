//! 抽屉的按键（蓝图「确认和提问的抽屉」第 3、4 条）：选、勾、换题、`Enter`、数字选第几项；编辑「其他」、
//! 补充、理由时按键进字里。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{Answer, Decided, Decision, Drawer, Edit, Item, Outcome, Pasted, Step};

impl Drawer {
    /// 按一下（抽屉开着时按键都先归它）。
    pub fn key(&mut self, key: KeyEvent) -> Step {
        // 按了别的键：上一下 `Esc` 不算了。写「不允许」的理由不算在编辑（光标落上去就在写，第 4 条）。
        let typing = matches!(self.editing, Some(Edit::Other | Edit::Notes));
        if key.code != KeyCode::Esc || typing {
            self.armed = None;
        }
        // `Ctrl+↑` `Ctrl+↓` 滚命令那一块（写着字也行）；`Ctrl+J` `Ctrl+K` 没在写字时也滚，写字时 `Ctrl+J` 照旧换行
        // （2026-10-22 项目主人：「ctrl+JK怎么不能滚」）。
        let typing = self.editing.is_some();
        let scroll = match key.code {
            KeyCode::Up | KeyCode::Down => true,
            KeyCode::Char('j' | 'k') => !typing,
            _ => false,
        };
        if key.modifiers.contains(KeyModifiers::CONTROL) && scroll {
            self.scroll_command(matches!(key.code, KeyCode::Down | KeyCode::Char('j')));
            return Step::Stay;
        }
        if let Some(edit) = self.editing {
            return self.edit_key(edit, key);
        }
        let plain = !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        let count = self.items(self.tab).len();
        // 没在写字：认 vim 的 `h` `j` `k` `l`，同引导（「确认和提问的抽屉」第 4 条，2026-10-11 项目主人）。
        let code = match key.code {
            KeyCode::Char('j') if plain => KeyCode::Down,
            KeyCode::Char('k') if plain => KeyCode::Up,
            KeyCode::Char('h') if plain => KeyCode::Left,
            KeyCode::Char('l') if plain => KeyCode::Right,
            code => code,
        };
        match code {
            KeyCode::Esc => return Step::Escape,
            KeyCode::Up | KeyCode::Down if count > 0 => {
                let cursor = &mut self.cursor[self.tab];
                *cursor = if code == KeyCode::Up {
                    cursor.saturating_sub(1)
                } else {
                    (*cursor + 1).min(count - 1)
                };
            }
            KeyCode::Left | KeyCode::BackTab => self.turn(false),
            KeyCode::Right | KeyCode::Tab => self.turn(true),
            KeyCode::Enter => return self.enter(),
            KeyCode::Backspace => self.unattach(),
            KeyCode::Char(c) if plain => return self.char(c),
            _ => {}
        }
        self.follow_cursor();
        Step::Stay
    }

    /// 光标落在「不允许」上就在写理由，离开就不写了（字留着；2026-10-07 项目主人：原来要先按一下回车才能写）。
    fn follow_cursor(&mut self) {
        let on_deny = self.current() == Some(Item::Decision(Decision::Deny));
        if on_deny && self.editing.is_none() {
            self.editing = Some(Edit::Reason);
        } else if !on_deny && self.editing == Some(Edit::Reason) {
            self.editing = None;
        }
    }

    /// 贴一张图（核心 Q-A）：提问贴给光标所在的这一道（「确认」页不收），确认只有光标在「不允许」上时收；收不了的交回
    /// `false`。能多选的当场重记这一道的回答（只贴了图也算答了）。
    pub fn attach(&mut self, pasted: Pasted) -> bool {
        let takes = if self.is_question() {
            !self.on_review()
        } else {
            self.editing == Some(Edit::Reason)
        };
        if !takes {
            return false;
        }
        self.images[self.tab].push(pasted);
        if self.is_question() && self.multiple(self.tab) {
            self.record();
        }
        true
    }

    /// 删这一页最后一张图（字删空了再按 `Backspace`、没在写字时按 `Backspace`）。
    fn unattach(&mut self) {
        if self.images.get_mut(self.tab).and_then(Vec::pop).is_some()
            && self.is_question()
            && self.multiple(self.tab)
        {
            self.record();
        }
    }

    /// 已经贴了几张（几页加起来）：新贴的照它接着编号。
    pub fn pasted_count(&self) -> usize {
        self.images.iter().map(Vec::len).sum()
    }

    /// 点中第几项：等于移过去按 `Enter`（能多选的是勾上、取消）。
    pub fn click(&mut self, index: usize) -> Step {
        self.editing = None;
        self.armed = None;
        let step = self.pick(index);
        self.follow_cursor();
        step
    }

    /// 编辑时的按键：`Enter` 保存，`Shift+Enter`、`Alt+Enter`、`Ctrl+J` 换行，`Esc` 退出编辑（「输入其他答案」的字留着，补充的清空：
    /// 2026-10-20 项目主人：「esc 之后应该清空补充内容而不是还留着，因为 esc 可能代表着要换选项」）。
    fn edit_key(&mut self, edit: Edit, key: KeyEvent) -> Step {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let newline = key
            .modifiers
            .intersects(KeyModifiers::SHIFT | KeyModifiers::ALT);
        let text = match edit {
            Edit::Notes => &mut self.notes[self.tab],
            Edit::Other | Edit::Reason => &mut self.typed[self.tab],
        };
        // 理由：方向键照样上下选，`Esc` 照旧是两下取消的一下（光标在「不允许」上就在写，没有「退出编辑」）。
        if edit == Edit::Reason {
            match key.code {
                KeyCode::Esc => return Step::Escape,
                KeyCode::Up | KeyCode::Down => {
                    self.editing = None;
                    return self.key(key);
                }
                _ => {}
            }
        }
        match key.code {
            KeyCode::Esc => {
                if edit == Edit::Notes {
                    text.clear();
                }
                self.editing = None;
            }
            KeyCode::Enter if newline => text.push('\n'),
            KeyCode::Char('j') if ctrl => text.push('\n'),
            KeyCode::Enter => return self.save(edit),
            // 字删空了再按删最后一张图（2026-10-22 项目主人照推荐定）；补充的字不管图。
            KeyCode::Backspace if text.is_empty() && edit != Edit::Notes => self.unattach(),
            KeyCode::Backspace => {
                text.pop();
            }
            KeyCode::Char(c) if !ctrl => text.push(c),
            _ => {}
        }
        Step::Stay
    }

    /// 编辑时按 `Enter`：「其他」单选的就答了这一道，多选的算勾上；补充只是存下；理由交「不允许」。
    fn save(&mut self, edit: Edit) -> Step {
        self.editing = None;
        let typed = self.typed[self.tab].trim().to_string();
        match edit {
            Edit::Notes => Step::Stay,
            Edit::Reason => Step::Done(Outcome::Decided(Decided {
                call_id: self.call_id.clone(),
                decision: Decision::Deny,
                reason: (!typed.is_empty()).then_some(typed),
                files: self.images[self.tab].clone(),
            })),
            // 能多选的：写好就算答了（同勾上，见 `toggle`），不往下走。
            Edit::Other if self.multiple(self.tab) => {
                self.record();
                Step::Stay
            }
            // 没写字、也没贴图的不算答；只贴了图的算（核心 Q-A）。
            Edit::Other if typed.is_empty() && self.images[self.tab].is_empty() => Step::Stay,
            Edit::Other if typed.is_empty() => self.settle(Answer::default()),
            Edit::Other => self.settle(Answer {
                text: Some(typed),
                ..Answer::default()
            }),
        }
    }

    /// 没在编辑时打了一个字：能多选的空格勾上；`n` 写补充；数字选第几项。
    fn char(&mut self, c: char) -> Step {
        match c {
            ' ' => self.toggle(),
            'n' if self.is_question() && !self.on_review() => self.editing = Some(Edit::Notes),
            _ => {
                if let Some(d @ 1..=9) = c.to_digit(10) {
                    let step = self.pick(d as usize - 1);
                    self.follow_cursor();
                    return step;
                }
            }
        }
        Step::Stay
    }

    /// 移到第几项、按 `Enter`；能多选的选项是勾上、取消。没有这一项的不动。
    fn pick(&mut self, index: usize) -> Step {
        let items = self.items(self.tab);
        let Some(item) = items.get(index) else {
            return Step::Stay;
        };
        self.cursor[self.tab] = index;
        if self.multiple(self.tab) && matches!(item, Item::Choice(_)) {
            self.toggle();
            return Step::Stay;
        }
        self.enter()
    }

    /// 能多选的：勾上、取消选中的那个选项，当场记成这一道的回答（2026-10-21 项目主人：「多选题勾选之后就应该算是提交了
    /// 内容，而不是必须要回车」，原来勾了不按 `Enter` 就切题目，这一道算没答）。
    fn toggle(&mut self) {
        if !self.multiple(self.tab) {
            return;
        }
        if let Some(Item::Choice(i)) = self.current()
            && let Some(mark) = self.checked[self.tab].get_mut(i)
        {
            *mark = !*mark;
        }
        self.record();
    }

    /// 能多选的这一道：照勾上的、「其他」写的记下回答；一个没勾、也没写的算没答。
    fn record(&mut self) {
        self.answers[self.tab] = self.gathered();
    }

    /// 换到下一页、上一页（连「确认」页），到头绕回来。
    fn turn(&mut self, forward: bool) {
        let n = self.pages() + usize::from(self.has_review());
        self.tab = if forward {
            (self.tab + 1) % n
        } else {
            (self.tab + n - 1) % n
        };
    }

    /// 按 `Enter`。
    fn enter(&mut self) -> Step {
        if self.on_review() {
            return Step::Done(Outcome::Answered(self.answered()));
        }
        match self.current() {
            Some(Item::Other) => {
                self.editing = Some(Edit::Other);
                Step::Stay
            }
            Some(Item::Decision(Decision::Deny)) => {
                self.editing = Some(Edit::Reason);
                Step::Stay
            }
            Some(Item::Decision(decision)) => Step::Done(Outcome::Decided(Decided {
                call_id: self.call_id.clone(),
                decision,
                reason: None,
                files: Vec::new(),
            })),
            // 能多选的：勾的时候已经记下了，`Enter` 只是往下走；一个没勾、也没写其他答案的这一道算没答（2026-10-20 项目主人：
            // 「我明明什么都没选，它怎么自动推荐了」；原来交光标所在的那一项）。
            Some(Item::Choice(_)) if self.multiple(self.tab) => {
                self.record();
                self.move_on()
            }
            Some(Item::Choice(i)) => match self.choose(i) {
                Some(answer) => self.settle(answer),
                None => self.move_on(),
            },
            None => Step::Stay,
        }
    }

    /// 单选的在第 `i` 个选项上按 `Enter` 的回答：就是它。
    fn choose(&self, i: usize) -> Option<Answer> {
        let question = self.question_at(self.tab)?;
        Some(Answer {
            picked: vec![question.options.get(i)?.label.clone()],
            ..Answer::default()
        })
    }

    /// 能多选的这一道的回答：勾上的，接着「其他」写的字；一个没勾、也没写的是 `None`。
    fn gathered(&self) -> Option<Answer> {
        let question = self.question_at(self.tab)?;
        let label = |i: usize| question.options[i].label.clone();
        let typed = self.typed[self.tab].trim();
        let text = (!typed.is_empty()).then(|| typed.to_string());
        let picked: Vec<String> = self.checked[self.tab]
            .iter()
            .enumerate()
            .filter(|(_, on)| **on)
            .map(|(i, _)| label(i))
            .collect();
        let images = !self.images[self.tab].is_empty();
        (!picked.is_empty() || text.is_some() || images).then_some(Answer {
            picked,
            text,
            ..Answer::default()
        })
    }

    /// 记下这一道的回答，跳到下一道没答的；都答了到「确认」页，只有一道题的直接交。
    fn settle(&mut self, answer: Answer) -> Step {
        self.answers[self.tab] = Some(answer);
        self.move_on()
    }

    /// 跳到下一道没答的（这一道答没答都往下）；都答了到「确认」页，只有一道题的直接交。
    fn move_on(&mut self) -> Step {
        if !self.has_review() {
            return Step::Done(Outcome::Answered(self.answered()));
        }
        let n = self.pages();
        self.tab = (1..n)
            .map(|k| (self.tab + k) % n)
            .find(|&p| self.answers[p].is_none())
            .unwrap_or(n);
        Step::Stay
    }
}
