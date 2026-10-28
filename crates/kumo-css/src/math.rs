//! `min()` / `max()` / `clamp()` / `vmin` / `vmax` の式。`Length` は `Copy` の線形結合なので、
//! 式そのものは表に登録して番号（`Length::expr`）で参照する。引数に `em` や `vw` が入ると値は適用時
//! （フォントサイズ・表示領域が分かったとき）まで決まらないため、比較はそこで行う。

use std::sync::{Arc, Mutex};

use crate::values::Length;

#[derive(Clone, Debug, PartialEq)]
pub enum MathExpr {
    Min(Vec<Length>),
    Max(Vec<Length>),
    /// 下限・中央・上限
    Clamp(Length, Length, Length),
}

static TABLE: Mutex<Vec<Arc<MathExpr>>> = Mutex::new(Vec::new());

/// 式を登録して番号（1 から）を返す。同じ式は同じ番号。
pub fn intern(expr: MathExpr) -> u32 {
    let Ok(mut table) = TABLE.lock() else {
        return 0;
    };
    if let Some(at) = table.iter().position(|e| **e == expr) {
        return at as u32 + 1;
    }
    table.push(Arc::new(expr));
    table.len() as u32
}

/// 番号から式を引く。
pub fn lookup(id: u32) -> Option<Arc<MathExpr>> {
    let index = usize::try_from(id.checked_sub(1)?).ok()?;
    TABLE.lock().ok()?.get(index).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_expressions_share_an_id() {
        let a = intern(MathExpr::Min(vec![Length::px(1.0), Length::px(2.0)]));
        let b = intern(MathExpr::Min(vec![Length::px(1.0), Length::px(2.0)]));
        let c = intern(MathExpr::Max(vec![Length::px(1.0), Length::px(2.0)]));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(
            *lookup(a).unwrap(),
            MathExpr::Min(vec![Length::px(1.0), Length::px(2.0)])
        );
        assert!(lookup(0).is_none());
    }
}
