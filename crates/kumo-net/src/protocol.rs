//! Browser ⇔ Network プロセス間のメッセージとフレーミング。
//!
//! 1フレーム = 4バイト（リトルエンディアン）の長さ + postcard でシリアライズした本体。

use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};

/// 1フレームの上限。レスポンス本文の上限（[`MAX_BODY_BYTES`]）に余裕を足した値。
pub const MAX_FRAME_BYTES: usize = 40 * 1024 * 1024;
/// レスポンス本文の上限。これを超えるレスポンスはエラーにする。
pub const MAX_BODY_BYTES: u64 = 32 * 1024 * 1024;

/// 取得するリソースの種類（コンテンツブロックの判定に使う）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceKind {
    Document,
    Stylesheet,
    Image,
    /// `<script src>`
    Script,
    /// スクリプトの `fetch` / `XMLHttpRequest`（同じオリジン。Cookie を送る）
    Fetch,
    /// 別のオリジンへのスクリプトの要求。Cookie などの資格情報を送らず、キャッシュも使わない
    FetchAnonymous,
    Other,
}

impl ResourceKind {
    /// adblock の request_type 文字列。
    pub fn as_adblock_type(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Stylesheet => "stylesheet",
            Self::Image => "image",
            Self::Script => "script",
            Self::Fetch | Self::FetchAnonymous => "xmlhttprequest",
            Self::Other => "other",
        }
    }
}

/// リクエストのメソッド。POST はフォーム送信用（キャッシュしない）。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestMethod {
    #[default]
    Get,
    Post {
        content_type: String,
        body: Vec<u8>,
    },
    /// スクリプトからの GET 以外の要求（PUT・DELETE・Content-Type なしの POST など）。キャッシュしない
    Send {
        method: String,
        content_type: Option<String>,
        body: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchRequest {
    pub id: u64,
    pub url: String,
    pub method: RequestMethod,
    pub kind: ResourceKind,
    /// 読み込み元の文書 URL（サードパーティ判定に使う）。文書自身なら `None`
    pub source_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchResponse {
    pub id: u64,
    pub status: u16,
    /// リダイレクト後の URL
    pub final_url: String,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
    pub from_cache: bool,
    /// `Access-Control-Allow-Origin` ヘッダの値（別オリジンのスクリプトに読ませてよいかの判定に使う）
    pub allow_origin: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FetchError {
    /// コンテンツブロックで遮断した
    Blocked,
    BadUrl(String),
    UnsupportedScheme(String),
    TooLarge,
    Network(String),
    /// Network プロセスが落ちた・応答しない
    Disconnected,
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blocked => write!(f, "コンテンツブロックで遮断"),
            Self::BadUrl(u) => write!(f, "不正な URL: {u}"),
            Self::UnsupportedScheme(s) => write!(f, "未対応のスキーム: {s}"),
            Self::TooLarge => write!(f, "レスポンスが大きすぎます"),
            Self::Network(e) => write!(f, "{e}"),
            Self::Disconnected => write!(f, "Network プロセスとの接続が切れました"),
        }
    }
}

impl std::error::Error for FetchError {}

/// Network プロセスの統計（about:memory 用）。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkStats {
    pub requests: u64,
    pub cache_hits: u64,
    pub revalidated: u64,
    pub blocked: u64,
    pub memory_cache_bytes: u64,
    pub disk_cache_bytes: u64,
    pub private_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToNetwork {
    Fetch(FetchRequest),
    Stats {
        id: u64,
    },
    /// 共有リングの本文を複製し終えた（`pos` は [`FromNetwork::SharedResponse`] の位置）
    Release {
        pos: u64,
    },
    Shutdown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FromNetwork {
    /// 接続直後に1度だけ送る。本文リング（共有メモリ）の場所と大きさ
    Hello {
        ring_path: String,
        ring_capacity: u64,
    },
    Response(FetchResponse),
    /// 本文を共有リングの `pos` から `len` バイトに置いた応答（`response.body` は空）
    SharedResponse {
        response: FetchResponse,
        pos: u64,
        len: u64,
    },
    Failed {
        id: u64,
        error: FetchError,
    },
    Stats {
        id: u64,
        stats: NetworkStats,
    },
}

/// メッセージを1フレームとして書き出す。
pub fn write_message<T: Serialize>(writer: &mut impl Write, message: &T) -> io::Result<()> {
    let bytes = postcard::to_stdvec(message).map_err(io::Error::other)?;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "フレームが大きすぎる",
        ));
    }
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()
}

/// 1フレームを読む。相手が閉じていれば `Ok(None)`。
pub fn read_message<T: for<'de> Deserialize<'de>>(reader: &mut impl Read) -> io::Result<Option<T>> {
    let mut len = [0u8; 4];
    match reader.read_exact(&mut len) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_le_bytes(len) as usize;
    if len > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "フレームが大きすぎる",
        ));
    }
    let mut bytes = vec![0u8; len];
    reader.read_exact(&mut bytes)?;
    postcard::from_bytes(&bytes)
        .map(Some)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_messages() {
        let messages = vec![
            ToNetwork::Fetch(FetchRequest {
                id: 7,
                url: "https://example.com/a.css".into(),
                method: RequestMethod::Post {
                    content_type: "text/plain".into(),
                    body: b"x=1".to_vec(),
                },
                kind: ResourceKind::Stylesheet,
                source_url: Some("https://example.com/".into()),
            }),
            ToNetwork::Stats { id: 1 },
            ToNetwork::Shutdown,
        ];
        let mut buf = Vec::new();
        for m in &messages {
            write_message(&mut buf, m).unwrap();
        }
        let mut reader = buf.as_slice();
        for m in &messages {
            assert_eq!(
                read_message::<ToNetwork>(&mut reader).unwrap().as_ref(),
                Some(m)
            );
        }
        assert_eq!(
            read_message::<ToNetwork>(&mut reader).unwrap(),
            None,
            "終端"
        );
    }

    #[test]
    fn rejects_corrupt_and_oversized_frames() {
        let mut huge = (u32::MAX).to_le_bytes().to_vec();
        huge.extend([0; 8]);
        assert!(read_message::<FromNetwork>(&mut huge.as_slice()).is_err());
        let garbage = [3u8, 0, 0, 0, 0xff, 0xff, 0xff];
        assert!(read_message::<FromNetwork>(&mut garbage.as_slice()).is_err());
        let truncated = [10u8, 0, 0, 0, 1];
        assert!(read_message::<FromNetwork>(&mut truncated.as_slice()).is_err());
    }
}
