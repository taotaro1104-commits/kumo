//! ダウンロード：保存すべき応答の判定、ファイル名の決定、ファイルへの書き出し。
//! 本文はメモリに溜めず、読みながらディスクへ書く（大きなファイルでもメモリは増えない）。

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use percent_encoding::percent_decode_str;
use url::Url;

use crate::protocol::{DownloadEvent, DownloadState, FetchRequest, RequestMethod, ResourceKind};
use crate::service::{NetworkService, header};

/// 経過を知らせる間隔の下限。
const REPORT_INTERVAL: Duration = Duration::from_millis(150);
const CHUNK: usize = 64 * 1024;
/// ファイル名の最大文字数（拡張子を含む）。
const MAX_NAME_CHARS: usize = 120;
/// 同じ名前があるとき `(1)` から数える上限。
const MAX_SUFFIX: u32 = 999;

/// ページとして表示できる種類（これ以外は保存する）。
fn is_displayable(mime: &str) -> bool {
    mime.is_empty()
        || mime.starts_with("text/")
        || mime.starts_with("image/")
        || mime.contains("javascript")
        || mime.ends_with("+xml")
        || mime.ends_with("+json")
        || matches!(mime, "application/xml" | "application/json")
}

/// この応答は表示せずに保存するか（ページの移動として取得したときの判定）。
pub(crate) fn wants_download(
    kind: ResourceKind,
    status: u16,
    headers: &ureq::http::HeaderMap,
) -> bool {
    if kind != ResourceKind::Document || !(200..300).contains(&status) {
        return false;
    }
    let attachment = header(headers, "content-disposition").is_some_and(|d| {
        d.trim_start()
            .to_ascii_lowercase()
            .starts_with("attachment")
    });
    let mime = header(headers, "content-type")
        .and_then(|c| c.split(';').next().map(|m| m.trim().to_ascii_lowercase()))
        .unwrap_or_default();
    attachment || !is_displayable(&mime)
}

/// `Content-Disposition` の `filename*=UTF-8''...` か `filename=...`。
fn disposition_name(value: &str) -> Option<String> {
    let mut plain = None;
    for part in value.split(';').skip(1) {
        let Some((key, val)) = part.split_once('=') else {
            continue;
        };
        let (key, val) = (key.trim().to_ascii_lowercase(), val.trim());
        if key == "filename*" {
            // 文字コード'言語'パーセントエンコードされた名前
            let encoded = val.splitn(3, '\'').nth(2).unwrap_or(val);
            return Some(percent_decode_str(encoded).decode_utf8_lossy().into_owned());
        }
        if key == "filename" {
            plain = Some(val.trim_matches('"').to_owned());
        }
    }
    plain
}

/// 保存するファイル名。ヘッダー、URL の末尾、の順に探し、使えない文字を除く。
pub fn file_name(disposition: Option<&str>, url: &Url) -> String {
    let from_url = || {
        url.path_segments()
            .and_then(|mut s| s.next_back())
            .map(|s| percent_decode_str(s).decode_utf8_lossy().into_owned())
    };
    let raw = disposition
        .and_then(disposition_name)
        .filter(|n| !n.trim().is_empty())
        .or_else(from_url)
        .unwrap_or_default();
    sanitize(&raw)
}

/// Windows のファイル名として安全な形にする（フォルダを抜ける指定・予約名・長すぎる名前を除く）。
fn sanitize(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let cleaned = cleaned.trim_matches(|c: char| c == ' ' || c == '.');
    if cleaned.is_empty() {
        return "download".to_owned();
    }
    let (stem, ext) = match cleaned.rfind('.') {
        Some(i) if i > 0 => cleaned.split_at(i),
        _ => (cleaned, ""),
    };
    let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&stem.to_ascii_uppercase().as_str())
        || (stem.len() == 4
            && (stem[..3].eq_ignore_ascii_case("COM") || stem[..3].eq_ignore_ascii_case("LPT"))
            && stem.as_bytes()[3].is_ascii_digit());
    let stem = if reserved {
        format!("_{stem}")
    } else {
        stem.to_owned()
    };
    let budget = MAX_NAME_CHARS.saturating_sub(ext.chars().count()).max(1);
    let stem: String = stem.chars().take(budget).collect();
    format!("{stem}{ext}")
}

/// 同じ名前が無い保存先を確保する（`.part` を作って予約する）。`name (1).ext` のように番号を付ける。
fn reserve(dir: &Path, name: &str) -> std::io::Result<(PathBuf, PathBuf, File)> {
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => name.split_at(i),
        _ => (name, ""),
    };
    for n in 0..=MAX_SUFFIX {
        let candidate = if n == 0 {
            name.to_owned()
        } else {
            format!("{stem} ({n}){ext}")
        };
        let target = dir.join(&candidate);
        let part = dir.join(format!("{candidate}.part"));
        if target.exists() {
            continue;
        }
        match OpenOptions::new().write(true).create_new(true).open(&part) {
            Ok(file) => return Ok((target, part, file)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::other("保存先の名前が決まりません"))
}

fn event(
    id: u64,
    name: &str,
    received: u64,
    total: Option<u64>,
    state: DownloadState,
) -> DownloadEvent {
    DownloadEvent {
        id,
        name: name.to_owned(),
        received,
        total,
        state,
    }
}

impl NetworkService {
    /// `url` を `dir` に保存する。経過は `report` に渡し、最後に必ず Done・Failed・Cancelled のどれかを1回渡す。
    /// `cancel` が真になったら中止して書きかけを消す。
    pub fn download(
        &self,
        id: u64,
        url: &str,
        dir: &Path,
        source_url: Option<&str>,
        cancel: &AtomicBool,
        report: &mut dyn FnMut(DownloadEvent),
    ) {
        let failed =
            |report: &mut dyn FnMut(DownloadEvent), name: &str, received, total, why: String| {
                report(event(id, name, received, total, DownloadState::Failed(why)));
            };
        let parsed = match Url::parse(url) {
            Ok(u) => u,
            Err(e) => return failed(report, "", 0, None, format!("{url}: {e}")),
        };
        let request = FetchRequest {
            id,
            url: url.to_owned(),
            method: RequestMethod::Get,
            kind: ResourceKind::Document,
            source_url: source_url.map(str::to_owned),
        };
        let mut response = match self.open_download(&request, &parsed) {
            Ok(r) => r,
            Err(e) => return failed(report, "", 0, None, e.to_string()),
        };
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return failed(report, "", 0, None, format!("HTTP {status}"));
        }
        let final_url =
            Url::parse(&ureq::ResponseExt::get_uri(&response).to_string()).unwrap_or(parsed);
        let name = file_name(
            header(response.headers(), "content-disposition").as_deref(),
            &final_url,
        );
        let total =
            header(response.headers(), "content-length").and_then(|v| v.parse::<u64>().ok());
        if let Err(e) = fs::create_dir_all(dir) {
            return failed(report, &name, 0, total, format!("{}: {e}", dir.display()));
        }
        let (target, part, mut file) = match reserve(dir, &name) {
            Ok(r) => r,
            Err(e) => return failed(report, &name, 0, total, e.to_string()),
        };
        let saved_name = target
            .file_name()
            .map_or_else(|| name.clone(), |n| n.to_string_lossy().into_owned());
        report(event(id, &saved_name, 0, total, DownloadState::Running));
        let outcome = copy_body(
            response.body_mut().with_config().limit(u64::MAX).reader(),
            &mut file,
            cancel,
            &mut |received| {
                report(event(
                    id,
                    &saved_name,
                    received,
                    total,
                    DownloadState::Running,
                ))
            },
        );
        drop(file);
        let (received, state) = match outcome {
            Ok(Copied::Finished(n)) => match fs::rename(&part, &target) {
                Ok(()) => (
                    n,
                    DownloadState::Done(target.to_string_lossy().into_owned()),
                ),
                Err(e) => (
                    n,
                    DownloadState::Failed(format!("{}: {e}", target.display())),
                ),
            },
            Ok(Copied::Cancelled(n)) => (n, DownloadState::Cancelled),
            Err((n, e)) => (n, DownloadState::Failed(e)),
        };
        if !matches!(state, DownloadState::Done(_)) {
            let _ = fs::remove_file(&part);
        }
        report(event(id, &saved_name, received, total, state));
    }
}

enum Copied {
    Finished(u64),
    Cancelled(u64),
}

/// 本文をファイルへ写す。受け取った量を一定間隔で `progress` に知らせる。
fn copy_body(
    mut body: impl Read,
    file: &mut File,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64),
) -> Result<Copied, (u64, String)> {
    let mut buf = vec![0u8; CHUNK];
    let mut received = 0u64;
    let mut last = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(Copied::Cancelled(received));
        }
        let n = body.read(&mut buf).map_err(|e| (received, e.to_string()))?;
        if n == 0 {
            file.flush().map_err(|e| (received, e.to_string()))?;
            return Ok(Copied::Finished(received));
        }
        file.write_all(&buf[..n])
            .map_err(|e| (received, e.to_string()))?;
        received += n as u64;
        if last.elapsed() >= REPORT_INTERVAL {
            progress(received);
            last = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn name_comes_from_header_then_url() {
        let u = url("https://example.com/dl/report%20final.pdf?x=1");
        assert_eq!(file_name(None, &u), "report final.pdf");
        assert_eq!(
            file_name(Some("attachment; filename=\"a b.zip\""), &u),
            "a b.zip"
        );
        assert_eq!(
            file_name(
                Some("attachment; filename=x.zip; filename*=UTF-8''%E6%97%A5%E6%9C%AC.zip"),
                &u
            ),
            "日本.zip",
            "filename* が優先"
        );
        assert_eq!(file_name(None, &url("https://example.com/")), "download");
    }

    #[test]
    fn unsafe_names_are_neutralized() {
        let u = url("https://example.com/x");
        let name = |d: &str| file_name(Some(d), &u);
        assert_eq!(
            name("attachment; filename=\"..\\\\..\\\\evil.exe\""),
            "__..__evil.exe",
            "区切りは全て置き換わり、フォルダを抜けられない"
        );
        assert!(!name("attachment; filename=\"../../etc/passwd\"").contains('/'));
        assert_eq!(name("attachment; filename=\"CON.txt\""), "_CON.txt");
        assert_eq!(name("attachment; filename=\"a<b>:c?.txt\""), "a_b__c_.txt");
        assert_eq!(name("attachment; filename=\"..\""), "download");
        let long = "x".repeat(300) + ".zip";
        let n = name(&format!("attachment; filename=\"{long}\""));
        assert_eq!(n.chars().count(), MAX_NAME_CHARS);
        assert!(n.ends_with(".zip"), "拡張子は残す");
    }

    #[test]
    fn detects_files_that_should_be_saved() {
        let headers = |pairs: &[(&str, &str)]| {
            let mut map = ureq::http::HeaderMap::new();
            for (k, v) in pairs {
                map.insert(
                    ureq::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                    v.parse().unwrap(),
                );
            }
            map
        };
        let doc = ResourceKind::Document;
        assert!(wants_download(
            doc,
            200,
            &headers(&[("content-type", "application/zip")])
        ));
        assert!(wants_download(
            doc,
            200,
            &headers(&[("content-type", "application/pdf")])
        ));
        assert!(wants_download(
            doc,
            200,
            &headers(&[("content-type", "video/mp4")])
        ));
        assert!(wants_download(
            doc,
            200,
            &headers(&[
                ("content-type", "text/html"),
                ("content-disposition", "Attachment; filename=a.html")
            ])
        ));
        assert!(!wants_download(
            doc,
            200,
            &headers(&[("content-type", "text/html; charset=utf-8")])
        ));
        assert!(!wants_download(
            doc,
            200,
            &headers(&[("content-type", "image/png")])
        ));
        assert!(!wants_download(
            doc,
            200,
            &headers(&[("content-type", "application/xhtml+xml")])
        ));
        assert!(
            !wants_download(doc, 200, &headers(&[])),
            "種類がなければ表示"
        );
        assert!(!wants_download(
            doc,
            404,
            &headers(&[("content-type", "application/zip")])
        ));
        assert!(!wants_download(
            ResourceKind::Image,
            200,
            &headers(&[("content-type", "application/zip")])
        ));
    }

    #[test]
    fn reserve_numbers_duplicates() {
        let dir = std::env::temp_dir().join(format!("kumo-dl-reserve-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.zip"), b"old").unwrap();
        let (target, part, _file) = reserve(&dir, "a.zip").unwrap();
        assert_eq!(target.file_name().unwrap(), "a (1).zip");
        assert!(part.exists());
        let (second, _, _f2) = reserve(&dir, "a.zip").unwrap();
        assert_eq!(
            second.file_name().unwrap(),
            "a (2).zip",
            "予約中の名前も避ける"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
