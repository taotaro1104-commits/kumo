//! `about:memory`：メモリの使用量と各タブの状態（凍結・破棄）を一覧するページ。開いた時点の値で、再読み込みで更新する。

use kumo_content::escape;

use crate::governor::{Policy, TabState};

pub const ABOUT_MEMORY_URL: &str = "about:memory";

/// ページに出す数値。
pub struct MemoryReport {
    pub browser_bytes: u64,
    pub content_bytes: u64,
    pub policy: Policy,
    pub pressure: &'static str,
    pub tabs: Vec<TabRow>,
    pub frozen_total: u32,
    pub discarded_total: u32,
}

pub struct TabRow {
    pub title: String,
    pub url: String,
    pub state: TabState,
    pub active: bool,
    /// 最後に表示してからの秒数
    pub idle_secs: u64,
}

const CSS: &str = "
html, body { margin: 0; background: #f6f7f9; color: #1f2328; font-family: 'Segoe UI', 'Yu Gothic UI', sans-serif; }
.wrap { max-width: 860px; margin: 0 auto; padding: 36px 24px; }
h1 { margin: 0 0 20px; font-size: 26px; color: #1a73e8; }
h2 { margin: 28px 0 10px; font-size: 14px; color: #4a5058; }
.box { background: #ffffff; border: 1px solid #e1e4e8; border-radius: 12px; padding: 6px 16px; }
.row { display: flex; justify-content: space-between; padding: 8px 0; border-bottom: 1px solid #eef0f2; font-size: 14px; }
.row:last-child { border-bottom: 0; }
.k { color: #4a5058; }
.v { font-weight: 600; }
.t { flex: 1; overflow: hidden; white-space: nowrap; }
.u { color: #6b7280; font-size: 12px; }
.st { width: 90px; text-align: right; font-weight: 600; }
.live { color: #188038; }
.frozen { color: #1a56c4; }
.discarded { color: #8a9099; }
";

fn mb(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

fn row(key: &str, value: &str) -> String {
    format!("<div class=row><span class=k>{key}</span><span class=v>{value}</span></div>")
}

pub fn memory_html(report: &MemoryReport) -> String {
    let total = report.browser_bytes + report.content_bytes;
    let mut html = format!(
        "<html><head><title>メモリ</title><style>{CSS}</style></head><body><div class=wrap><h1>メモリ</h1>"
    );
    html.push_str("<h2>使用量</h2><div class=box>");
    html.push_str(&row("Browser プロセス", &mb(report.browser_bytes)));
    html.push_str(&row("Content プロセス", &mb(report.content_bytes)));
    html.push_str(&row("合計（予算の対象）", &mb(total)));
    html.push_str(&row("予算", &mb(report.policy.budget)));
    html.push_str(&row("OS のメモリ圧迫", report.pressure));
    html.push_str("</div><h2>方針</h2><div class=box>");
    html.push_str(&row(
        "背景に回って凍結するまで",
        &format!("{} 秒", report.policy.freeze_after.as_secs()),
    ));
    html.push_str(&row(
        "見なくなって破棄するまで",
        &format!("{} 秒", report.policy.discard_after.as_secs()),
    ));
    html.push_str(&row(
        "これまでの降格",
        &format!(
            "凍結 {} 回 / 破棄 {} 回",
            report.frozen_total, report.discarded_total
        ),
    ));
    html.push_str("</div><h2>タブ</h2><div class=box>");
    for tab in &report.tabs {
        let state = tab.state.label();
        let marker = if tab.active { "（表示中）" } else { "" };
        html.push_str(&format!(
            "<div class=row><span class=t>{}{marker}<br><span class=u>{} ・ {} 秒前</span></span>\
             <span class='st {state}'>{state}</span></div>",
            escape(&tab.title),
            escape(&tab.url),
            tab.idle_secs
        ));
    }
    html.push_str("</div></div></body></html>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> MemoryReport {
        MemoryReport {
            browser_bytes: 8 * 1024 * 1024,
            content_bytes: 40 * 1024 * 1024,
            policy: Policy::default(),
            pressure: "なし",
            tabs: vec![
                TabRow {
                    title: "<b>x</b>".into(),
                    url: "https://a.example/".into(),
                    state: TabState::Live,
                    active: true,
                    idle_secs: 0,
                },
                TabRow {
                    title: "old".into(),
                    url: "https://b.example/".into(),
                    state: TabState::Frozen,
                    active: false,
                    idle_secs: 90,
                },
            ],
            frozen_total: 3,
            discarded_total: 1,
        }
    }

    #[test]
    fn shows_totals_budget_and_states() {
        let html = memory_html(&report());
        assert!(html.contains("48.0 MB"), "合計");
        assert!(html.contains("200.0 MB"), "予算");
        assert!(html.contains("class='st frozen'>frozen"));
        assert!(html.contains("凍結 3 回 / 破棄 1 回"));
        assert!(html.contains("（表示中）"));
    }

    #[test]
    fn escapes_page_titles() {
        let html = memory_html(&report());
        assert!(html.contains("&lt;b&gt;x&lt;/b&gt;"));
        assert!(!html.contains("<b>x</b>"));
    }
}
