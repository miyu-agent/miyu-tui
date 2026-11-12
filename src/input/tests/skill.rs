//! 选定的技能当一整块（2026-11-12 项目主人：「$导入技能 回车后需要做成类占位符，Ctrl+W，左右方向移动光标，都应该视为一个整体」）：
//! 左右键一下跳过、Ctrl+W 和退格一下删掉，发出去还是 `$名字`，不当粘贴块进正文。

use super::typed;

#[test]
fn a_picked_skill_moves_and_deletes_as_one_piece() {
    let mut e = typed("用 ");
    e.insert_skill("导入技能");
    assert_eq!(e.text(), "用 $导入技能 ");
    let block = ("用 ".len(), "用 $导入技能".len());
    assert_eq!(e.blocks(), [block]);
    // 左键：先过空格，再一下跳到块头；右键一下跳到块尾。
    e.left(false);
    e.left(false);
    assert_eq!(e.cursor(), block.0);
    e.right(false);
    assert_eq!(e.cursor(), block.1);
    // 退格一下删掉整块。
    e.backspace();
    assert_eq!(e.text(), "用  ");
    assert!(e.blocks().is_empty());
    // Ctrl+W 也是。
    let mut e = typed("看 ");
    e.insert_skill("pdf");
    e.left(false);
    e.delete_word();
    assert_eq!(e.text(), "看  ");
    assert!(e.blocks().is_empty());
}

#[test]
fn a_picked_skill_is_sent_as_its_name_and_not_drawn_as_a_paste_chip() {
    let mut e = typed("");
    e.insert_skill("pdf");
    e.insert("看一下");
    let draft = e.draft();
    assert_eq!(draft.expand(), "$pdf 看一下");
    let b = &draft.blocks[0];
    assert!(b.is_skill(&draft.text[b.start..b.end]));
}
