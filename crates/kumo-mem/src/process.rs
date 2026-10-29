//! プロセス全体のメモリ使用量（Windows: Private Working Set、Linux: PSS）。

/// OS ごとの代表値と詳細。
#[derive(Clone, Debug)]
pub struct ProcessMemory {
    /// 代表値の名前（例: "private working set" / "pss"）
    pub label: &'static str,
    /// 代表値（バイト）
    pub primary: usize,
    pub details: Vec<(&'static str, usize)>,
}

#[cfg(windows)]
pub fn process_memory() -> Option<ProcessMemory> {
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX2,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    let mut counters = PROCESS_MEMORY_COUNTERS_EX2 {
        cb: size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32,
        ..Default::default()
    };
    // SAFETY: counters は cb で示したサイズの有効な書き込み先。EX2 は先頭が
    // PROCESS_MEMORY_COUNTERS と同じレイアウトなのでポインタキャストは API の想定どおり。
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            (&raw mut counters).cast::<PROCESS_MEMORY_COUNTERS>(),
            counters.cb,
        )
    };
    if ok == 0 {
        // EX2 を知らない Windows では呼び出し自体が失敗するので、従来の構造体で取り直す
        let mut basic = PROCESS_MEMORY_COUNTERS {
            cb: size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            ..Default::default()
        };
        // SAFETY: basic は cb で示したサイズの有効な書き込み先
        let ok = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &raw mut basic, basic.cb) };
        if ok == 0 {
            return None;
        }
        return Some(ProcessMemory {
            label: "private commit",
            primary: basic.PagefileUsage,
            details: vec![("working set", basic.WorkingSetSize)],
        });
    }
    // 古い Windows では EX2 の PrivateWorkingSetSize が 0 のままなので private commit で代替
    let (label, primary) = if counters.PrivateWorkingSetSize > 0 {
        ("private working set", counters.PrivateWorkingSetSize)
    } else {
        ("private commit", counters.PrivateUsage)
    };
    Some(ProcessMemory {
        label,
        primary,
        details: vec![
            ("working set", counters.WorkingSetSize),
            ("peak working set", counters.PeakWorkingSetSize),
            ("private commit", counters.PrivateUsage),
        ],
    })
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn process_memory() -> Option<ProcessMemory> {
    let text = std::fs::read_to_string("/proc/self/smaps_rollup").ok()?;
    let field = |name: &str| {
        text.lines()
            .find(|l| l.starts_with(name))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|kb| kb.parse::<usize>().ok())
            .map(|kb| kb * 1024)
    };
    Some(ProcessMemory {
        label: "pss",
        primary: field("Pss:")?,
        details: vec![("rss", field("Rss:").unwrap_or(0))],
    })
}

#[cfg(not(any(windows, target_os = "linux", target_os = "android")))]
pub fn process_memory() -> Option<ProcessMemory> {
    None
}
