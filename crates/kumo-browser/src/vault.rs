//! 保存するパスワードの暗号化。ChaCha20-Poly1305 で包み、鍵は OS の資格情報ストア
//! （Windows の資格情報マネージャ）に置く。鍵はデータベースのファイルと別の場所にあるので、
//! ファイルだけが持ち出されても中身は読めない。ストアが使えないときは保存しない（平文では持たない）。

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

#[derive(Debug, PartialEq, Eq)]
pub struct VaultError(&'static str);

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "パスワードの保管庫: {}", self.0)
    }
}

impl std::error::Error for VaultError {}

/// 暗号化の鍵を持つ。`Debug` は鍵を出さない。
pub struct Vault {
    key: [u8; KEY_LEN],
}

impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Vault(..)")
    }
}

impl Drop for Vault {
    fn drop(&mut self) {
        self.key = [0; KEY_LEN]; // 鍵を残さない
    }
}

impl Vault {
    pub fn from_key(key: [u8; KEY_LEN]) -> Self {
        Self { key }
    }

    /// `aad`（どの欄の値か）を認証して暗号化する。結果は `ナンス ‖ 暗号文`。
    pub fn seal(&self, plain: &[u8], aad: &[u8]) -> Result<Vec<u8>, VaultError> {
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::fill(&mut nonce).map_err(|_| VaultError("乱数を得られません"))?;
        let cipher = ChaCha20Poly1305::new(&self.key.into());
        let sealed = cipher
            .encrypt(&Nonce::from(nonce), Payload { msg: plain, aad })
            .map_err(|_| VaultError("暗号化できません"))?;
        let mut out = nonce.to_vec();
        out.extend(sealed);
        Ok(out)
    }

    /// `seal` の逆。鍵・`aad` が違う、または改ざんされていれば失敗する。
    pub fn open(&self, sealed: &[u8], aad: &[u8]) -> Result<Vec<u8>, VaultError> {
        if sealed.len() < NONCE_LEN {
            return Err(VaultError("データが壊れています"));
        }
        let (nonce, body) = sealed.split_at(NONCE_LEN);
        let nonce = Nonce::try_from(nonce).map_err(|_| VaultError("データが壊れています"))?;
        ChaCha20Poly1305::new(&self.key.into())
            .decrypt(&nonce, Payload { msg: body, aad })
            .map_err(|_| VaultError("復号できません"))
    }

    /// OS の資格情報ストアにある鍵を使う。なければ（`create` なら）作って入れる。
    /// ストアが使えない環境では `None`。
    pub fn system(create: bool) -> Option<Self> {
        system::load(create).map(Self::from_key)
    }
}

#[cfg(windows)]
mod system {
    use super::KEY_LEN;

    const SERVICE: &str = "kumo-browser";
    const ACCOUNT: &str = "password-vault-key";

    fn entry() -> Option<keyring_core::Entry> {
        if keyring_core::get_default_store().is_none() {
            keyring_core::set_default_store(windows_native_keyring_store::Store::new().ok()?);
        }
        keyring_core::Entry::new(SERVICE, ACCOUNT).ok()
    }

    pub(super) fn load(create: bool) -> Option<[u8; KEY_LEN]> {
        let entry = entry()?;
        if let Ok(secret) = entry.get_secret()
            && let Ok(key) = <[u8; KEY_LEN]>::try_from(secret.as_slice())
        {
            return Some(key);
        }
        if !create {
            return None;
        }
        let mut key = [0u8; KEY_LEN];
        getrandom::fill(&mut key).ok()?;
        entry.set_secret(&key).ok()?;
        Some(key)
    }
}

#[cfg(not(windows))]
mod system {
    use super::KEY_LEN;

    pub(super) fn load(_create: bool) -> Option<[u8; KEY_LEN]> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_tamper_detection() {
        let vault = Vault::from_key([7; KEY_LEN]);
        let sealed = vault.seal(b"hunter2", b"a|b").unwrap();
        assert!(
            !sealed.windows(7).any(|w| w == b"hunter2"),
            "平文が残らない"
        );
        assert_eq!(vault.open(&sealed, b"a|b").unwrap(), b"hunter2");
        assert!(
            vault.open(&sealed, b"a|c").is_err(),
            "別の欄の値としては開けない"
        );
        assert!(Vault::from_key([8; KEY_LEN]).open(&sealed, b"a|b").is_err());
        let mut bad = sealed.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert!(vault.open(&bad, b"a|b").is_err());
        assert!(vault.open(&sealed[..5], b"a|b").is_err());
    }

    #[test]
    fn same_plaintext_gets_a_fresh_nonce() {
        let vault = Vault::from_key([1; KEY_LEN]);
        assert_ne!(
            vault.seal(b"x", b"").unwrap(),
            vault.seal(b"x", b"").unwrap()
        );
    }

    #[test]
    fn debug_does_not_leak_the_key() {
        assert_eq!(format!("{:?}", Vault::from_key([9; KEY_LEN])), "Vault(..)");
    }

    /// OS の資格情報ストアに触る（実際の鍵を作る）ので、手で `--ignored` を付けて動かす。
    #[test]
    #[ignore]
    fn system_key_is_stable_across_loads() {
        let first = Vault::system(true).expect("資格情報ストアが使える");
        let sealed = first.seal(b"x", b"").unwrap();
        let second = Vault::system(false).expect("作った鍵を読める");
        assert_eq!(second.open(&sealed, b"").unwrap(), b"x");
    }
}
