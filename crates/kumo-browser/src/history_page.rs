//! `about:history`：閲覧履歴の一覧ページ。削除は `kumo:unhistory?u=URL`、全消去は `kumo:clearhistory` のリンクで行う。

use kumo_content::i18n::fill;
use kumo_content::i18n::library::Lib;
use kumo_content::i18n::pages::Pg;
use kumo_content::{Lang, escape};
use url::form_urlencoded;

use crate::history::HistoryEntry;

pub const HISTORY_URL: &str = "about:history";
/// 1件の削除リンクの接頭辞（ページ内の `kumo:` リンク）。
pub(crate) const UNHISTORY_PREFIX: &str = "kumo:unhistory?u=";
pub(crate) const CLEAR_HISTORY: &str = "kumo:clearhistory";
/// 一覧に出す件数の上限。
pub(crate) const HISTORY_LIMIT: usize = 500;
/// これより古い訪問は「N 日前」ではなく日付で示す。
const RELATIVE_DAYS: u64 = 7;

const CSS: &str = "
html, body { margin: 0; background: #f6f7f9; color: #1f2328; font-family: 'Segoe UI', 'Yu Gothic UI', sans-serif; }
.wrap { max-width: 860px; margin: 0 auto; padding: 36px 24px; }
.head { display: flex; align-items: center; margin-bottom: 20px; }
h1 { flex: 1; margin: 0; font-size: 26px; color: #1a73e8; }
a.clear { padding: 6px 14px; border-radius: 14px; border: 1px solid #f0c7c4; color: #b3261e; font-size: 13px;
          text-decoration: none; background: #ffffff; }
.box { background: #ffffff; border: 1px solid #e1e4e8; border-radius: 12px; padding: 4px 16px; }
.row { display: flex; align-items: center; padding: 10px 0; border-bottom: 1px solid #eef0f2; }
.row:last-child { border-bottom: 0; }
.when { flex: none; width: 96px; color: #6b7280; font-size: 12px; }
.main { flex: 1; overflow: hidden; }
a.t { display: block; color: #1f2328; font-size: 14px; font-weight: 600; text-decoration: none; white-space: nowrap; overflow: hidden; }
.u { color: #6b7280; font-size: 12px; white-space: nowrap; overflow: hidden; }
.n { flex: none; padding: 0 10px; color: #6b7280; font-size: 12px; }
a.rm { flex: none; padding: 4px 10px; border-radius: 12px; color: #b3261e; font-size: 12px; text-decoration: none; }
.empty { padding: 28px; text-align: center; color: #6b7280; font-size: 14px; background: #ffffff;
         border: 1px dashed #d5d9de; border-radius: 12px; }
";

/// 年月日（UTC）。1970-01-01 からの日数から暦の日付を求める。
fn civil_date(secs: u64) -> String {
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year}-{month:02}-{day:02}")
}

/// 「5 分前」のような相対表示。古い訪問は日付（タイムゾーンを持たないので日単位まで）。
fn when(visited: u64, now: u64, lang: Lang) -> String {
    let secs = now.saturating_sub(visited);
    let n = |v: u64| v.to_string();
    if secs < 60 {
        Lib::AgoNow.t(lang).to_owned()
    } else if secs < 3_600 {
        fill(Lib::AgoMinutes.t(lang), &[&n(secs / 60)])
    } else if secs < 86_400 {
        fill(Lib::AgoHours.t(lang), &[&n(secs / 3_600)])
    } else if secs < RELATIVE_DAYS * 86_400 {
        fill(Lib::AgoDays.t(lang), &[&n(secs / 86_400)])
    } else {
        civil_date(visited)
    }
}

/// 一覧の HTML。`list` は新しい順、`now` は現在時刻（UNIX 秒）。
pub fn history_html(list: &[HistoryEntry], now: u64, lang: Lang) -> String {
    let title = Lib::History.t(lang);
    let mut html = format!(
        "<html><head><title>{title}</title><style>{CSS}</style></head><body><div class=wrap>\
         <div class=head><h1>{title}</h1>"
    );
    if !list.is_empty() {
        html.push_str(&format!(
            "<a class=clear href='{CLEAR_HISTORY}'>{}</a>",
            Lib::HistoryClear.t(lang)
        ));
    }
    html.push_str("</div>");
    if list.is_empty() {
        html.push_str(&format!("<div class=empty>{}</div>", Pg::NoHistory.t(lang)));
    } else {
        html.push_str("<div class=box>");
        for e in list {
            let label = if e.visit.title.trim().is_empty() {
                &e.url
            } else {
                &e.visit.title
            };
            let enc: String = form_urlencoded::byte_serialize(e.url.as_bytes()).collect();
            html.push_str(&format!(
                "<div class=row><div class=when>{when}</div>\
                 <div class=main><a class=t href='{url}'>{label}</a><div class=u>{url}</div></div>\
                 <div class=n>{visits}</div>\
                 <a class=rm href='{UNHISTORY_PREFIX}{enc}'>{delete}</a></div>",
                when = escape(&when(e.visit.last_visit, now, lang)),
                url = escape(&e.url),
                label = escape(label),
                visits = fill(Lib::Visits.t(lang), &[&e.visit.count.to_string()]),
                delete = Pg::Delete.t(lang),
            ));
        }
        html.push_str("</div>");
    }
    html.push_str("</div></body></html>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::Visit;

    fn entry(url: &str, title: &str, last_visit: u64, count: u32) -> HistoryEntry {
        HistoryEntry {
            url: url.to_owned(),
            visit: Visit {
                title: title.to_owned(),
                last_visit,
                count,
            },
        }
    }

    #[test]
    fn dates_are_computed_from_the_epoch() {
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(951_782_400), "2000-02-29", "うるう日");
        assert_eq!(civil_date(1_700_000_000), "2023-11-14");
    }

    #[test]
    fn times_are_relative_then_dated() {
        let now = 1_700_000_000;
        assert_eq!(when(now - 5, now, Lang::En), "just now");
        assert_eq!(when(now - 5 * 60, now, Lang::En), "5 min ago");
        assert_eq!(when(now - 3 * 3_600, now, Lang::Ja), "3 時間前");
        assert_eq!(when(now - 2 * 86_400, now, Lang::Es), "hace 2 d");
        assert_eq!(when(now - 30 * 86_400, now, Lang::En), "2023-10-15");
        assert_eq!(
            when(now + 100, now, Lang::En),
            "just now",
            "未来の時刻でも壊れない"
        );
    }

    #[test]
    fn lists_escaped_entries_with_delete_and_clear_links() {
        let now = 1_700_000_000;
        let html = history_html(
            &[
                entry("https://a.example/?q=1&r=<2>", "A <b>", now - 10, 3),
                entry("https://b.example/", "", now - 4_000, 1),
            ],
            now,
            Lang::En,
        );
        assert!(html.contains("A &lt;b&gt;") && !html.contains("<b>"));
        assert!(html.contains("kumo:unhistory?u=https%3A%2F%2Fb.example%2F"));
        assert!(html.contains("kumo:clearhistory"));
        assert!(html.contains("3 visits"));
        assert_eq!(html.matches("class=row").count(), 2);
        assert!(
            html.contains("class=t href='https://b.example/'>https://b.example/<"),
            "題名がなければ URL"
        );
    }

    #[test]
    fn empty_state_has_no_clear_button() {
        let html = history_html(&[], 0, Lang::Ja);
        assert!(!html.contains("kumo:clearhistory"));
        assert!(html.contains("まだ履歴がありません"));
    }
}
