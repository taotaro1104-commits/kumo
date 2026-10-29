//! 表示言語。Kumo 自身の画面（UI・内部ページ・エラーページ）の文言を 5 つの言語で持つ。
//! 文言は [`messages!`] で「キー → [日本語, English, 简体中文, 한국어, Español]」の表にし、
//! `キー.t(lang)` で引く。`{0}` `{1}` は [`fill`] で埋める値の場所。ページの中身（Web の文書）は訳さない。

use serde::{Deserialize, Serialize};

/// 表示言語。順序は各表の列と同じ。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lang {
    #[default]
    Ja,
    En,
    Zh,
    Ko,
    Es,
}

impl Lang {
    pub const ALL: [Lang; 5] = [Lang::Ja, Lang::En, Lang::Zh, Lang::Ko, Lang::Es];

    /// 表の列の番号。
    pub fn index(self) -> usize {
        self as usize
    }

    /// 設定に保存する符号（ISO 639-1）。
    pub fn code(self) -> &'static str {
        match self {
            Self::Ja => "ja",
            Self::En => "en",
            Self::Zh => "zh",
            Self::Ko => "ko",
            Self::Es => "es",
        }
    }

    /// その言語自身での名前（言語の選択肢に出す。どの表示言語でも同じ）。
    pub fn native_name(self) -> &'static str {
        match self {
            Self::Ja => "日本語",
            Self::En => "English",
            Self::Zh => "简体中文",
            Self::Ko => "한국어",
            Self::Es => "Español",
        }
    }

    /// 符号（`ja`・`en-US`・`zh_CN` など。大文字小文字・地域は無視）から選ぶ。
    pub fn from_code(code: &str) -> Option<Self> {
        let primary = code
            .trim()
            .split(['-', '_', '.'])
            .next()?
            .to_ascii_lowercase();
        Self::ALL.into_iter().find(|l| l.code() == primary)
    }
}

/// `{0}` `{1}` … を `args` で置き換える。
pub fn fill(template: &str, args: &[&str]) -> String {
    let mut out = template.to_owned();
    for (i, arg) in args.iter().enumerate() {
        out = out.replace(&format!("{{{i}}}"), arg);
    }
    out
}

/// 文言の表を作る：キーの列挙型と、`t(lang)`・`all()`・`ALL` を持つ。
macro_rules! messages {
    ($(#[$meta:meta])* $name:ident { $($key:ident => [$ja:expr, $en:expr, $zh:expr, $ko:expr, $es:expr],)* }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name { $($key),* }

        impl $name {
            #[allow(dead_code)]
            pub const ALL: &'static [$name] = &[$($name::$key),*];

            /// 5 言語の文言（`Lang::index` の順）。
            pub fn all(self) -> [&'static str; 5] {
                match self { $(Self::$key => [$ja, $en, $zh, $ko, $es]),* }
            }

            pub fn t(self, lang: $crate::i18n::Lang) -> &'static str {
                self.all()[lang.index()]
            }
        }
    };
}

pub mod errors;
pub mod library;
pub mod pages;
pub mod ui;

#[cfg(test)]
mod tests {
    use super::*;

    /// 表の全部が 5 言語とも埋まっていて、`{n}` の場所が言語ごとに揃っている。
    fn check(all: Vec<(String, [&'static str; 5])>) {
        for (key, texts) in all {
            let holes = |s: &str| (0..4).filter(|i| s.contains(&format!("{{{i}}}"))).count();
            for (i, text) in texts.iter().enumerate() {
                assert!(
                    !text.trim().is_empty(),
                    "{key}: {} が空",
                    Lang::ALL[i].code()
                );
                assert_eq!(
                    holes(text),
                    holes(texts[0]),
                    "{key}: {{n}} の数が違う ({i})"
                );
            }
            if texts[0].is_ascii() {
                continue;
            }
            assert_ne!(texts[0], texts[1], "{key}: 英語が日本語のまま");
        }
    }

    #[test]
    fn every_message_is_filled_in_all_languages() {
        let name = |m: &dyn std::fmt::Debug| format!("{m:?}");
        check(ui::Ui::ALL.iter().map(|m| (name(m), m.all())).collect());
        check(pages::Pg::ALL.iter().map(|m| (name(m), m.all())).collect());
        check(errors::Er::ALL.iter().map(|m| (name(m), m.all())).collect());
        check(
            library::Lib::ALL
                .iter()
                .map(|m| (name(m), m.all()))
                .collect(),
        );
    }

    #[test]
    fn languages_round_trip_through_their_codes() {
        for l in Lang::ALL {
            assert_eq!(Lang::from_code(l.code()), Some(l));
        }
        assert_eq!(Lang::from_code("en-US"), Some(Lang::En));
        assert_eq!(Lang::from_code("zh_CN"), Some(Lang::Zh));
        assert_eq!(Lang::from_code("ES"), Some(Lang::Es));
        assert_eq!(Lang::from_code("fr"), None);
        assert_eq!(Lang::default(), Lang::Ja);
    }

    #[test]
    fn fill_replaces_numbered_holes() {
        assert_eq!(fill("{1}/{0}/{1}", &["a", "b"]), "b/a/b");
        assert_eq!(
            pages::Pg::DemotionCount.t(Lang::En),
            "Frozen {0} / discarded {1}"
        );
    }
}
