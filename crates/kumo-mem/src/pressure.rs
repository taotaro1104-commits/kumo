//! OS のメモリ圧迫の検出（設計書：Windows の QueryMemoryResourceNotification、Linux の PSI 相当）。
//! 空き物理メモリの割合から段階を決める。割合の判定は OS に依存しない純粋な関数に分けてある。

/// 圧迫の段階。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pressure {
    #[default]
    None,
    /// 空きが少ない：背景のタブを凍結する
    Moderate,
    /// 空きがほとんどない：背景のタブを破棄する
    Critical,
}

/// 空きが全体のこの割合（%）を下回ったら中程度。
const MODERATE_BELOW_PERCENT: u64 = 15;
/// 同じく危険。
const CRITICAL_BELOW_PERCENT: u64 = 7;

/// 空き `available` / 全体 `total`（同じ単位）から段階を決める。
pub fn classify(available: u64, total: u64) -> Pressure {
    if total == 0 {
        return Pressure::None;
    }
    let percent = available.saturating_mul(100) / total;
    if percent < CRITICAL_BELOW_PERCENT {
        Pressure::Critical
    } else if percent < MODERATE_BELOW_PERCENT {
        Pressure::Moderate
    } else {
        Pressure::None
    }
}

/// 今の OS のメモリ圧迫。調べられなければ `None`（圧迫なし）。
pub fn memory_pressure() -> Pressure {
    physical_memory().map_or(Pressure::None, |(available, total)| {
        classify(available, total)
    })
}

/// (空き, 全体) の物理メモリ（バイト）。
#[cfg(windows)]
fn physical_memory() -> Option<(u64, u64)> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: status は dwLength で示したサイズの有効な書き込み先
    let ok = unsafe { GlobalMemoryStatusEx(&raw mut status) };
    (ok != 0).then_some((status.ullAvailPhys, status.ullTotalPhys))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn physical_memory() -> Option<(u64, u64)> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let field = |name: &str| {
        text.lines()
            .find(|l| l.starts_with(name))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|kb| kb.parse::<u64>().ok())
            .map(|kb| kb * 1024)
    };
    Some((field("MemAvailable:")?, field("MemTotal:")?))
}

#[cfg(not(any(windows, target_os = "linux", target_os = "android")))]
fn physical_memory() -> Option<(u64, u64)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_available_ratio() {
        assert_eq!(classify(50, 100), Pressure::None);
        assert_eq!(classify(15, 100), Pressure::None);
        assert_eq!(classify(14, 100), Pressure::Moderate);
        assert_eq!(classify(7, 100), Pressure::Moderate);
        assert_eq!(classify(6, 100), Pressure::Critical);
        assert_eq!(classify(0, 100), Pressure::Critical);
    }

    #[test]
    fn unknown_total_is_no_pressure() {
        assert_eq!(classify(0, 0), Pressure::None);
    }

    #[test]
    fn levels_are_ordered() {
        assert!(Pressure::Critical > Pressure::Moderate);
        assert!(Pressure::Moderate > Pressure::None);
    }

    #[test]
    fn real_machine_reports_some_level() {
        let _ = memory_pressure(); // 失敗しても None になるだけ（パニックしない）
    }
}
