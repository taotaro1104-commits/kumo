//! 読み込みの負荷レベル：サイトの読み込みでどこまで取得・実行するかを、軽い方から重い方へ 5 段階で選ぶ。
//!
//! | レベル | 名前 | 取得するもの | スクリプトの予算（1 つ / 合計 / 時間） |
//! |---|---|---|---|
//! | 1 | 最軽量 | HTML と埋め込みの CSS だけ | 実行しない |
//! | 2 | 軽量 | + 外部 CSS | 実行しない |
//! | 3 | 標準 | + 画像・スクリプト（既定） | 512KB / 1.5MB / 2 秒 |
//! | 4 | 高負荷 | 同上 | 2 倍 |
//! | 5 | 最大 | 同上、スクリプトの数も 2 倍 | 4 倍 |

use crate::i18n::Lang;
use crate::i18n::pages::Pg;
use kumo_js::ScriptLimits;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LoadLevel {
    Minimal,
    Light,
    #[default]
    Standard,
    Heavy,
    Maximum,
}

impl LoadLevel {
    pub const ALL: [LoadLevel; 5] = [
        Self::Minimal,
        Self::Light,
        Self::Standard,
        Self::Heavy,
        Self::Maximum,
    ];

    /// 1〜5 の番号から。範囲外は `None`。
    pub fn from_number(n: u8) -> Option<Self> {
        Self::ALL.get(usize::from(n).checked_sub(1)?).copied()
    }

    pub fn number(self) -> u8 {
        match self {
            Self::Minimal => 1,
            Self::Light => 2,
            Self::Standard => 3,
            Self::Heavy => 4,
            Self::Maximum => 5,
        }
    }

    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Self::Minimal => Pg::Level1,
            Self::Light => Pg::Level2,
            Self::Standard => Pg::Level3,
            Self::Heavy => Pg::Level4,
            Self::Maximum => Pg::Level5,
        }
        .t(lang)
    }

    pub fn description(self, lang: Lang) -> &'static str {
        match self {
            Self::Minimal => Pg::Level1Hint,
            Self::Light => Pg::Level2Hint,
            Self::Standard => Pg::Level3Hint,
            Self::Heavy => Pg::Level4Hint,
            Self::Maximum => Pg::Level5Hint,
        }
        .t(lang)
    }

    /// 外部スタイルシートを取得するか。
    pub fn stylesheets(self) -> bool {
        self != Self::Minimal
    }

    /// 画像を取得するか。
    pub fn images(self) -> bool {
        self >= Self::Standard
    }

    /// スクリプトを取得して実行するか。
    pub fn javascript(self) -> bool {
        self >= Self::Standard
    }

    /// 倍率（標準を 1 として）。
    fn scale(self) -> usize {
        match self {
            Self::Heavy => 2,
            Self::Maximum => 4,
            _ => 1,
        }
    }

    /// スクリプトの予算。
    pub fn script_limits(self) -> ScriptLimits {
        let base = ScriptLimits::default();
        let k = self.scale();
        ScriptLimits {
            max_bytes: base.max_bytes * k,
            bytes_budget: base.bytes_budget * k,
            time_budget: base.time_budget * k as u32,
        }
    }

    /// 1ページで先に取得する外部スクリプトの数の上限。
    pub fn max_scripts(self) -> usize {
        match self {
            Self::Maximum => 128,
            _ => 64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_round_trip_and_reject_out_of_range() {
        for level in LoadLevel::ALL {
            assert_eq!(LoadLevel::from_number(level.number()), Some(level));
        }
        assert_eq!(LoadLevel::from_number(0), None);
        assert_eq!(LoadLevel::from_number(6), None);
        assert_eq!(LoadLevel::default().number(), 3);
    }

    #[test]
    fn levels_get_heavier() {
        assert!(!LoadLevel::Minimal.stylesheets() && !LoadLevel::Minimal.images());
        assert!(LoadLevel::Light.stylesheets() && !LoadLevel::Light.images());
        assert!(!LoadLevel::Light.javascript());
        assert!(LoadLevel::Standard.images() && LoadLevel::Standard.javascript());
        let std = LoadLevel::Standard.script_limits();
        assert_eq!(std, ScriptLimits::default());
        let (heavy, max) = (
            LoadLevel::Heavy.script_limits(),
            LoadLevel::Maximum.script_limits(),
        );
        assert_eq!(heavy.max_bytes, std.max_bytes * 2);
        assert_eq!(max.time_budget, std.time_budget * 4);
        assert!(LoadLevel::Maximum.max_scripts() > LoadLevel::Heavy.max_scripts());
    }
}
