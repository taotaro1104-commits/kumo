//! 読み込めなかったときのページ。原因を分かる言葉で示し、次にできること（再読み込み・戻る）を置く。

use kumo_net::protocol::FetchError;

use crate::i18n::errors::Er;
use crate::i18n::{Lang, fill};
use crate::page::escape;

/// エラーの見出しと、次にすること（日本語）。
pub fn describe_error(error: &FetchError) -> (&'static str, &'static str) {
    describe_error_in(Lang::Ja, error)
}

/// エラーの見出しと、次にすること。
pub fn describe_error_in(lang: Lang, error: &FetchError) -> (&'static str, &'static str) {
    let (head, hint) = classify(error);
    (head.t(lang), hint.t(lang))
}

fn classify(error: &FetchError) -> (Er, Er) {
    match error {
        FetchError::Blocked => (Er::BlockedHead, Er::BlockedHint),
        FetchError::BadUrl(_) => (Er::BadUrlHead, Er::BadUrlHint),
        FetchError::UnsupportedScheme(_) => (Er::SchemeHead, Er::SchemeHint),
        FetchError::TooLarge => (Er::TooLargeHead, Er::TooLargeHint),
        FetchError::Disconnected => (Er::DisconnectedHead, Er::DisconnectedHint),
        // 通常は Browser が保存に切り替えるので表示されない
        FetchError::Download => (Er::OtherHead, Er::OtherHint),
        FetchError::Network(message) => classify_network(message),
    }
}

fn classify_network(message: &str) -> (Er, Er) {
    let m = message.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| m.contains(w));
    if has(&[
        "os error 2)",
        "os error 3)",
        "no such file",
        "見つかりません",
    ]) {
        (Er::FileMissingHead, Er::FileMissingHint)
    } else if has(&[
        "lookup",
        "resolve",
        "dns",
        "no such host",
        "os error 11001",
        "name or service",
    ]) {
        (Er::HostMissingHead, Er::HostMissingHint)
    } else if has(&["timed out", "timeout"]) {
        (Er::TimeoutHead, Er::TimeoutHint)
    } else if has(&["refused", "os error 10061", "os error 111)"]) {
        (Er::RefusedHead, Er::RefusedHint)
    } else if has(&["certificate", "tls", "handshake"]) {
        (Er::TlsHead, Er::TlsHint)
    } else {
        (Er::OtherHead, Er::OtherHint)
    }
}

const CSS: &str = "
html, body { margin: 0; background: #f6f7f9; color: #1f2328; font-family: 'Segoe UI', 'Yu Gothic UI', sans-serif; }
.card { max-width: 560px; margin: 72px auto 0; padding: 32px 36px; background: #ffffff;
        border: 1px solid #e1e4e8; border-radius: 16px; }
.icon { width: 44px; height: 44px; line-height: 44px; border-radius: 22px; background: #fdecea; color: #b3261e;
        text-align: center; font-size: 24px; font-weight: 700; }
h1 { margin: 20px 0 8px; font-size: 22px; font-weight: 600; }
.hint { margin: 0 0 20px; color: #4a5058; font-size: 15px; line-height: 1.6; }
.url { padding: 8px 12px; background: #f1f3f4; border-radius: 8px; color: #4a5058; font-size: 13px; overflow: hidden;
       white-space: nowrap; }
.actions { display: flex; gap: 10px; margin-top: 24px; }
.btn { display: block; padding: 0 18px; height: 36px; line-height: 36px; border-radius: 18px; text-decoration: none;
       font-size: 14px; font-weight: 600; }
.primary { background: #1a73e8; color: #ffffff; }
.secondary { background: #ffffff; color: #1a56c4; border: 1px solid #c6d4f5; height: 34px; line-height: 34px; }
.detail { margin-top: 24px; color: #80868b; font-size: 12px; }
";

/// エラーページの HTML。題名（「読み込めません」）はタブの見出しになる。
pub fn error_html(lang: Lang, url: &str, error: &FetchError) -> String {
    let (headline, hint) = describe_error_in(lang, error);
    format!(
        "<html><head><title>{}</title><style>{CSS}</style></head><body><div class=card>\
         <div class=icon>!</div><h1>{headline}</h1><p class=hint>{hint}</p>\
         <div class=url>{}</div>\
         <div class=actions><a class='btn primary' href='kumo:reload'>{}</a>\
         <a class='btn secondary' href='kumo:back'>{}</a></div>\
         <div class=detail>{}</div></div></body></html>",
        Er::PageTitle.t(lang),
        escape(url),
        Er::Reload.t(lang),
        Er::Back.t(lang),
        escape(&fill(Er::Detail.t(lang), &[&error.to_string()]))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_html_escapes_and_offers_actions() {
        let html = error_html(
            Lang::Ja,
            "https://a.example/<x>",
            &FetchError::Network("<oops>".into()),
        );
        assert!(html.contains("&lt;x&gt;") && html.contains("&lt;oops&gt;"));
        assert!(html.contains("kumo:reload") && html.contains("kumo:back"));
        assert_eq!(
            describe_error(&FetchError::TooLarge).0,
            "ページが大きすぎます"
        );
        assert_eq!(
            describe_error(&FetchError::BadUrl("x".into())).0,
            "URL が正しくありません"
        );
        assert!(
            describe_error(&FetchError::Disconnected)
                .0
                .contains("プロセス")
        );
        assert_eq!(
            classify_network("C:\\a.html: (os error 2)").0.t(Lang::Ja),
            "ファイルが見つかりません"
        );
    }
}
