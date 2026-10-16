//! 引导挑过的主色，关引导以后接着拿住（蓝图「第一次打开的引导」第 23a 条，2026-10-16 项目主人报：从完成页进新会话还是闪一下）。
//!
//! 引导「主题」那一步挑的主色写在人格上（`persona.set` 的 `seed`），终端手里的人格列表要重读回来才是新的。引导一关，配色就从
//! 「引导里挑的」换成「人格列表里那一位的」：列表还没回来的那几帧照旧的主色算，整屏画成旧配色，回来了再换回来，头像跟着重做、
//! 重传两次。所以关引导时挑过主色的先拿住它，人格列表里那一位的主色变成它（或者不再跟随头像）才放开。

/// 拿住的主色。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SeedHold(Option<u32>);

impl SeedHold {
    /// 引导关了：关以前挑过主色的（`picked`）拿住它。
    pub fn closed(&mut self, picked: Option<u32>) {
        if picked.is_some() {
            self.0 = picked;
        }
    }

    /// 这一帧用哪个主色：引导开着时照它挑的（`picked`）；拿住的在候选里的照它，人格列表里的（`listed`）已经是它了就放开；
    /// 都没有的照人格列表里的。`following` 是还跟随着头像：不跟随了一并放开。
    pub fn seed(
        &mut self,
        (picked, listed): (Option<u32>, Option<u32>),
        seeds: &[u32],
        following: bool,
    ) -> Option<u32> {
        if !following || (self.0.is_some() && listed == self.0) {
            self.0 = None;
        }
        let held = self.0.filter(|s| seeds.contains(s));
        picked.or(held).or(listed)
    }
}

#[cfg(test)]
mod tests {
    use super::SeedHold;

    #[test]
    fn a_seed_picked_in_the_guide_holds_until_the_persona_list_has_it() {
        let seeds = [0x11, 0x22];
        let mut hold = SeedHold::default();
        // 引导开着：照挑的。
        assert_eq!(
            hold.seed((Some(0x22), Some(0x11)), &seeds, true),
            Some(0x22)
        );
        hold.closed(Some(0x22));
        // 关了，人格列表还是旧的：照拿住的，不闪回旧的。
        assert_eq!(hold.seed((None, Some(0x11)), &seeds, true), Some(0x22));
        // 列表回来了，那一位的主色是它：放开，以后照列表的。
        assert_eq!(hold.seed((None, Some(0x22)), &seeds, true), Some(0x22));
        assert_eq!(
            hold.seed((None, Some(0x11)), &seeds, true),
            Some(0x11),
            "放开了"
        );
    }

    #[test]
    fn nothing_is_held_without_a_pick_or_when_not_following() {
        let seeds = [0x11, 0x22];
        let mut hold = SeedHold::default();
        hold.closed(None);
        assert_eq!(
            hold.seed((None, Some(0x11)), &seeds, true),
            Some(0x11),
            "没挑过的照列表"
        );
        hold.closed(Some(0x22));
        assert_eq!(
            hold.seed((None, None), &seeds, false),
            None,
            "不跟随头像了放开"
        );
        assert_eq!(hold.seed((None, Some(0x11)), &seeds, true), Some(0x11));
        hold.closed(Some(0x33));
        assert_eq!(
            hold.seed((None, Some(0x11)), &seeds, true),
            Some(0x11),
            "不在候选里的不用"
        );
    }
}
