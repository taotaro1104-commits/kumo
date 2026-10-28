//! ブラウザ UI のスタイルシート。

pub(crate) const CSS: &str = "
html, body { margin: 0; background: #e3e6ea; color: #1f2328;
             font-family: 'Segoe UI', 'Yu Gothic UI', system-ui, sans-serif; font-size: 13px; }
.tabs { display: flex; box-sizing: border-box; height: 38px; padding: 6px 8px 0; gap: 4px; align-items: flex-end; }
.tab { display: flex; align-items: center; flex: 0 1 220px; min-width: 72px; height: 32px;
       padding: 0 4px 0 12px; border-radius: 9px 9px 0 0; overflow: hidden; }
.tab.hover { background: #d4d8de; }
.tab.active { background: #ffffff; }
.tab a.title { flex: 1; color: #4a5058; text-decoration: none; white-space: nowrap; overflow: hidden; }
.tab.dormant a.title { color: #8a9099; }
.tab.active a.title { color: #1f2328; font-weight: 600; }
.dot { display: inline-block; width: 14px; color: #1a73e8; font-size: 10px; }
.tab a.close { flex: none; width: 20px; height: 20px; line-height: 20px; border-radius: 10px; text-align: center;
               color: #6b7280; text-decoration: none; font-size: 11px; }
.tab a.close.hover { background: #c5cad1; color: #1f2328; }
a.new { flex: none; width: 28px; height: 28px; line-height: 28px; margin-bottom: 2px; border-radius: 14px;
        text-align: center; color: #4a5058; text-decoration: none; font-size: 16px; }
a.new.hover { background: #d4d8de; }
.nav { display: flex; box-sizing: border-box; align-items: center; height: 38px; padding: 0 8px; gap: 4px;
       background: #ffffff; border-bottom: 1px solid #d5d9de; }
.nav.loading { border-bottom: 2px solid #1a73e8; }
.nav a.btn { flex: none; width: 30px; height: 30px; line-height: 30px; border-radius: 15px; text-align: center;
             color: #3c4043; text-decoration: none; font-size: 16px; }
.nav a.btn.hover { background: #eceef1; }
.nav a.btn.off { color: #c3c7cc; background: #ffffff; }
a.url { flex: 1; display: flex; box-sizing: border-box; align-items: center; height: 30px; padding: 0 14px 0 4px;
        background: #f1f3f4; border: 1px solid #f1f3f4; border-radius: 15px; text-decoration: none;
        white-space: nowrap; overflow: hidden; font-size: 14px; color: #1f2328; }
a.url.hover { background: #e8eaed; border-color: #e8eaed; }
a.star { flex: none; width: 30px; height: 30px; line-height: 30px; border-radius: 15px; text-align: center;
          color: #5f6368; text-decoration: none; font-size: 17px; }
a.star.on { color: #f9ab00; }
a.star.hover { background: #eceef1; }
.ask { flex: 1; overflow: hidden; white-space: nowrap; font-size: 13px; color: #1f2328; }
a.askbtn { flex: none; height: 28px; line-height: 28px; padding: 0 12px; border-radius: 14px; background: #f1f3f4;
           color: #1f2328; font-size: 12px; text-decoration: none; }
a.askbtn.main { background: #1a73e8; color: #ffffff; }
a.askbtn.hover { background: #e0e3e6; }
a.askbtn.main.hover { background: #1765cc; }
a.url.focused { background: #ffffff; border: 2px solid #1a73e8; padding-left: 12px; }
.chip { flex: none; height: 22px; line-height: 22px; padding: 0 8px; margin-right: 4px; border-radius: 11px;
        font-size: 12px; color: #4a5058; }
.chip.insecure { background: #fce8e6; color: #b3261e; }
.chip.file { background: #e8eaed; }
.host { color: #1f2328; }
.rest { color: #6b7280; }
.placeholder { color: #80868b; }
.caret { color: #1a73e8; }
.badge { flex: none; height: 24px; line-height: 24px; padding: 0 10px; border-radius: 12px; font-size: 12px; }
.badge.loading { background: #e8f0fe; color: #1a56c4; }
.badge.lost { background: #fce8e6; color: #b3261e; }
";
