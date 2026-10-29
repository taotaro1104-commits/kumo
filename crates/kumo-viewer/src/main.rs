//! Kumo ブラウザ（デスクトップ版）。使い方は [`kumo_viewer`] を参照。
#![forbid(unsafe_code)]

use kumo_mem::TaggedAllocator;

#[global_allocator]
static ALLOC: TaggedAllocator = TaggedAllocator;

fn main() {
    kumo_viewer::desktop_main();
}
