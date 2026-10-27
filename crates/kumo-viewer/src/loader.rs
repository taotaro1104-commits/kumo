//! リソース取得（ローカルファイル / HTTP(S)）と URL 解決。

use std::path::PathBuf;

use url::Url;

/// 1リソースの上限（巨大レスポンスでメモリを使い切らないため）。
const MAX_RESOURCE_BYTES: u64 = 16 * 1024 * 1024;

/// 文書の URL を基準に相対参照を解決して読み込む。
#[derive(Clone, Debug)]
pub struct Loader {
    base: Url,
}

impl Loader {
    /// 引数（ファイルパスまたは URL）から作る。
    pub fn new(target: &str) -> Result<Self, String> {
        let base = if target.starts_with("http://")
            || target.starts_with("https://")
            || target.starts_with("file:")
        {
            Url::parse(target).map_err(|e| format!("URL を解釈できません: {e}"))?
        } else {
            let path = std::fs::canonicalize(PathBuf::from(target))
                .map_err(|e| format!("ファイルが見つかりません: {target}: {e}"))?;
            Url::from_file_path(&path)
                .map_err(|()| format!("パスを URL にできません: {}", path.display()))?
        };
        Ok(Self { base })
    }

    pub fn resolve(&self, reference: &str) -> Option<Url> {
        self.base.join(reference.trim()).ok()
    }

    /// 文書本体を読み込む。
    pub fn load_document(&self) -> Result<Vec<u8>, String> {
        self.fetch(&self.base)
    }

    /// 相対参照を解決して読み込む。失敗時はエラー内容を返す。
    pub fn load(&self, reference: &str) -> Result<Vec<u8>, String> {
        let url = self
            .resolve(reference)
            .ok_or_else(|| format!("URL を解決できません: {reference}"))?;
        self.fetch(&url)
    }

    fn fetch(&self, url: &Url) -> Result<Vec<u8>, String> {
        match url.scheme() {
            "file" => {
                let path = url
                    .to_file_path()
                    .map_err(|()| format!("不正なファイルURL: {url}"))?;
                std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
            }
            "http" | "https" => {
                let mut response = ureq::get(url.as_str())
                    .header("User-Agent", "Kumo/0.1 (Phase 0 static viewer)")
                    .call()
                    .map_err(|e| format!("{url}: {e}"))?;
                response
                    .body_mut()
                    .with_config()
                    .limit(MAX_RESOURCE_BYTES)
                    .read_to_vec()
                    .map_err(|e| format!("{url}: {e}"))
            }
            "data" => Err("data: URL は Phase 0 では未対応".to_owned()),
            other => Err(format!("未対応のスキーム: {other}")),
        }
    }
}

/// 画像の取得。失敗は描画しないだけなので握りつぶさずログに残す。
impl kumo_image::ImageLoader for Loader {
    fn load(&self, src: &str) -> Option<Vec<u8>> {
        match Loader::load(self, src) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                eprintln!("[image] {e}");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_references() {
        let loader = Loader::new("https://en.wikipedia.org/wiki/Rust").unwrap();
        assert_eq!(
            loader.resolve("/w/load.php?a=b").unwrap().as_str(),
            "https://en.wikipedia.org/w/load.php?a=b"
        );
        assert_eq!(
            loader
                .resolve("//upload.wikimedia.org/x.png")
                .unwrap()
                .as_str(),
            "https://upload.wikimedia.org/x.png"
        );
    }

    #[test]
    fn missing_file_is_an_error() {
        assert!(Loader::new("definitely/not/here.html").is_err());
    }
}

#[cfg(test)]
mod file_tests {
    use super::*;

    #[test]
    fn loads_files_and_relative_resources() {
        let path = format!(
            "{}/../../tests/fixtures/basic.html",
            env!("CARGO_MANIFEST_DIR")
        );
        let loader = Loader::new(&path).unwrap();
        let html = loader.load_document().unwrap();
        assert!(String::from_utf8_lossy(&html).contains("Kumo"));
        assert!(
            loader.load("blank.html").is_ok(),
            "同じディレクトリの相対参照"
        );
        assert!(loader.load("missing.css").is_err());
        assert!(loader.load("data:text/css,p{}").is_err());
        assert!(loader.load("ftp://example.com/x").is_err());
        assert!(kumo_image::ImageLoader::load(&loader, "missing.png").is_none());
    }
}
