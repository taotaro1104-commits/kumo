//! スクリプトの大きさ・実行時間の予算。

use std::time::Duration;

/// 1つのスクリプトの大きさの上限。巨大なバンドルは純 Rust のエンジンでは実用的な時間・メモリで動かない。
pub const SCRIPT_MAX_BYTES: usize = 512 * 1024;
/// 1ページが実行するスクリプトの合計の大きさの予算（設計書の「JS ヒープ＋バイトコード 10MB」に収める）。
pub const SCRIPT_BYTES_BUDGET: usize = 1536 * 1024;
/// 1ページがスクリプトの実行に使える時間の予算。使い切ったら以降のスクリプトは実行しない。
pub const SCRIPT_TIME_BUDGET: Duration = Duration::from_secs(2);

/// スクリプトの大きさ・実行時間の予算（読み込みの負荷レベルで変わる）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptLimits {
    /// 1つのスクリプトの大きさの上限
    pub max_bytes: usize,
    /// 1ページが実行するスクリプトの合計の大きさの予算
    pub bytes_budget: usize,
    /// 1ページがスクリプトの実行に使える時間の予算
    pub time_budget: Duration,
}

impl Default for ScriptLimits {
    fn default() -> Self {
        Self {
            max_bytes: SCRIPT_MAX_BYTES,
            bytes_budget: SCRIPT_BYTES_BUDGET,
            time_budget: SCRIPT_TIME_BUDGET,
        }
    }
}
