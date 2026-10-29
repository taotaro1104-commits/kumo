//! `about:downloads`：ダウンロードの一覧ページ。中止は `kumo:dlcancel?id=N`、整理は `kumo:dlclear` のリンクで行う。

use kumo_content::i18n::library::Lib;
use kumo_content::{Lang, escape};
use kumo_net::protocol::DownloadState;

use crate::downloads::DownloadItem;

pub const DOWNLOADS_URL: &str = "about:downloads";
pub(crate) const CANCEL_PREFIX: &str = "kumo:dlcancel?id=";
pub(crate) const CLEAR_FINISHED: &str = "kumo:dlclear";

const CSS: &str = "
html, body { margin: 0; background: #f6f7f9; color: #1f2328; font-family: 'Segoe UI', 'Yu Gothic UI', sans-serif; }
.wrap { max-width: 860px; margin: 0 auto; padding: 36px 24px; }
.head { display: flex; align-items: center; margin-bottom: 8px; }
h1 { flex: 1; margin: 0; font-size: 26px; color: #1a73e8; }
a.clear { padding: 6px 14px; border-radius: 14px; border: 1px solid #c6d4f5; color: #1a56c4; font-size: 13px;
          text-decoration: none; background: #ffffff; }
.dir { margin-bottom: 20px; color: #6b7280; font-size: 12px; }
.box { background: #ffffff; border: 1px solid #e1e4e8; border-radius: 12px; padding: 4px 16px; }
.row { display: flex; align-items: center; padding: 12px 0; border-bottom: 1px solid #eef0f2; }
.row:last-child { border-bottom: 0; }
.main { flex: 1; overflow: hidden; }
.name { color: #1f2328; font-size: 14px; font-weight: 600; white-space: nowrap; overflow: hidden; }
.info { margin-top: 3px; color: #6b7280; font-size: 12px; white-space: nowrap; overflow: hidden; }
.bar { margin-top: 6px; height: 6px; border-radius: 3px; background: #e8eaed; }
.fill { height: 6px; border-radius: 3px; background: #1a73e8; }
.ok { color: #1e7e34; }
.ng { color: #b3261e; }
a.act { flex: none; margin-left: 12px; padding: 4px 12px; border-radius: 12px; border: 1px solid #f0c7c4; color: #b3261e;
        font-size: 12px; text-decoration: none; }
.empty { padding: 28px; text-align: center; color: #6b7280; font-size: 14px; background: #ffffff;
         border: 1px dashed #d5d9de; border-radius: 12px; }
";

/// バイト数を読みやすい単位で。
pub(crate) fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// 進み具合の文字列と、バーの幅（%）。合計が分からないときは受信量だけ。
fn progress(item: &DownloadItem) -> (String, Option<u64>) {
    match item.total.filter(|t| *t > 0) {
        Some(total) => {
            let percent = (item.received.min(total) * 100 / total).min(100);
            (
                format!(
                    "{} / {} ({percent}%)",
                    human_size(item.received),
                    human_size(total)
                ),
                Some(percent),
            )
        }
        None => (human_size(item.received), None),
    }
}

fn row(item: &DownloadItem, lang: Lang) -> String {
    let (status, bar, action) = match &item.state {
        DownloadState::Running => {
            let (text, percent) = progress(item);
            let bar = percent.map_or_else(String::new, |p| {
                format!("<div class=bar><div class=fill style='width:{p}%'></div></div>")
            });
            let cancel = format!(
                "<a class=act href='{CANCEL_PREFIX}{}'>{}</a>",
                item.id,
                Lib::DownloadCancel.t(lang)
            );
            (escape(&text), bar, cancel)
        }
        DownloadState::Done(path) => (
            format!(
                "<span class=ok>{}</span> · {} · {}",
                Lib::DownloadDone.t(lang),
                human_size(item.received),
                escape(path)
            ),
            String::new(),
            String::new(),
        ),
        DownloadState::Failed(why) => (
            format!(
                "<span class=ng>{}</span> · {}",
                Lib::DownloadFailed.t(lang),
                escape(why)
            ),
            String::new(),
            String::new(),
        ),
        DownloadState::Cancelled => (
            format!("<span class=ng>{}</span>", Lib::DownloadCancelled.t(lang)),
            String::new(),
            String::new(),
        ),
    };
    format!(
        "<div class=row><div class=main><div class=name>{name}</div><div class=info>{status}</div>{bar}\
         <div class=info>{url}</div></div>{action}</div>",
        name = escape(&item.name),
        url = escape(&item.url),
    )
}

/// 一覧の HTML（新しい順）。`dir` は保存先フォルダ。
pub fn downloads_html(items: &[DownloadItem], dir: &str, lang: Lang) -> String {
    let title = Lib::Downloads.t(lang);
    let mut html = format!(
        "<html><head><title>{title}</title><style>{CSS}</style></head><body><div class=wrap>\
         <div class=head><h1>{title}</h1>"
    );
    if items.iter().any(|d| !d.is_running()) {
        html.push_str(&format!(
            "<a class=clear href='{CLEAR_FINISHED}'>{}</a>",
            Lib::DownloadClearFinished.t(lang)
        ));
    }
    html.push_str(&format!(
        "</div><div class=dir>{}: {}</div>",
        Lib::DownloadFolder.t(lang),
        escape(dir)
    ));
    if items.is_empty() {
        html.push_str(&format!(
            "<div class=empty>{}</div>",
            Lib::DownloadsEmpty.t(lang)
        ));
    } else {
        html.push_str("<div class=box>");
        for item in items.iter().rev() {
            html.push_str(&row(item, lang));
        }
        html.push_str("</div>");
    }
    html.push_str("</div></body></html>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: u64, state: DownloadState, received: u64, total: Option<u64>) -> DownloadItem {
        DownloadItem {
            id,
            name: format!("f{id}<x>.zip"),
            url: format!("https://e.example/{id}?a=1&b=2"),
            received,
            total,
            state,
        }
    }

    #[test]
    fn sizes_are_human_readable() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1023), "1023 B");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(human_size(3 << 30), "3.0 GB");
    }

    #[test]
    fn running_items_show_progress_and_a_cancel_link() {
        let html = downloads_html(
            &[item(
                7,
                DownloadState::Running,
                512 * 1024,
                Some(1024 * 1024),
            )],
            "C:\\Users\\me\\Downloads",
            Lang::En,
        );
        assert!(html.contains("512.0 KB / 1.0 MB (50%)"), "{html}");
        assert!(html.contains("width:50%"));
        assert!(html.contains("kumo:dlcancel?id=7"));
        assert!(
            html.contains("f7&lt;x&gt;.zip") && !html.contains("<x>"),
            "名前は無害化"
        );
        assert!(html.contains("C:\\Users\\me\\Downloads"));
        assert!(
            !html.contains("kumo:dlclear"),
            "終わった項目がなければ整理を出さない"
        );
    }

    #[test]
    fn unknown_total_shows_only_received() {
        let html = downloads_html(
            &[item(1, DownloadState::Running, 2048, None)],
            "d",
            Lang::En,
        );
        assert!(html.contains("2.0 KB") && !html.contains("class=fill"));
    }

    #[test]
    fn finished_items_show_their_outcome_newest_first() {
        let html = downloads_html(
            &[
                item(1, DownloadState::Done("C:\\d\\a.zip".into()), 10, Some(10)),
                item(2, DownloadState::Failed("HTTP 404".into()), 0, None),
                item(3, DownloadState::Cancelled, 5, None),
            ],
            "d",
            Lang::En,
        );
        assert!(html.contains("Done") && html.contains("C:\\d\\a.zip"));
        assert!(html.contains("Failed") && html.contains("HTTP 404"));
        assert!(html.contains("Cancelled"));
        assert!(html.contains("kumo:dlclear"));
        assert!(
            !html.contains("kumo:dlcancel"),
            "終わったものは中止できない"
        );
        assert!(
            html.find("f3").unwrap() < html.find("f1").unwrap(),
            "新しい順"
        );
    }

    #[test]
    fn empty_state_guides() {
        assert!(
            downloads_html(&[], "d", Lang::Ja).contains("ダウンロードしたファイルはありません")
        );
    }
}
