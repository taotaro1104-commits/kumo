//! `about:memory`：メモリの使用量と各タブの状態（凍結・破棄）を一覧するページ。開いた時点の値で、再読み込みで更新する。

use kumo_content::i18n::{fill, pages::Pg};
use kumo_content::{Lang, escape};
use kumo_mem::Pressure;

use crate::governor::{Policy, TabState};

pub const ABOUT_MEMORY_URL: &str = "about:memory";

/// ページに出す数値。
pub struct MemoryReport {
    pub browser_bytes: u64,
    pub content_bytes: u64,
    pub policy: Policy,
    pub pressure: Pressure,
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

pub fn memory_html(report: &MemoryReport, lang: Lang) -> String {
    let t = |m: Pg| m.t(lang);
    let total = report.browser_bytes + report.content_bytes;
    let seconds = |n: u64| fill(t(Pg::Seconds), &[&n.to_string()]);
    let pressure = match report.pressure {
        Pressure::None => Pg::PressureNone,
        Pressure::Moderate => Pg::PressureModerate,
        Pressure::Critical => Pg::PressureCritical,
    };
    let mut html = format!(
        "<html><head><title>{0}</title><style>{CSS}</style></head><body><div class=wrap><h1>{0}</h1>",
        t(Pg::MemoryTitle)
    );
    html.push_str(&format!("<h2>{}</h2><div class=box>", t(Pg::Usage)));
    html.push_str(&row(t(Pg::BrowserProcess), &mb(report.browser_bytes)));
    html.push_str(&row(t(Pg::ContentProcess), &mb(report.content_bytes)));
    html.push_str(&row(t(Pg::Total), &mb(total)));
    html.push_str(&row(t(Pg::Budget), &mb(report.policy.budget)));
    html.push_str(&row(t(Pg::Pressure), t(pressure)));
    html.push_str(&format!("</div><h2>{}</h2><div class=box>", t(Pg::Policy)));
    html.push_str(&row(
        t(Pg::FreezeAfter),
        &seconds(report.policy.freeze_after.as_secs()),
    ));
    html.push_str(&row(
        t(Pg::DiscardAfter),
        &seconds(report.policy.discard_after.as_secs()),
    ));
    html.push_str(&row(
        t(Pg::Demotions),
        &fill(
            t(Pg::DemotionCount),
            &[
                &report.frozen_total.to_string(),
                &report.discarded_total.to_string(),
            ],
        ),
    ));
    html.push_str(&format!(
        "</div><h2>{}</h2><div class=box>",
        t(Pg::TabsTitle)
    ));
    for tab in &report.tabs {
        let state = tab.state.label();
        let marker = if tab.active { t(Pg::Showing) } else { "" };
        html.push_str(&format!(
            "<div class=row><span class=t>{}{marker}<br><span class=u>{} ・ {}</span></span>\
             <span class='st {state}'>{state}</span></div>",
            escape(&tab.title),
            escape(&tab.url),
            fill(t(Pg::SecondsAgo), &[&tab.idle_secs.to_string()])
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
            pressure: Pressure::None,
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
        let html = memory_html(&report(), Lang::Ja);
        assert!(html.contains("48.0 MB"), "合計");
        assert!(html.contains("200.0 MB"), "予算");
        assert!(html.contains("class='st frozen'>frozen"));
        assert!(html.contains("凍結 3 回 / 破棄 1 回"));
        assert!(html.contains("（表示中）"));
    }

    #[test]
    fn escapes_page_titles() {
        let html = memory_html(&report(), Lang::Ja);
        assert!(html.contains("&lt;b&gt;x&lt;/b&gt;"));
        assert!(!html.contains("<b>x</b>"));
    }

    #[test]
    fn labels_follow_the_chosen_language() {
        let html = memory_html(&report(), Lang::En);
        assert!(html.contains("Browser process") && html.contains("Frozen 3 / discarded 1"));
        assert!(html.contains("90 s ago") && html.contains("(showing)"));
        assert!(!html.contains("凍結"));
    }
}
