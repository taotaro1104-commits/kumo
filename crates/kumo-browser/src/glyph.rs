//! ツールバーの記号。Android の標準フォントには `↻` `⌂` `☰` がなく表示が崩れるので、
//! Android では同じ役目で表示できる記号に替える（実機で確かめた組）。

#[cfg(not(target_os = "android"))]
pub const RELOAD: &str = "↻";
#[cfg(not(target_os = "android"))]
pub const HOME: &str = "⌂";
#[cfg(not(target_os = "android"))]
pub const LIST: &str = "☰";

#[cfg(target_os = "android")]
pub const RELOAD: &str = "♻";
#[cfg(target_os = "android")]
pub const HOME: &str = "☖";
#[cfg(target_os = "android")]
pub const LIST: &str = "≡";
