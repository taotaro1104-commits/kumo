//! HTTP キャッシュの方針（RFC 9111 の主要部分）：保存してよいか、いつまで新鮮か。

use std::time::{Duration, SystemTime};

/// 経験的な新鮮さ（Last-Modified からの経過時間に掛ける割合、RFC 9111 §4.2.2）。
const HEURISTIC_FRACTION: f64 = 0.1;
/// 経験的な新鮮さの上限。
const HEURISTIC_MAX: Duration = Duration::from_secs(24 * 60 * 60);

/// キャッシュ判断に使うレスポンスヘッダ。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CacheHeaders {
    pub cache_control: Option<String>,
    pub expires: Option<String>,
    pub date: Option<String>,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
}

/// 保存できるなら新鮮でいられる期限（`None` は即座に再検証が必要）を返す。
/// 保存してはいけないなら `Err(())`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Storability {
    /// 保存しない（no-store / private など）
    NotStorable,
    /// 保存し、`fresh_until` まで再検証なしで使える（`None` は毎回再検証）
    Store { fresh_until: Option<SystemTime> },
}

fn directives(cache_control: &str) -> impl Iterator<Item = (String, Option<&str>)> {
    cache_control.split(',').filter_map(|part| {
        let part = part.trim();
        if part.is_empty() {
            return None;
        }
        let (name, value) = match part.split_once('=') {
            Some((n, v)) => (n.trim(), Some(v.trim().trim_matches('"'))),
            None => (part, None),
        };
        Some((name.to_ascii_lowercase(), value))
    })
}

/// レスポンスの保存可否と新鮮さを判定する。`status` は 200 のみ保存する。
pub fn storability(status: u16, headers: &CacheHeaders, now: SystemTime) -> Storability {
    if status != 200 {
        return Storability::NotStorable;
    }
    let mut max_age = None;
    let mut no_cache = false;
    for (name, value) in headers
        .cache_control
        .as_deref()
        .map(directives)
        .into_iter()
        .flatten()
    {
        match name.as_str() {
            "no-store" | "private" => return Storability::NotStorable,
            "no-cache" => no_cache = true,
            "max-age" => max_age = value.and_then(|v| v.parse::<u64>().ok()),
            _ => {}
        }
    }
    if no_cache {
        return Storability::Store { fresh_until: None };
    }
    if let Some(secs) = max_age {
        return Storability::Store {
            fresh_until: Some(now + Duration::from_secs(secs)),
        };
    }
    if let Some(expires) = headers.expires.as_deref() {
        // 解釈できない Expires は「すでに期限切れ」（RFC 9111 §5.3）
        let until = httpdate::parse(expires).filter(|t| *t > now);
        return Storability::Store { fresh_until: until };
    }
    let heuristic = headers
        .last_modified
        .as_deref()
        .and_then(httpdate::parse)
        .and_then(|modified| now.duration_since(modified).ok())
        .map(|age| now + age.mul_f64(HEURISTIC_FRACTION).min(HEURISTIC_MAX));
    let has_validator = headers.etag.is_some() || headers.last_modified.is_some();
    if heuristic.is_none() && !has_validator {
        // 期限も検証手段もないものは保存しても再利用できない
        return Storability::NotStorable;
    }
    Storability::Store {
        fresh_until: heuristic,
    }
}

/// IMF-fixdate（`Sun, 06 Nov 1994 08:49:37 GMT`）だけを解釈する最小実装。
pub mod httpdate {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];

    pub fn parse(s: &str) -> Option<SystemTime> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        let [_, day, month, year, time, zone] = parts.as_slice() else {
            return None;
        };
        if !zone.eq_ignore_ascii_case("GMT") {
            return None;
        }
        let day: u64 = day.parse().ok()?;
        let month = MONTHS.iter().position(|m| m.eq_ignore_ascii_case(month))? as u64 + 1;
        let year: u64 = year.parse().ok()?;
        let mut hms = time.split(':').map(|v| v.parse::<u64>().ok());
        let (h, m, sec) = (hms.next()??, hms.next()??, hms.next()??);
        if !(1..=31).contains(&day) || h > 23 || m > 59 || sec > 60 || year < 1970 {
            return None;
        }
        let days = days_from_civil(year, month, day);
        Some(UNIX_EPOCH + Duration::from_secs(days * 86_400 + h * 3600 + m * 60 + sec))
    }

    /// 1970-01-01 からの日数（Howard Hinnant のアルゴリズム）。
    fn days_from_civil(y: u64, m: u64, d: u64) -> u64 {
        let y = if m <= 2 { y - 1 } else { y };
        let era = y / 400;
        let yoe = y - era * 400;
        let mp = (m + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// IMF-fixdate に書式化する（If-Modified-Since に元の文字列を使えない場合の予備）。
    pub fn format(t: SystemTime) -> String {
        let secs = t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let (days, rem) = (secs / 86_400, secs % 86_400);
        let (y, m, d) = civil_from_days(days);
        let weekday = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][(days % 7) as usize];
        let month = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ][(m - 1) as usize];
        format!(
            "{weekday}, {d:02} {month} {y} {:02}:{:02}:{:02} GMT",
            rem / 3600,
            rem % 3600 / 60,
            rem % 60
        )
    }

    fn civil_from_days(z: u64) -> (u64, u64, u64) {
        let z = z + 719_468;
        let era = z / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + u64::from(m <= 2);
        (y, m, d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn parses_and_formats_http_dates() {
        let t = httpdate::parse("Sun, 06 Nov 1994 08:49:37 GMT").unwrap();
        assert_eq!(t, at(784_111_777));
        assert_eq!(httpdate::format(t), "Sun, 06 Nov 1994 08:49:37 GMT");
        assert_eq!(
            httpdate::format(at(1_798_761_600)),
            "Fri, 01 Jan 2027 00:00:00 GMT"
        );
        assert!(httpdate::parse("yesterday").is_none());
        assert!(httpdate::parse("Sun, 06 Nov 1994 25:00:00 GMT").is_none());
    }

    #[test]
    fn cache_control_rules() {
        let now = at(1_000_000);
        let h = |cc: &str| CacheHeaders {
            cache_control: Some(cc.into()),
            ..CacheHeaders::default()
        };
        assert_eq!(
            storability(200, &h("no-store"), now),
            Storability::NotStorable
        );
        assert_eq!(
            storability(200, &h("private, max-age=60"), now),
            Storability::NotStorable
        );
        assert_eq!(
            storability(200, &h("public, max-age=60"), now),
            Storability::Store {
                fresh_until: Some(at(1_000_060))
            }
        );
        assert_eq!(
            storability(200, &h("no-cache, max-age=60"), now),
            Storability::Store { fresh_until: None }
        );
        assert_eq!(
            storability(404, &h("max-age=60"), now),
            Storability::NotStorable
        );
    }

    #[test]
    fn expires_and_heuristics() {
        let now = httpdate::parse("Mon, 01 Jan 2024 00:00:00 GMT").unwrap();
        let expires = CacheHeaders {
            expires: Some("Mon, 01 Jan 2024 01:00:00 GMT".into()),
            ..CacheHeaders::default()
        };
        assert_eq!(
            storability(200, &expires, now),
            Storability::Store {
                fresh_until: Some(now + Duration::from_secs(3600))
            }
        );
        let modified = CacheHeaders {
            last_modified: Some("Sun, 31 Dec 2023 14:00:00 GMT".into()),
            ..CacheHeaders::default()
        };
        // 10時間前に更新 → 1時間新鮮
        assert_eq!(
            storability(200, &modified, now),
            Storability::Store {
                fresh_until: Some(now + Duration::from_secs(3600))
            }
        );
        let etag_only = CacheHeaders {
            etag: Some("\"v1\"".into()),
            ..CacheHeaders::default()
        };
        assert_eq!(
            storability(200, &etag_only, now),
            Storability::Store { fresh_until: None }
        );
        assert_eq!(
            storability(200, &CacheHeaders::default(), now),
            Storability::NotStorable
        );
    }
}
