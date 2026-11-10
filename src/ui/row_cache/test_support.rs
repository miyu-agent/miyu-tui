//! 测试里造 [`Rows`]：一块一块给，每块几行、是第几条的。

use std::rc::Rc;

use super::Rows;
use crate::ui::rows::Row;

impl Rows {
    /// 照 `(几行, 第几条)` 一块块拼起来；行都是空的。
    pub fn of_chunks(chunks: &[(usize, Option<usize>)], row: &Row) -> Rows {
        let mut out = Rows::default();
        for &(n, owner) in chunks {
            out.push_owned(Rc::from(vec![row.clone(); n]), owner);
        }
        out
    }

    /// 照 `(几行, 第几条)` 一块块拼起来；行都是空白的。
    pub fn of_blank(chunks: &[(usize, Option<usize>)]) -> Rows {
        let row = Row::new(Vec::new(), 0, 0);
        Rows::of_chunks(chunks, &row)
    }

    /// 照 `(行, 第几条)` 一块块拼起来：行里要带点开的目标时用。
    pub fn of_owned(chunks: Vec<(Vec<Row>, Option<usize>)>) -> Rows {
        let mut out = Rows::default();
        for (rows, owner) in chunks {
            out.push_owned(Rc::from(rows), owner);
        }
        out
    }

    /// 照给的行拼成一块。
    pub fn of_rows(rows: Vec<Row>) -> Rows {
        let mut out = Rows::default();
        out.push_owned(Rc::from(rows), None);
        out
    }
}
