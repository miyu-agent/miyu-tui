//! 配色（图纸 `shellhook.md`「怎么印」第 4 条）：照终端界面的配置项 `tui.theme*`、跟随壁纸找到的、终端会话那位人格的主题色
//! 挑，算法和全屏同一份（`theme::choice::{seeds, pick}`）；配色那一项没人设过的跟随终端（各个头默认跟随人格，shellhook 例外，
//! 2026-10-20 项目主人定）。每次说话现取：换了壁纸、改了配色，下一句就是新的。shell 里不铺
//! 底色（滚动区里铺一块底很怪）。

use serde_json::Value;

use crate::config::Config;
use crate::core::Persona;
use crate::theme::choice::{self, Choice, PersonaSeeds};
use crate::theme::wallpaper::Wallpaper;

/// 配色那一项（`Choice::KEYS` 的第一个）。
const TUI_THEME: &str = "tui.theme";
/// 跟随终端。
const TERMINAL: &str = "terminal";

/// 挑配色要的几样。
pub struct Look {
    choice: Choice,
    /// 自选的主色（`tui.theme_seed`）。
    chosen: Option<u32>,
    /// 终端的底色深不深（明暗自动、跟随终端照它）；问不到的是 `None`。
    dark: Option<bool>,
    wallpaper: Wallpaper,
    /// 终端会话用哪位人格，人格列表：跟随头像照它的主题色、头像的候选。
    persona: Option<String>,
    personas: Vec<Persona>,
    /// 上一次设上的：主色、名字，一样的不再设。
    applied: Option<(Option<u32>, String)>,
    /// 配置项读回来过（读不到的也算：照出厂的）。
    ready: bool,
}

impl Look {
    /// 起来时：照出厂的设置先挑一套，配置项读回来再换。
    pub fn new(dark: Option<bool>) -> Self {
        Self {
            choice: Choice {
                colors: TERMINAL.to_string(),
                ground: false,
                ..Choice::default()
            },
            chosen: None,
            dark,
            wallpaper: Wallpaper::default(),
            persona: None,
            personas: Vec::new(),
            applied: None,
            ready: false,
        }
    }

    /// 要读的配置项：配色那几项、自选的主色。
    pub fn keys() -> Vec<&'static str> {
        let mut keys = Choice::KEYS.to_vec();
        keys.push(choice::SEED_KEY);
        keys
    }

    /// 配置项读回来了：换上；选着跟随壁纸的去找壁纸（在后台找，找到了下一帧换）。
    pub fn configured(&mut self, got: Option<&Value>) {
        self.ready = true;
        let Some(got) = got else { return };
        self.choice.apply(got);
        self.choice.ground = false;
        // 配色没人设过（照默认值）的跟随终端，不照终端界面的默认（2026-10-20 项目主人定：各个头默认跟随人格，shellhook
        // 例外；人设了的照设的）。
        if got["items"][TUI_THEME]["origin"]["layer"]
            .as_str()
            .is_none_or(|layer| layer == "default")
        {
            self.choice.colors = TERMINAL.to_string();
        }
        self.chosen = choice::chosen_seed(got);
        if self.choice.colors == "wallpaper" {
            self.wallpaper.look(self.choice.dark(self.dark), true);
        }
    }

    /// 人格 `id` 的名字（列表到了才有）。
    pub fn persona_name(&self, id: &str) -> Option<String> {
        self.personas
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.clone().unwrap_or_else(|| p.id.clone()))
    }

    /// 现在挑的是哪种配色（`tui.theme` 的值）。
    pub fn colors(&self) -> &str {
        &self.choice.colors
    }

    /// 先照 `colors` 这种画（`/theme` 列表里挪到哪种当场换样子，还不写设置）；下一帧 [`Look::apply`] 换上。
    pub fn preview(&mut self, colors: &str) {
        self.choice.colors = colors.to_string();
        if colors == "wallpaper" {
            self.wallpaper.look(self.choice.dark(self.dark), true);
        }
    }

    /// 配置项读回来过。
    pub fn ready(&self) -> bool {
        self.ready
    }

    /// 终端会话用的人格。
    pub fn persona(&mut self, id: String) {
        self.persona = Some(id);
    }

    /// 人格列表。
    pub fn personas(&mut self, list: Vec<Persona>) {
        self.personas = list;
    }

    /// 每一帧以前看一次：壁纸找好了的收下；照现在的几样挑，变了才设。
    pub fn apply(&mut self, config: &Config) {
        self.wallpaper.collect();
        let persona = self
            .persona
            .as_deref()
            .and_then(|id| self.personas.iter().find(|p| p.id == id));
        let given = persona.map(|p| PersonaSeeds {
            seed: p.seed.as_deref(),
            seeds: p.seeds.as_deref().unwrap_or_default(),
        });
        let (_, seed) = choice::seeds(
            &self.choice.colors,
            self.wallpaper.seeds(),
            given,
            self.chosen,
        );
        let Some(picked) = choice::pick(&self.choice, self.dark, &config.themes, seed) else {
            return;
        };
        let now = (seed, picked.name.clone());
        if self.applied.as_ref() != Some(&now) {
            crate::theme::set(picked.palette);
            self.applied = Some(now);
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Look;

    fn got(value: &str, layer: &str) -> serde_json::Value {
        json!({"items": {"tui.theme": {"value": value, "origin": {"layer": layer}}}})
    }

    #[test]
    fn the_colors_follow_the_terminal_unless_someone_chose_them() {
        // 终端界面默认跟随人格，shellhook 默认跟随终端；人设了的照设的（2026-10-20 项目主人定）。
        let mut look = Look::new(Some(true));
        assert_eq!(look.choice.colors, "terminal", "配置读回来以前");
        look.configured(Some(&got("persona", "default")));
        assert_eq!(look.choice.colors, "terminal", "照默认值的");
        look.configured(Some(&got("tokyonight", "user")));
        assert_eq!(look.choice.colors, "tokyonight", "人设了的");
    }
}
