//! `file:` URL の読み込み（ネットワークを使わない）。

use url::Url;

use crate::protocol::{FetchError, FetchRequest, FetchResponse, MAX_BODY_BYTES};

/// ローカルのファイルを読んで応答にする。大きさの上限は HTTP の本文と同じ。
pub(crate) fn read_file(request: &FetchRequest, url: &Url) -> Result<FetchResponse, FetchError> {
    let path = url
        .to_file_path()
        .map_err(|()| FetchError::BadUrl(url.to_string()))?;
    let io_error = |e: std::io::Error| FetchError::Network(format!("{}: {e}", path.display()));
    let metadata = std::fs::metadata(&path).map_err(io_error)?;
    if metadata.len() > MAX_BODY_BYTES {
        return Err(FetchError::TooLarge);
    }
    let body = std::fs::read(&path).map_err(io_error)?;
    Ok(FetchResponse {
        id: request.id,
        status: 200,
        final_url: url.to_string(),
        content_type: None,
        body,
        from_cache: false,
        allow_origin: None,
    })
}
