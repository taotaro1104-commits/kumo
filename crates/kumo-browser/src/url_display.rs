//! URL の見せ方：安全性の表示と、ドメインを強調した表記（URL 欄・リンク先の吹き出し・新しいタブで使う）。

use url::Url;

/// 通信の安全性。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Security {
    /// https：暗号化されている
    Secure,
    /// http：暗号化されていない
    Insecure,
    /// ローカルファイル
    File,
    /// それ以外（about: など）。表示しない
    None,
}

impl Security {
    /// URL 欄の左に出す印と言葉、CSS のクラス。
    pub fn chip(self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::Secure => Some(("secure", "🔒")),
            Self::Insecure => Some(("insecure", "⚠ 保護されていない通信")),
            Self::File => Some(("file", "🗋 ファイル")),
            Self::None => None,
        }
    }
}

/// 表示用に分けた URL。`host` を濃く、`rest` を薄く表示する。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UrlParts {
    pub security: Security,
    pub host: String,
    pub rest: String,
}

/// URL を表示用に分ける。https は「https://」を省き、パーセントエンコードは読める形に戻す。
pub fn format_url(text: &str) -> UrlParts {
    let Ok(url) = Url::parse(text) else {
        return UrlParts {
            security: Security::None,
            host: String::new(),
            rest: text.to_owned(),
        };
    };
    let decode = |s: &str| {
        percent_encoding::percent_decode_str(s)
            .decode_utf8_lossy()
            .into_owned()
    };
    match url.scheme() {
        "https" | "http" => {
            let security = if url.scheme() == "https" {
                Security::Secure
            } else {
                Security::Insecure
            };
            let host = url
                .host_str()
                .unwrap_or_default()
                .trim_start_matches("www.")
                .to_owned();
            let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
            let mut rest = format!("{port}{}", url.path());
            if rest == "/" {
                rest.clear();
            }
            if let Some(query) = url.query() {
                rest.push('?');
                rest.push_str(query);
            }
            if let Some(fragment) = url.fragment() {
                rest.push('#');
                rest.push_str(fragment);
            }
            UrlParts {
                security,
                host,
                rest: decode(&rest),
            }
        }
        "file" => {
            let path = url
                .to_file_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| decode(url.path()));
            UrlParts {
                security: Security::File,
                host: String::new(),
                rest: path,
            }
        }
        _ => UrlParts {
            security: Security::None,
            host: String::new(),
            rest: text.to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_hides_the_scheme_and_emphasizes_the_domain() {
        let parts = format_url("https://www.example.com/wiki/Caf%C3%A9?q=1#top");
        assert_eq!(parts.security, Security::Secure);
        assert_eq!(parts.host, "example.com");
        assert_eq!(parts.rest, "/wiki/Café?q=1#top");
        assert_eq!(
            format_url("https://a.example/").rest,
            "",
            "ルートのパスは省く"
        );
    }

    #[test]
    fn insecure_files_and_others() {
        let http = format_url("http://localhost:8080/x");
        assert_eq!(http.security, Security::Insecure);
        assert_eq!(
            (http.host.as_str(), http.rest.as_str()),
            ("localhost", ":8080/x")
        );
        assert!(
            Security::Insecure
                .chip()
                .unwrap()
                .1
                .contains("保護されていない")
        );
        let file = if cfg!(windows) {
            format_url("file:///C:/dir/a%20b.html")
        } else {
            format_url("file:///dir/a%20b.html")
        };
        assert_eq!(file.security, Security::File);
        assert!(file.rest.ends_with("a b.html"), "{}", file.rest);
        assert_eq!(format_url("about:blank").security, Security::None);
        assert_eq!(format_url("not a url").rest, "not a url");
        assert_eq!(Security::None.chip(), None);
    }
}
