//! HTTP の応答の読み取りと、切れた接続の判定。

use crate::protocol::{FetchError, MAX_BODY_BYTES};
use crate::service::{header, network_error};

/// 使い回した接続がすでに閉じられていたことを示すエラーか（GET なら安全にやり直せる）。
pub(crate) fn is_stale_connection(e: &ureq::Error) -> bool {
    let text = e.to_string();
    text.contains("close_notify")
        || matches!(e, ureq::Error::Io(io) if matches!(
            io.kind(),
            std::io::ErrorKind::UnexpectedEof
                | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::BrokenPipe
        ))
}

/// 応答の (最終 URL, Content-Type, Access-Control-Allow-Origin, 本文)。
pub(crate) type ReadResponse = (String, Option<String>, Option<String>, Vec<u8>);

pub(crate) fn read_response(
    response: &mut ureq::http::Response<ureq::Body>,
) -> Result<ReadResponse, FetchError> {
    let final_url = ureq::ResponseExt::get_uri(&*response).to_string();
    let content_type = header(response.headers(), "content-type");
    let allow_origin = header(response.headers(), "access-control-allow-origin");
    let declared = header(response.headers(), "content-length").and_then(|v| v.parse::<u64>().ok());
    let body = read_body(response.body_mut(), declared)?;
    Ok((final_url, content_type, allow_origin, body))
}

/// 本文を読む。サーバーが TLS の close_notify を送らずに接続を閉じた場合（rustls は異常終了とみなす）は、
/// 長さが宣言されていない（接続の終わりが本文の終わり）か、宣言どおり揃っているときだけ、そこまでを本文とする。
fn read_body(body: &mut ureq::Body, declared: Option<u64>) -> Result<Vec<u8>, FetchError> {
    use std::io::Read;
    let mut reader = body.with_config().limit(MAX_BODY_BYTES).reader();
    let mut out = Vec::new();
    let mut chunk = [0u8; 16 * 1024];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => return Ok(out),
            Ok(n) => out.extend_from_slice(&chunk[..n]),
            Err(e)
                if missing_close_notify(&e) && declared.is_none_or(|n| n == out.len() as u64) =>
            {
                return Ok(out);
            }
            Err(e) => {
                return Err(match ureq::Error::from(e) {
                    ureq::Error::BodyExceedsLimit(_) => FetchError::TooLarge,
                    other => network_error(other),
                });
            }
        }
    }
}

fn missing_close_notify(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::UnexpectedEof && e.to_string().contains("close_notify")
}
