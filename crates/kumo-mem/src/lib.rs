//! タグ付きグローバルアロケータとプロセスメモリ計測。
//!
//! 設計書の方針どおり `unsafe` はこの crate（アロケータ・FFI）に閉じ込める。
//! 確保量はスレッドごとの「現在のサブシステムタグ」に計上する。解放は解放時点の
//! タグに計上されるため、内訳は近似値（ヘッダを付けない＝計測でメモリを増やさない方式）。

use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};

use mimalloc::MiMalloc;

mod pressure;
mod process;

pub use pressure::{Pressure, classify, memory_pressure};
pub use process::{ProcessMemory, process_memory};

/// 計上先のサブシステム。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Subsystem {
    Other = 0,
    Dom,
    Style,
    Layout,
    Text,
    Paint,
    Image,
    Network,
}

impl Subsystem {
    pub const ALL: [Subsystem; 8] = [
        Subsystem::Other,
        Subsystem::Dom,
        Subsystem::Style,
        Subsystem::Layout,
        Subsystem::Text,
        Subsystem::Paint,
        Subsystem::Image,
        Subsystem::Network,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Other => "other",
            Self::Dom => "dom",
            Self::Style => "style",
            Self::Layout => "layout",
            Self::Text => "text",
            Self::Paint => "paint",
            Self::Image => "image",
            Self::Network => "network",
        }
    }
}

const TAGS: usize = Subsystem::ALL.len();

static CURRENT: [AtomicIsize; TAGS] = [const { AtomicIsize::new(0) }; TAGS];
static PEAK: [AtomicUsize; TAGS] = [const { AtomicUsize::new(0) }; TAGS];

thread_local! {
    // const 初期化の Cell<u8> は遅延初期化もデストラクタも持たないため、アロケータ内で安全に読める
    static TAG: Cell<u8> = const { Cell::new(0) };
}

fn current_tag() -> usize {
    TAG.try_with(Cell::get).map_or(0, usize::from)
}

fn record(delta: isize) {
    let tag = current_tag();
    let now = CURRENT[tag].fetch_add(delta, Ordering::Relaxed) + delta;
    if delta > 0 {
        PEAK[tag].fetch_max(now.max(0) as usize, Ordering::Relaxed);
    }
}

/// mimalloc をラップし、サブシステム別の確保量を数えるアロケータ。
pub struct TaggedAllocator;

// SAFETY: 実際の確保・解放はすべて MiMalloc に委譲し、引数をそのまま渡している。
// 追加で行うのはアトミックカウンタの更新とスレッドローカルの読み出しのみで、
// どちらもメモリ確保を行わないため再入しない。
unsafe impl GlobalAlloc for TaggedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: 呼び出し元が GlobalAlloc::alloc の契約を満たしている
        let ptr = unsafe { MiMalloc.alloc(layout) };
        if !ptr.is_null() {
            record(layout.size() as isize);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        record(-(layout.size() as isize));
        // SAFETY: ptr は同じアロケータで同じ layout により確保されたもの
        unsafe { MiMalloc.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: alloc と同じ
        let ptr = unsafe { MiMalloc.alloc_zeroed(layout) };
        if !ptr.is_null() {
            record(layout.size() as isize);
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: 呼び出し元が GlobalAlloc::realloc の契約を満たしている
        let new = unsafe { MiMalloc.realloc(ptr, layout, new_size) };
        if !new.is_null() {
            record(new_size as isize - layout.size() as isize);
        }
        new
    }
}

/// スコープ中の確保を指定サブシステムに計上するガード。
pub struct TagGuard {
    previous: u8,
}

/// `let _g = kumo_mem::scope(Subsystem::Layout);` のように使う。
pub fn scope(subsystem: Subsystem) -> TagGuard {
    let previous = TAG.with(|t| t.replace(subsystem as u8));
    TagGuard { previous }
}

impl Drop for TagGuard {
    fn drop(&mut self) {
        TAG.with(|t| t.set(self.previous));
    }
}

/// サブシステム1つ分の集計。
#[derive(Clone, Copy, Debug)]
pub struct Usage {
    pub subsystem: Subsystem,
    /// 現在の正味確保量（バイト、近似）
    pub current: isize,
    pub peak: usize,
}

/// about:memory 相当の内訳。`TaggedAllocator` を global_allocator にしたときのみ意味を持つ。
pub fn report() -> Vec<Usage> {
    Subsystem::ALL
        .iter()
        .map(|&s| Usage {
            subsystem: s,
            current: CURRENT[s as usize].load(Ordering::Relaxed),
            peak: PEAK[s as usize].load(Ordering::Relaxed),
        })
        .collect()
}

/// 内訳を人が読める表にする。
pub fn format_report(usages: &[Usage], process: Option<ProcessMemory>) -> String {
    let mb = |b: f64| b / (1024.0 * 1024.0);
    let mut out = String::from("subsystem      current(MB)   peak(MB)\n");
    for u in usages {
        out.push_str(&format!(
            "{:<12} {:>12.2} {:>10.2}\n",
            u.subsystem.name(),
            mb(u.current as f64),
            mb(u.peak as f64)
        ));
    }
    let total: isize = usages.iter().map(|u| u.current).sum();
    out.push_str(&format!(
        "{:<12} {:>12.2}\n",
        "heap total",
        mb(total as f64)
    ));
    if let Some(p) = process {
        out.push_str(&format!(
            "{:<20} {:>8.2} MB\n",
            p.label,
            mb(p.primary as f64)
        ));
        for (name, value) in &p.details {
            out.push_str(&format!("  {:<18} {:>8.2} MB\n", name, mb(*value as f64)));
        }
    }
    out
}

/// mimalloc v2（libmimalloc-sys 0.1.49 同梱の c_src/mimalloc/v2/include/mimalloc.h）の
/// `mi_option_e` の値（libmimalloc-sys に定数がないものを補う）。
const MI_OPTION_EAGER_COMMIT: libmimalloc_sys::mi_option_t = 3;
const MI_OPTION_PURGE_DELAY: libmimalloc_sys::mi_option_t = 15;

/// 省メモリ向けにアロケータを設定する。起動直後、最初の確保より前に呼ぶ。
/// セグメントを先行コミットせず、解放されたページは遅延なく OS へ返す。
pub fn configure_for_low_memory() {
    // SAFETY: mi_option_set は整数オプションを書き換えるだけで、いつ呼んでもよい
    unsafe {
        libmimalloc_sys::mi_option_set(MI_OPTION_EAGER_COMMIT, 0);
        libmimalloc_sys::mi_option_set(MI_OPTION_PURGE_DELAY, 0);
    }
    // main より前（Rust ランタイムの初期化時）に最初のセグメント（32MiB）が先行コミット
    // されているので、設定を変えた直後に未使用部分を OS へ返す。これで起動直後の
    // コミット量が約 33MB から約 1MB になる。
    release_free_memory();
}

/// 既定ヒープを切り替え、スコープ終了（panic による巻き戻しを含む）で元に戻す内部ガード。
/// 公開APIはクロージャ形式なので、切り替えは必ず LIFO で入れ子になり、
/// 削除済みのヒープが既定に戻されることはない。
struct RestoreDefault {
    previous: *mut libmimalloc_sys::mi_heap_t,
    /// drop 時に削除する一時ヒープ（`None` なら切り替えのみ）
    owned: Option<*mut libmimalloc_sys::mi_heap_t>,
}

impl Drop for RestoreDefault {
    fn drop(&mut self) {
        // SAFETY: previous はこのガードより外側（先に作られ、後に破棄される）のヒープなので有効。
        // owned はこのガードが作った同スレッドのヒープで、mi_heap_delete は残っているブロックを
        // 解放せずバッキングヒープへ移すため、外部の参照は有効なまま。
        unsafe {
            libmimalloc_sys::mi_heap_set_default(self.previous);
            if let Some(heap) = self.owned {
                libmimalloc_sys::mi_heap_delete(heap);
            }
        }
    }
}

/// 一時ヒープの中で `f` を実行する。`f` 中のこのスレッドの確保はすべて一時ヒープに入り、
/// 終了時にヒープごと削除される（生き残ったブロックは通常ヒープへ移る）。
///
/// レイアウトのように大量の短命オブジェクトを作る処理を囲むと、長命データ（DOM・
/// ディスプレイリスト）と同じページに混ざらず、終了時にページごと OS へ返せる
/// （設計書の「フレーム内一時データはまとめて捨てる」に相当）。
pub fn with_scratch_heap<R>(f: impl FnOnce() -> R) -> R {
    // SAFETY: mi_heap_new は新しいヒープを返し、set_default は現在スレッドの既定を切り替えるだけ
    let _restore = unsafe {
        let heap = libmimalloc_sys::mi_heap_new();
        RestoreDefault {
            previous: libmimalloc_sys::mi_heap_set_default(heap),
            owned: Some(heap),
        }
    };
    f()
}

/// 一時ヒープの中から、長命データを通常（バッキング）ヒープに確保するために使う。
pub fn with_long_lived<R>(f: impl FnOnce() -> R) -> R {
    // SAFETY: バッキングヒープはスレッドの生存中つねに有効
    let _restore = unsafe {
        let backing = libmimalloc_sys::mi_heap_get_backing();
        RestoreDefault {
            previous: libmimalloc_sys::mi_heap_set_default(backing),
            owned: None,
        }
    };
    f()
}

/// アロケータ内部の統計（コミット量・断片化の確認用）を標準エラーへ出す。
pub fn print_allocator_stats() {
    // SAFETY: NULL を渡すと既定の出力先（stderr）を使う
    unsafe { libmimalloc_sys::mi_stats_print_out(None, std::ptr::null_mut()) }
}

/// 未使用ページを OS へ返す（アイドル時・タブ休止時に呼ぶ）。
pub fn release_free_memory() {
    // SAFETY: mi_collect は引数の真偽値だけを受け取り、任意のタイミングで呼べる
    unsafe { libmimalloc_sys::mi_collect(true) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[global_allocator]
    static ALLOC: TaggedAllocator = TaggedAllocator;

    #[test]
    fn allocations_are_attributed_to_scope() {
        let before = report()[Subsystem::Paint as usize].current;
        let data = {
            let _g = scope(Subsystem::Paint);
            vec![0u8; 1 << 20]
        };
        let after = report()[Subsystem::Paint as usize].current;
        assert!(after - before >= 1 << 20, "{before} -> {after}");
        assert_eq!(current_tag(), Subsystem::Other as usize, "ガードで元に戻る");
        drop(data);
        release_free_memory();
        let text = format_report(&report(), process_memory());
        assert!(text.contains("paint"));
    }

    #[test]
    fn scratch_heap_blocks_survive_heap_deletion() {
        let survivor = with_scratch_heap(|| {
            let transient: Vec<u64> = (0..10_000).collect();
            let kept = with_long_lived(|| vec![7u8; 4096]);
            let nested = with_scratch_heap(|| vec![1u16; 100]);
            let migrated: Vec<u32> = (0..1000).collect();
            drop(transient);
            assert_eq!(nested.len(), 100);
            (kept, migrated)
        });
        // 一時ヒープ削除後もブロックは通常ヒープへ移されて有効
        assert!(survivor.0.iter().all(|&b| b == 7));
        assert_eq!(survivor.1.iter().sum::<u32>(), 499_500);
        let mut grown = survivor.1;
        grown.extend(0..1000);
        assert_eq!(grown.len(), 2000);
    }

    #[test]
    fn process_memory_is_available() {
        let p = process_memory().expect("計測できる");
        assert!(p.primary > 0);
    }
}
