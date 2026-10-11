//! 配置页人格的窗里的头像（蓝图 `tui.md`「配置页」第 36 条，2026-10-10 加，照「第一次打开的引导」第 21 条）：回车开编辑窗
//! 填图片路径；填了的先 `blob.put` 传上去，再 `persona.set {persona, avatar: {blob, expect}}`；空着回车、原来有头像的去掉
//! （`avatar: {unset, expect}`）。`expect` 是 `persona.get` 给的头像版本，别处先换过的核心拒（照原话写在状态行）。

use serde_json::{Value, json};

use super::choose::{LineEdit, PersonaView};
use super::target::{Target, said};
use crate::core::Refusal;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts, Tone, Waiting};

impl Settings {
    /// 头像那一行回车：开编辑窗填路径（空着的，填了换、空着回车去掉）。
    pub(in crate::settings) fn edit_avatar(&mut self, view: PersonaView, texts: &Texts) {
        let title = &texts.more.persona_edit.avatar_path;
        let edit = LineEdit::open("persona.avatar", title, false, &Value::Null);
        self.popup = Some(Popup::Line(edit.to(Target::Avatar(Box::new(view)))));
    }

    /// 编辑窗写好了：填了路径的先传，空着的去掉原来的头像；不是头像的交回 `false`。
    pub(in crate::settings) fn avatar_write(&mut self, target: Target, value: &Value) -> bool {
        let Target::Avatar(view) = target else {
            return false;
        };
        let version = view.detail.as_ref().and_then(|d| d.avatar.clone());
        let text = value.as_str().unwrap_or_default().trim();
        let text = text.trim_matches(['\'', '"']);
        if text.is_empty() {
            if version.is_some() {
                let body = json!({"avatar": {"unset": true, "expect": version}});
                self.set_persona(&view.id, body, false);
            }
        } else {
            let cwd = std::env::current_dir().unwrap_or_default();
            let path = crate::local::path(text, std::env::home_dir().as_deref(), &cwd);
            // 超了核心的上限、核心不认的格式，发出去以前在后台先缩小（「配置页」第 36 条，`app/avatars.rs`）。
            let params = json!({"path": path.display().to_string()});
            self.ask(
                "blob.put",
                params,
                Waiting::AvatarPut(view.id.clone(), version),
            );
        }
        self.popup = Some(Popup::Persona(*view));
        true
    }

    /// 头像传上去了：设到人格上；传不上的照核心的原话写在状态行。
    pub(in crate::settings) fn avatar_put(
        &mut self,
        (persona, version): (String, Option<String>),
        result: Result<Value, Refusal>,
    ) {
        crate::avatars::shrink::done();
        match result {
            Ok(got) => {
                let body = json!({"avatar": {"blob": got["blob"], "expect": version}});
                self.set_persona(&persona, body, false);
            }
            Err(refusal) => self.say(said(&refusal), Tone::Bad),
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use serde_json::json;

    use crate::config::Config;
    use crate::settings::Settings;
    use crate::settings::pages::choose::PersonaView;
    use crate::settings::pages::personas::Detail;
    use crate::settings::popup::Popup;

    fn press(page: &mut Settings, code: KeyCode) {
        let texts = Config::builtin().unwrap().text.settings;
        page.key(KeyEvent::new(code, KeyModifiers::NONE), &texts);
    }

    /// 人格的窗开着，光标在头像那一行（名字下面）；原来有头像，版本 `v9`。
    fn opened() -> Settings {
        let detail = Detail {
            id: "mine".into(),
            name: Some("小助手".into()),
            avatar: Some("v9".into()),
            ..Detail::default()
        };
        Settings {
            popup: Some(Popup::Persona(PersonaView {
                id: "mine".into(),
                detail: Some(detail),
                error: None,
                cursor: 1,
            })),
            ..Settings::default()
        }
    }

    #[test]
    fn a_typed_path_is_uploaded_then_set_with_the_version_it_replaces() {
        // 「配置页」第 36 条（2026-10-10 项目主人：创建人格时给一个头像文件路径的选项，配置页同样能改）。
        let mut page = opened();
        press(&mut page, KeyCode::Enter);
        assert!(matches!(page.popup, Some(Popup::Line(_))), "开编辑窗");
        // 照这台机器的写法拼一个绝对路径（`/tmp/...` 在 Windows 上会被补上盘符）；文件不在，原样交。
        let typed = std::env::temp_dir().join("a.png").display().to_string();
        for c in typed.chars() {
            press(&mut page, KeyCode::Char(c));
        }
        press(&mut page, KeyCode::Enter);
        let asks = page.take_asks();
        assert_eq!(asks[0].1, "blob.put");
        assert_eq!(asks[0].2, json!({"path": typed}));
        let texts = Config::builtin().unwrap().text.settings;
        page.answer(asks[0].0, Ok(json!({"blob": "sha256:cd"})), &texts);
        let asks = page.take_asks();
        assert_eq!(asks[0].1, "persona.set");
        assert_eq!(
            asks[0].2,
            json!({"persona": "mine", "avatar": {"blob": "sha256:cd", "expect": "v9"}})
        );
    }

    #[test]
    fn a_picture_path_is_handed_over_as_is_and_shrunk_when_sent() {
        // 2026-10-11 项目主人：头像超过 1 MiB 或 1024 像素就拒「不太合理」，终端先缩小再传。缩小挪到发出去的那一刻、在后台
        // 做（review D2：原来在界面线程里，大照片卡界面，`app/avatars.rs`），配置页交的是原来的路径；缩成什么样由
        // `avatars/shrink` 的测试守着。
        let mut page = opened();
        press(&mut page, KeyCode::Enter);
        for c in "/tmp/photo.png".chars() {
            press(&mut page, KeyCode::Char(c));
        }
        press(&mut page, KeyCode::Enter);
        let asks = page.take_asks();
        assert_eq!(asks[0].1, "blob.put");
        assert_eq!(asks[0].2["path"], "/tmp/photo.png");
    }

    #[test]
    fn an_empty_path_takes_the_avatar_away() {
        let mut page = opened();
        press(&mut page, KeyCode::Enter);
        press(&mut page, KeyCode::Enter);
        let asks = page.take_asks();
        assert_eq!(
            asks[0].2,
            json!({"persona": "mine", "avatar": {"unset": true, "expect": "v9"}})
        );
    }
}
