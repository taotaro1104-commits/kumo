//! `var()` の置換。

use std::collections::HashMap;
use std::sync::Arc;

use kumo_dom::LocalName;

/// 置換の再帰上限（循環参照の防止）。
const MAX_DEPTH: u8 = 16;
/// 置換結果の上限（入れ子の倍々展開によるメモリ爆発を防ぐ）。
const MAX_OUTPUT: usize = 64 * 1024;

/// `css` 内の `var(--name[, fallback])` をすべて置換する。未定義かつフォールバックなしなら `None`。
pub fn substitute(css: &str, vars: &HashMap<LocalName, Arc<str>>) -> Option<String> {
    substitute_depth(css, vars, 0)
}

fn substitute_depth(css: &str, vars: &HashMap<LocalName, Arc<str>>, depth: u8) -> Option<String> {
    if depth > MAX_DEPTH {
        return None;
    }
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = find_var(rest) {
        out.push_str(&rest[..start]);
        let args_start = start + 4;
        let close = matching_paren(&rest[args_start..])? + args_start;
        let args = &rest[args_start..close];
        let (name, fallback) = match top_level_comma(args) {
            Some(i) => (args[..i].trim(), Some(&args[i + 1..])),
            None => (args.trim(), None),
        };
        let value = match vars.get(&LocalName::from(name)) {
            Some(v) => substitute_depth(v, vars, depth + 1),
            None => fallback.and_then(|f| substitute_depth(f.trim(), vars, depth + 1)),
        }?;
        out.push_str(&value);
        if out.len() > MAX_OUTPUT {
            return None;
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    Some(out)
}

fn find_var(s: &str) -> Option<usize> {
    s.as_bytes()
        .windows(4)
        .position(|w| w.eq_ignore_ascii_case(b"var("))
}

/// `(` の内側の文字列について、対応する `)` の位置を返す。
fn matching_paren(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(i),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn top_level_comma(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<LocalName, Arc<str>> {
        pairs
            .iter()
            .map(|(k, v)| (LocalName::from(*k), Arc::from(*v)))
            .collect()
    }

    #[test]
    fn substitutes_defined_and_fallback() {
        let v = vars(&[("--a", "red"), ("--b", "var(--a)")]);
        assert_eq!(substitute("var(--a)", &v).as_deref(), Some("red"));
        assert_eq!(
            substitute("1px solid var(--b)", &v).as_deref(),
            Some("1px solid red")
        );
        assert_eq!(
            substitute("var(--x, rgb(1, 2, 3))", &v).as_deref(),
            Some("rgb(1, 2, 3)")
        );
        assert_eq!(substitute("var(--x, var(--a))", &v).as_deref(), Some("red"));
        assert_eq!(substitute("var(--x)", &v), None);
    }

    #[test]
    fn cycles_fail() {
        let v = vars(&[("--a", "var(--b)"), ("--b", "var(--a)")]);
        assert_eq!(substitute("var(--a)", &v), None);
    }
}
