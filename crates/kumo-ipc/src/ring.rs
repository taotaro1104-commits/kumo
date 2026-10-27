//! 共有メモリのリングバッファの領域管理（書き手側）。
//!
//! 書き手は空きがあれば連続した領域を確保して本文を書き、位置と長さをメッセージで知らせる。
//! 読み手は複製したら位置を返す（release）。返却は確保順と前後してもよい。
//! 空きが足りなければ確保に失敗するので、呼び出し側は通常のメッセージで送ればよい（待たない）。

use std::collections::VecDeque;

#[derive(Debug)]
struct Region {
    start: u64,
    end: u64,
    released: bool,
}

/// 位置は単調増加の絶対位置で表す（リング上のオフセットは `pos % capacity`）。
#[derive(Debug)]
pub struct RingAllocator {
    capacity: u64,
    head: u64,
    tail: u64,
    regions: VecDeque<Region>,
}

impl RingAllocator {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity as u64,
            head: 0,
            tail: 0,
            regions: VecDeque::new(),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity as usize
    }

    /// 使用中のバイト数（折り返しで飛ばした分を含む）。
    pub fn used(&self) -> usize {
        (self.head - self.tail) as usize
    }

    /// `len` バイトの連続領域を確保し、絶対位置を返す。入らなければ `None`。
    pub fn alloc(&mut self, len: usize) -> Option<u64> {
        let len = len as u64;
        if len == 0 || len > self.capacity {
            return None;
        }
        let mut start = self.head;
        // リングの終端をまたぐなら次の周回の先頭から（終端の端数は捨てる）
        if start % self.capacity + len > self.capacity {
            start = start.next_multiple_of(self.capacity);
        }
        if start + len - self.tail > self.capacity {
            return None;
        }
        self.regions.push_back(Region {
            start,
            end: start + len,
            released: false,
        });
        self.head = start + len;
        Some(start)
    }

    /// 読み手が複製し終えた領域を返す。知らない位置は無視する（壊れた返却で破綻しない）。
    pub fn release(&mut self, start: u64) {
        if let Some(region) = self.regions.iter_mut().find(|r| r.start == start) {
            region.released = true;
        }
        while self.regions.front().is_some_and(|r| r.released) {
            let region = self.regions.pop_front().expect("先頭がある");
            self.tail = region.end;
        }
        if self.regions.is_empty() {
            self.tail = self.head;
        }
    }

    /// 絶対位置をリング上のオフセットにする。
    pub fn offset(&self, pos: u64) -> usize {
        (pos % self.capacity) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_until_full_and_frees_in_any_order() {
        let mut ring = RingAllocator::new(100);
        let a = ring.alloc(40).unwrap();
        let b = ring.alloc(40).unwrap();
        assert_eq!((a, b), (0, 40));
        assert_eq!(ring.alloc(30), None, "空きは 20 しかない");
        ring.release(b);
        assert_eq!(ring.alloc(30), None, "先頭（a）が残っているので空かない");
        ring.release(a);
        assert_eq!(ring.used(), 0);
        // 終端ちょうどまでは収まる
        let c = ring.alloc(20).unwrap();
        assert_eq!(ring.offset(c), 80);
        // 終端をまたぐ確保は次の周回の先頭へ（端数は捨てる）
        let d = ring.alloc(30).unwrap();
        assert_eq!((d, ring.offset(d)), (100, 0));
        assert_eq!(ring.alloc(60), None, "c と d の 50 が使用中で空きは 50");
        let e = ring.alloc(50).unwrap();
        assert_eq!(ring.offset(e), 30);
        assert_eq!(ring.alloc(1), None, "満杯");
        ring.release(e);
        ring.release(999); // 知らない位置
        ring.release(c);
        ring.release(d);
        assert_eq!(ring.used(), 0);
        assert_eq!(ring.alloc(0), None);
        assert_eq!(ring.alloc(101), None);
    }
}
