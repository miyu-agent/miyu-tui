//! 软件包列表（`package.list`）：连上、工具目录变了时要。每个包清单写的图标记下，时间线上包的工具照它画（蓝图「图标」）；
//! 终端这个软件包（核心 9-1 上，蓝图「输入历史列表」第 8 条）照 `tui` 那一项的 `state` 把输入历史放进包的状态目录（老
//! 位置有的先搬过去），读进来，和连上以前打的合在一起。只放一次。

use super::App;
use crate::core::{Command, PackageEntry};
use crate::input::{Saved, place};

/// 终端这个包的编号（`resources/packages/tui/package.toml`）。
const PACKAGE: &str = "tui";

impl App {
    /// 包的名字交给开着的配置页、引导。
    pub(super) fn share_package_names(&mut self) {
        let names = &self.link.package_names;
        if let Some(page) = self.boxes.settings.as_mut() {
            page.package_names.clone_from(names);
        }
        if let Some(oobe) = self.boxes.oobe.as_mut() {
            oobe.preset.package_names.clone_from(names);
        }
    }

    /// 连上了、工具目录变了：去要软件包。
    pub(super) fn ask_packages(&mut self) {
        self.link.core.send(Command::ListPackages);
    }

    /// 软件包回来了：记下每个包的图标，排好的行里有旧的图标，扔掉重排；照 `tui` 的状态目录放输入历史（只放一次），没有的
    /// （旧核心、没装清单）照旧放老位置。
    pub(super) fn packages_arrived(&mut self, packages: &[PackageEntry]) {
        let icons = packages
            .iter()
            .filter_map(|p| Some((p.id.clone(), p.icon.clone()?)))
            .collect();
        if self.link.human.icons() != &icons {
            self.link.human.set_icons(icons);
            *self.media.row_cache.borrow_mut() = Default::default();
        }
        // 包的名字、组：预设的功能照组、包分组时写组名（`features.rs`）；开着的配置页、引导跟着换。
        self.link.package_names = crate::features::Packages {
            names: packages
                .iter()
                .filter_map(|p| Some((p.id.clone(), p.name.clone()?)))
                .collect(),
            groups: packages
                .iter()
                .filter_map(|p| Some((p.id.clone(), p.group.clone()?)))
                .collect(),
        };
        self.share_package_names();
        if self.typing.history_placed {
            return;
        }
        self.typing.history_placed = true;
        let state = packages
            .iter()
            .find(|p| p.id == PACKAGE)
            .and_then(|p| p.state.as_deref());
        let path = place(state, self.typing.old_history.take()).filter(|_| !cfg!(test));
        let keep = self.config.layout.history_keep;
        self.typing.input.keep_history(Saved::at(path), keep);
    }
}
