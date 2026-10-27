//! 読み込めなかったときのページ。原因を分かる言葉で示し、次にできること（再読み込み・戻る）を置く。

use kumo_net::protocol::FetchError;

use crate::page::escape;

/// エラーの見出しと、次にすること。
pub fn describe_error(error: &FetchError) -> (&'static str, &'static str) {
    match error {
        FetchError::Blocked => (
            "このページはブロックされました",
            "コンテンツブロックのフィルタに一致したため、読み込みませんでした。",
        ),
        FetchError::BadUrl(_) => (
            "URL が正しくありません",
            "アドレスの綴りを確認してください。",
        ),
        FetchError::UnsupportedScheme(_) => (
            "この種類のアドレスは開けません",
            "Kumo が開けるのは http・https・file のページです。",
        ),
        FetchError::TooLarge => ("ページが大きすぎます", "32MB を超えるページは開けません。"),
        FetchError::Disconnected => (
            "通信を担当するプロセスが止まりました",
            "ブラウザを起動し直してください。",
        ),
        FetchError::Network(message) => describe_network(message),
    }
}

fn describe_network(message: &str) -> (&'static str, &'static str) {
    let m = message.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| m.contains(w));
    if has(&[
        "os error 2)",
        "os error 3)",
        "no such file",
        "見つかりません",
    ]) {
        (
            "ファイルが見つかりません",
            "パスが正しいか、ファイルが移動・削除されていないか確認してください。",
        )
    } else if has(&[
        "lookup",
        "resolve",
        "dns",
        "no such host",
        "os error 11001",
        "name or service",
    ]) {
        (
            "サーバーが見つかりません",
            "アドレスの綴りと、インターネットへの接続を確認してください。",
        )
    } else if has(&["timed out", "timeout"]) {
        (
            "応答がありません",
            "サーバーが混んでいる可能性があります。時間をおいて再読み込みしてください。",
        )
    } else if has(&["refused", "os error 10061", "os error 111)"]) {
        (
            "接続が拒否されました",
            "サーバーが動いていないか、接続を受け付けていません。",
        )
    } else if has(&["certificate", "tls", "handshake"]) {
        (
            "安全な接続を確立できません",
            "サーバーの証明書に問題があるため、安全のため読み込みを中止しました。",
        )
    } else {
        (
            "ページを読み込めませんでした",
            "インターネットへの接続を確認して、再読み込みしてください。",
        )
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

/// エラーページの HTML。題名は「読み込めません」（タブの見出しになる）。
pub fn error_html(url: &str, error: &FetchError) -> String {
    let (headline, hint) = describe_error(error);
    format!(
        "<html><head><title>読み込めません</title><style>{CSS}</style></head><body><div class=card>\
         <div class=icon>!</div><h1>{headline}</h1><p class=hint>{hint}</p>\
         <div class=url>{}</div>\
         <div class=actions><a class='btn primary' href='kumo:reload'>再読み込み</a>\
         <a class='btn secondary' href='kumo:back'>戻る</a></div>\
         <div class=detail>詳細: {}</div></div></body></html>",
        escape(url),
        escape(&error.to_string())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_html_escapes_and_offers_actions() {
        let html = error_html(
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
            describe_network("C:\\a.html: (os error 2)").0,
            "ファイルが見つかりません"
        );
    }
}
