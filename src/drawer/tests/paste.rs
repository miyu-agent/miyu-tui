//! 抽屉开着时粘贴（2026-11-27）：插在光标处；长的照输入框的规矩收成一块，块上写 `[已粘贴 N 行]`，交出去换回原文；
//! 没在编辑不收。

use super::*;
use crate::input::PasteRule;

fn rule() -> PasteRule {
    PasteRule {
        lines: 2,
        chars: 100,
        label: "[已粘贴 {lines} 行]".into(),
    }
}

#[test]
fn a_paste_goes_in_where_the_cursor_is() {
    let mut d = Drawer::question(None, asked());
    d.paste("不收", &rule());
    assert_eq!(d.typed[0], "", "没在编辑不收");
    d.key(key(KeyCode::Char('3')));
    typed(&mut d, "前后");
    d.key(key(KeyCode::Left));
    d.paste("中\r\n间", &rule());
    assert_eq!(d.typed[0], "前中\n间后", "插在光标处，换行统一成 \\n");
}

#[test]
fn a_long_paste_folds_into_a_block_and_is_handed_in_whole() {
    let mut d = Drawer::question(None, asked());
    d.key(key(KeyCode::Char('3')));
    typed(&mut d, "看这段：");
    d.paste("一\n二\n三", &rule());
    typed(&mut d, " 怎么样");
    assert_eq!(
        d.typed[0], "看这段：[已粘贴 3 行] 怎么样",
        "写字的地方写块上的字"
    );
    let got = answers(d.key(key(KeyCode::Enter)));
    assert_eq!(
        got[0].text.as_deref(),
        Some("看这段：一\n二\n三 怎么样"),
        "交出去换回原文"
    );
    let mut d = Drawer::question(None, asked());
    d.key(key(KeyCode::Char('n')));
    d.paste("一\n二\n三", &rule());
    d.key(key(KeyCode::Backspace));
    assert_eq!(d.notes[0], "", "退格整块删掉");
    d.paste("甲\n乙\n丙", &rule());
    d.key(key(KeyCode::Enter));
    let got = answers(d.key(key(KeyCode::Char('1'))));
    assert_eq!(
        got[0].notes.as_deref(),
        Some("甲\n乙\n丙"),
        "补充里的块也换回原文"
    );
}
