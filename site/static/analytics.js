// Google タグの初期化（インラインだと CSP で止まるので外部ファイルにしてある）。
// 測定 ID は <body> の data-ga から読む（hugo.toml の params.ga）。
window.dataLayer = window.dataLayer || [];
function gtag() { dataLayer.push(arguments); }
gtag('js', new Date());
gtag('config', document.currentScript.dataset.ga);
