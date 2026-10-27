//! ファイルを使った共有メモリ。作成側（[`SharedMemory`]、読み書き）がファイルを作って大きさを決め、
//! 相手はパスで読み取り専用に開く（[`SharedView`]）。
//! 中身への同時アクセスはメッセージの受け渡しで直列化する（片方が書き終えて通知してから、もう片方が読む）。
//!
//! - 名前には推測できない値を含め、既存のファイルは開かない（他人が置いたファイルやリンクを壊さない）。
//! - 相手から届いたパスは、Kumo の共有メモリのファイル（一時ディレクトリの `kumo-*.shm`）しか開かない。
//! - Windows では「閉じたら消す」で作るので、プロセスが異常終了しても全員が閉じた時点で OS が消す。

use std::fs::{File, OpenOptions};
use std::hash::{BuildHasher, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use memmap2::{Mmap, MmapMut};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// 推測されにくい 64 ビット値（暗号用途ではない。名前の先取りを防ぐため）。
pub fn random_token() -> u64 {
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    hasher.write_u128(nanos);
    hasher.write_u32(std::process::id());
    hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    hasher.finish()
}

/// 共有メモリ用ファイルの新しいパスを作る（プロセス内で重ならず、外から推測しにくい）。
pub fn unique_path(prefix: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "kumo-{prefix}-{}-{n}-{:016x}.shm",
        std::process::id(),
        random_token()
    ))
}

/// Kumo の共有メモリのファイルか（一時ディレクトリ直下の `kumo-*.shm`）。相手から届いたパスの検査に使う。
pub fn is_shared_path(path: &Path) -> bool {
    let name_ok = path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("kumo-") && n.ends_with(".shm"));
    name_ok && path.parent() == Some(std::env::temp_dir().as_path())
}

fn check_path(path: &Path) -> io::Result<()> {
    if is_shared_path(path) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Kumo の共有メモリではないパス",
        ))
    }
}

/// 作成側の共有メモリ（読み書きできる）。
pub struct SharedMemory {
    map: MmapMut,
    path: PathBuf,
    /// Windows では閉じたときに消えるよう作った手元のハンドル
    _file: File,
}

impl SharedMemory {
    /// `path` に `len` バイトの共有メモリを作る。既にファイルがあれば失敗する。
    pub fn create(path: &Path, len: usize) -> io::Result<Self> {
        check_path(path)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        restrict(&mut options);
        let file = options.open(path)?;
        file.set_len(len.max(1) as u64)?;
        Ok(Self {
            map: map_mut(&file)?,
            path: path.to_owned(),
            _file: file,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.map
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.map
    }

    /// 4バイト単位の画素（0RGB）として見る。マップはページ境界に揃うので整列は満たす。
    pub fn as_pixels(&self) -> &[u32] {
        bytemuck::cast_slice(&self.map[..self.map.len() / 4 * 4])
    }

    pub fn as_pixels_mut(&mut self) -> &mut [u32] {
        let len = self.map.len() / 4 * 4;
        bytemuck::cast_slice_mut(&mut self.map[..len])
    }

    /// `offset` から `data` を書く。範囲外なら書かずにエラー。
    pub fn write_at(&mut self, offset: usize, data: &[u8]) -> io::Result<()> {
        let end = offset
            .checked_add(data.len())
            .filter(|&end| end <= self.map.len())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "共有メモリの範囲外"))?;
        self.map[offset..end].copy_from_slice(data);
        Ok(())
    }

    /// `offset` から `len` バイトを複製して読む。範囲外ならエラー。
    pub fn read_at(&self, offset: usize, len: usize) -> io::Result<Vec<u8>> {
        read_range(&self.map, offset, len)
    }
}

/// 相手が作った共有メモリを読み取り専用で開いたもの。
pub struct SharedView {
    map: Mmap,
    path: PathBuf,
}

impl SharedView {
    /// 相手から届いたパスを開く。Kumo の共有メモリのファイルでなければ開かない。
    pub fn open(path: &Path) -> io::Result<Self> {
        check_path(path)?;
        let file = OpenOptions::new().read(true).open(path)?;
        Ok(Self {
            map: map_read(&file)?,
            path: path.to_owned(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn as_pixels(&self) -> &[u32] {
        bytemuck::cast_slice(&self.map[..self.map.len() / 4 * 4])
    }

    pub fn read_at(&self, offset: usize, len: usize) -> io::Result<Vec<u8>> {
        read_range(&self.map, offset, len)
    }
}

fn read_range(bytes: &[u8], offset: usize, len: usize) -> io::Result<Vec<u8>> {
    let end = offset
        .checked_add(len)
        .filter(|&end| end <= bytes.len())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "共有メモリの範囲外"))?;
    Ok(bytes[offset..end].to_vec())
}

/// 作成するファイルを本人だけが使えるようにし、Windows では閉じたら消えるようにする。
#[cfg(windows)]
fn restrict(options: &mut OpenOptions) {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_DELETE_ON_CLOSE: u32 = 0x0400_0000;
    options.custom_flags(FILE_FLAG_DELETE_ON_CLOSE);
}

#[cfg(unix)]
fn restrict(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
}

#[cfg(not(any(windows, unix)))]
fn restrict(_options: &mut OpenOptions) {}

// SAFETY（以下2つ共通）: ファイルは Kumo のプロセス同士だけが使う一時ファイルで、大きさは作成時に固定し
// 以後変えない（縮めてマップ外を参照させることはない）。中身は他プロセスからも書かれるが、
// 読み書きはメッセージで直列化しており、こちらはバイト列の複製しかしないので、型の不変条件が
// 壊れることはない（読んだ値が想定外でも、検査済みの範囲のバイトとして扱うだけ）。
fn map_mut(file: &File) -> io::Result<MmapMut> {
    #[allow(unsafe_code)]
    unsafe {
        MmapMut::map_mut(file)
    }
}

fn map_read(file: &File) -> io::Result<Mmap> {
    #[allow(unsafe_code)]
    unsafe {
        Mmap::map(file)
    }
}

impl Drop for SharedMemory {
    fn drop(&mut self) {
        // マップを外してから消す。Windows では読み手がまだ開いていれば、全員が閉じた時点で OS が消す
        let map = std::mem::replace(
            &mut self.map,
            MmapMut::map_anon(1).expect("1バイトの匿名マップ"),
        );
        drop(map);
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_mappings_see_the_same_bytes() {
        let path = unique_path("test");
        let mut a = SharedMemory::create(&path, 4096).unwrap();
        let b = SharedView::open(&path).unwrap();
        a.write_at(100, b"hello").unwrap();
        assert_eq!(b.read_at(100, 5).unwrap(), b"hello");
        assert_eq!(b.len(), 4096);
        assert!(a.write_at(4094, b"xyz").is_err(), "範囲外は書かない");
        assert!(b.read_at(usize::MAX, 2).is_err());
        a.as_pixels_mut()[0] = 0x00ff_0000;
        assert_eq!(b.as_pixels()[0], 0x00ff_0000);
        assert_eq!(b.as_pixels().len(), 1024);
        drop(b);
        drop(a);
        assert!(!path.exists(), "作成側が消す");
    }

    #[test]
    fn file_disappears_even_if_the_reader_outlives_the_owner() {
        let path = unique_path("test");
        let owner = SharedMemory::create(&path, 4096).unwrap();
        let reader = SharedView::open(&path).unwrap();
        drop(owner);
        assert_eq!(reader.len(), 4096, "読み手のマップは有効なまま");
        drop(reader);
        assert!(!path.exists(), "全員が閉じたら残らない");
    }

    #[test]
    fn never_reuses_or_truncates_existing_files() {
        let path = unique_path("test");
        std::fs::write(&path, b"keep").unwrap();
        assert!(
            SharedMemory::create(&path, 4096).is_err(),
            "既存のファイルは開かない"
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"keep");
        std::fs::remove_file(&path).unwrap();
        assert_ne!(unique_path("test"), unique_path("test"));
    }

    #[test]
    fn only_kumo_shared_files_can_be_opened() {
        let outside = std::env::current_dir().unwrap().join("Cargo.toml");
        assert!(SharedView::open(&outside).is_err());
        let wrong_name = std::env::temp_dir().join("other.shm");
        assert!(!is_shared_path(&wrong_name));
        let nested = std::env::temp_dir().join("x").join("kumo-a.shm");
        assert!(!is_shared_path(&nested));
        assert!(is_shared_path(&unique_path("frame")));
        assert!(
            SharedMemory::create(&outside, 1).is_err(),
            "作るのも一時ディレクトリだけ"
        );
    }
}
