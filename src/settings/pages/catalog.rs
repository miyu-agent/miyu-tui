//! 供应商页按 `a` 先弹目录（蓝图「配置页」第 11a 条，2026-10-26 项目主人：「a 新建时先弹目录」）：开窗问一次
//! `provider.catalog {"featured": true}`，列常用的几家，接着「更多供应商…」（换成全目录、本地筛）和「自定义…」（空的编辑窗）。
//! 选了一家开编辑窗，编号照目录的编号起（和引导一样改成合写法的、和已有的错开），存的时候不合写法的另写 `catalog`。
//! 按键同选模型的窗：`j` `k` 挪（跳过接不上的），`/` 筛，`Enter` 选，`Esc` 先清筛的再关。画在 `ui/settings/catalog.rs`。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::super::popup::Popup;
use super::super::{Settings, Waiting, forms};

/// 拿全目录时的 `limit`（核心不写时只给 50 家；同 `oobe.json` 的 `catalog_limit`）。
const ALL: u64 = 10_000;

/// 目录里的一家。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// 目录里的编号。
    pub id: String,
    /// 名字，照核心给的。
    pub name: String,
    /// 现在接得上（有驱动、有地址）；接不上的暗着、选不了。
    pub supported: bool,
}

/// 窗里的一行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row<'a> {
    /// 一家。
    Provider(&'a Entry),
    /// 「更多供应商…」：换成全目录。
    More,
    /// 「自定义…」：空的编辑窗。
    Custom,
}

impl Row<'_> {
    /// 光标停得上。
    pub fn stops(&self) -> bool {
        match self {
            Row::Provider(p) => p.supported,
            Row::More | Row::Custom => true,
        }
    }
}

/// 目录窗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogPick {
    /// 目录里的几家；回来以前是 `None`。
    pub providers: Option<Vec<Entry>>,
    /// 列的是全目录（按过「更多供应商…」）。
    pub all: bool,
    /// 筛的字。
    pub search: String,
    /// 在打筛的字。
    pub typing: bool,
    /// 选中第几行。
    pub sel: usize,
}

impl CatalogPick {
    fn new() -> Self {
        Self {
            providers: None,
            all: false,
            search: String::new(),
            typing: false,
            sel: 0,
        }
    }

    /// 一行行：对得上筛的几家（全目录照名字排），常用的那一份后面接「更多供应商…」，最后「自定义…」。
    pub fn rows(&self) -> Vec<Row<'_>> {
        let all = self.providers.iter().flatten().collect();
        let found = crate::fuzzy::rank(&self.search, all, |p| vec![p.name.as_str(), p.id.as_str()]);
        let mut rows: Vec<Row> = found.into_iter().map(Row::Provider).collect();
        if !self.all {
            rows.push(Row::More);
        }
        rows.push(Row::Custom);
        rows
    }

    /// 光标停到第一行停得上的。
    fn top(&mut self) {
        self.sel = self.rows().iter().position(Row::stops).unwrap_or(0);
    }

    /// 往下（`down`）、往上挪到下一行停得上的；没有的停在原地。
    fn step(&mut self, down: bool) {
        let rows = self.rows();
        let next = if down {
            (self.sel + 1..rows.len()).find(|&i| rows[i].stops())
        } else {
            (0..self.sel).rev().find(|&i| rows[i].stops())
        };
        self.sel = next.unwrap_or(self.sel);
    }

    /// 筛的字变了。
    fn refilter(&mut self) {
        self.top();
    }
}

/// `provider.catalog` 回来的几家；全目录照名字排。
fn entries(catalog: &Value, all: bool) -> Vec<Entry> {
    let text = |v: &Value| v.as_str().map(str::to_string);
    let mut out: Vec<Entry> = catalog["providers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let id = text(&p["id"])?;
            Some(Entry {
                name: text(&p["name"]).unwrap_or_else(|| id.clone()),
                supported: p["supported"].as_bool().unwrap_or(false),
                id,
            })
        })
        .collect();
    if all {
        out.sort_by_key(|p| p.name.to_lowercase());
    }
    out
}

impl Settings {
    /// `a`（供应商页）：开目录窗。常用的几家开配置页时读好了的当场列出来；还没读到的（刚开、读得慢）问一次。
    pub(in crate::settings) fn open_catalog(&mut self) {
        let mut pick = CatalogPick::new();
        match &self.more.featured {
            Some(featured) => {
                pick.providers = Some(featured.clone());
                pick.top();
            }
            None => self.ask(
                "provider.catalog",
                json!({"featured": true}),
                Waiting::Catalog(false),
            ),
        }
        self.popup = Some(Popup::Catalog(pick));
    }

    /// 目录回来了：常用的那一份记下；窗还开着、列的还是这一份才换上；没拿到的当空的（「更多」「自定义」照样能选），
    /// 也不记，下次开窗再问。
    pub(in crate::settings) fn catalog_answer(&mut self, all: bool, result: Option<Value>) {
        if !all {
            self.more.featured_asked = false;
            if let Some(got) = &result {
                self.more.featured = Some(entries(got, false));
            }
        }
        if let Some(Popup::Catalog(pick)) = &mut self.popup
            && pick.all == all
        {
            pick.providers = Some(entries(&result.unwrap_or_default(), all));
            pick.top();
        }
    }

    /// 目录窗里按了一个键。
    pub(in crate::settings) fn catalog_key(&mut self, mut pick: CatalogPick, key: KeyEvent) {
        if pick.typing {
            match key.code {
                KeyCode::Enter => pick.typing = false,
                KeyCode::Esc => {
                    pick.typing = false;
                    pick.search.clear();
                    pick.refilter();
                }
                KeyCode::Backspace => {
                    pick.search.pop();
                    pick.refilter();
                }
                KeyCode::Char(c) => {
                    pick.search.push(c);
                    pick.refilter();
                }
                KeyCode::Down => pick.step(true),
                KeyCode::Up => pick.step(false),
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => pick.step(true),
                KeyCode::Char('k') | KeyCode::Up => pick.step(false),
                KeyCode::Char('/') => {
                    pick.typing = true;
                    pick.search.clear();
                    pick.refilter();
                }
                KeyCode::Enter => return self.catalog_pick(pick),
                KeyCode::Esc if !pick.search.is_empty() => {
                    pick.search.clear();
                    pick.refilter();
                }
                KeyCode::Esc => return,
                _ => {}
            }
        }
        self.popup = Some(Popup::Catalog(pick));
    }

    /// 粘贴进筛的字。
    pub(in crate::settings) fn catalog_paste(pick: &mut CatalogPick, text: &str) {
        if pick.typing {
            pick.search.push_str(text);
            pick.refilter();
        }
    }

    /// 选了选中的那一行。
    fn catalog_pick(&mut self, mut pick: CatalogPick) {
        let chosen = match pick.rows().get(pick.sel) {
            Some(Row::Provider(p)) if p.supported => Some((p.id.clone(), p.name.clone())),
            Some(Row::Provider(_)) | None => {
                self.popup = Some(Popup::Catalog(pick));
                return;
            }
            Some(Row::More) => {
                pick.all = true;
                pick.providers = None;
                pick.search.clear();
                pick.sel = 0;
                self.popup = Some(Popup::Catalog(pick));
                self.ask(
                    "provider.catalog",
                    json!({"limit": ALL}),
                    Waiting::Catalog(true),
                );
                return;
            }
            Some(Row::Custom) => None,
        };
        let Some(data) = &self.data else {
            return;
        };
        let form = match chosen {
            // 目录里有的：只弹填 key 的窗（2026-10-29 项目主人，照 opencode：用户不知道去哪填 key），地址、名字照目录；回车存、
            // 存完照旧取模型列表。要改地址这些的，存了以后在供应商栏 `Enter` 开整个窗。
            Some((catalog, name)) => {
                let existing: Vec<String> =
                    self.view.providers.iter().map(|p| p.id.clone()).collect();
                let id = forms::unique(forms::path_name(&catalog), &existing);
                forms::listed(&self.view, data, &self.draft, &id, &catalog).key_only(name)
            }
            None => forms::provider(&self.view, data, &self.draft, None),
        };
        self.popup = Some(Popup::Form(form));
    }
}
